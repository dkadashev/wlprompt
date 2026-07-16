// SPDX-License-Identifier:  GPL-3.0-or-later
use log::trace;
use std::time::Duration;

use anyhow::Context as _;
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_compositor, delegate_keyboard, delegate_layer, delegate_output, delegate_seat,
    delegate_shm, output,
    reexports::{calloop, calloop_wayland_source},
    seat::{self, keyboard},
    shell::{
        WaylandSurface,
        wlr_layer::{
            Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
            LayerSurfaceConfigure,
        },
    },
    shm::{Shm, ShmHandler, slot::SlotPool},
};
use wayland_client::{
    Connection, Dispatch, QueueHandle,
    globals::{GlobalList, GlobalListContents, registry_queue_init},
    protocol::{wl_keyboard, wl_output, wl_registry, wl_seat, wl_shm, wl_surface},
};

pub struct Config {
    // ARGB8888 format, little-endian, so actually BGRA, see
    // * https://wayland.freedesktop.org/docs/html/apa.html#protocol-spec-wl_shm-enum-format,
    // * drm_fourcc.h in the Linux kernel tree
    // * https://www.kernel.org/doc/html/latest/userspace-api/media/v4l/pixfmt-rgb.html#bits-per-component
    // The last one might seem irrelevant, since it's about V4L, but note that it says that the
    // layout with AR24 code is BGRA.
    pub bg_color: [u8; 4],
    pub height: u32,
}

// LATER: replace with something appropriate
// For now this just shows a grey bar at the top of the screen
pub fn run_ui(cfg: &Config) -> anyhow::Result<()> {
    // Create a Wayland connection by connecting to the server through the
    // environment-provided configuration. See https://wayland-book.com/wayland-display.html
    let conn = Connection::connect_to_env().context("failed to establish wayland connection")?;

    // Make an event queue and retrieve the list of Wayland global objects (see
    // https://wayland-book.com/registry.html)
    let (globals, event_queue) =
        registry_queue_init::<GuiState>(&conn).context("failed to init wayland event queue")?;
    let qh = event_queue.handle();

    // This will be processing all wayland events and sending them to us to handle
    let mut event_loop: calloop::EventLoop<GuiState> =
        calloop::EventLoop::try_new().context("failed to initialize an event loop")?;
    // WaylandSource is an adaptor for wayland_client::EventQueue to act as a calloop's event source
    calloop_wayland_source::WaylandSource::new(conn, event_queue)
        .insert(event_loop.handle())
        .context("failed to insert wayland event source into the event loop")?;

    let mut state = GuiState::new(cfg, &globals, &qh)?;

    println!("Press Esc to exit");
    while !state.done {
        event_loop
            .dispatch(Duration::from_secs(1), &mut state)
            .context("event loop dispatch failed")?;
    }
    Ok(())
}

struct SurfaceProperties {
    width: u32,
    height: u32,
    scale_factor: u32,
    bg_color: [u8; 4],
}

impl Default for SurfaceProperties {
    fn default() -> SurfaceProperties {
        SurfaceProperties {
            width: 512,
            height: 25,
            scale_factor: 1,
            bg_color: [0, 0, 0, 0],
        }
    }
}

struct KeyboardState {
    keyboard: Option<wl_keyboard::WlKeyboard>,
    modifiers: keyboard::Modifiers,
}

struct GuiState {
    seat_state: seat::SeatState,
    output_state: output::OutputState,
    shm: Shm,
    pool: SlotPool,

    surface_properties: SurfaceProperties,
    surface: LayerSurface,
    kbd_state: KeyboardState,

    editor: super::wayline::Editor,

    done: bool,
}

impl GuiState {
    pub fn new(
        cfg: &Config,
        globals: &GlobalList,
        qh: &QueueHandle<GuiState>,
    ) -> anyhow::Result<GuiState> {
        let shm = Shm::bind(globals, qh).context("wl_shm is not available")?;
        // The initial pool size is 1MB, should be enough even for 4K displays, but if it's not then
        // the pool will be automatically resized
        let pool = SlotPool::new(1024 * 1024, &shm).context("failed to create a mem pool")?;

        Ok(GuiState {
            seat_state: seat::SeatState::new(globals, qh),
            output_state: output::OutputState::new(globals, qh),
            shm,
            pool,

            surface_properties: SurfaceProperties {
                height: cfg.height,
                bg_color: cfg.bg_color,
                ..Default::default()
            },
            surface: prepare_surface(globals, qh)?,
            kbd_state: KeyboardState {
                keyboard: None,
                modifiers: keyboard::Modifiers::default(),
            },
            editor: super::wayline::Editor::new(),

            done: false,
        })
    }

    pub fn draw(&mut self, _qh: &QueueHandle<Self>) {
        trace!("GuiState::draw()");
        let width = self.surface_properties.width * self.surface_properties.scale_factor;
        let height = self.surface_properties.height * self.surface_properties.scale_factor;

        let (buffer, canvas) = self
            .pool
            .create_buffer(
                width.cast_signed(),
                height.cast_signed(),
                width.cast_signed() * 4, // stride, 4 bytes per pixel, hence multiply by 4
                wl_shm::Format::Argb8888,
            )
            .expect("failed to get a buffer from the pool");

        for px in canvas.chunks_exact_mut(4) {
            px.copy_from_slice(&self.surface_properties.bg_color);
        }

        self.surface
            .wl_surface()
            .damage_buffer(0, 0, width.cast_signed(), height.cast_signed());

        buffer
            .attach_to(self.surface.wl_surface())
            .expect("failed to attach buffer to the surface");
        self.surface.commit();
    }
}

// Implement `Dispatch<WlRegistry, GlobalListContents> for our state. Necessary for
// being able to use registry_queue_init()
impl Dispatch<wl_registry::WlRegistry, GlobalListContents> for GuiState {
    fn event(
        _state: &mut Self,
        _proxy: &wl_registry::WlRegistry,
        event: <wl_registry::WlRegistry as wayland_client::Proxy>::Event,
        _data: &GlobalListContents,
        _conn: &Connection,
        _qhandle: &QueueHandle<Self>,
    ) {
        trace!(event:?; "WlRegistry event");
    }
}

// Necessary for delegate_compositor
impl CompositorHandler for GuiState {
    fn scale_factor_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        new_factor: i32,
    ) {
        trace!(new_factor; "CompositorHandler::scale_factor_changed");
        self.surface_properties.scale_factor = new_factor.cast_unsigned();
        // The surface is double buffered, the following call affects the pending buffer,
        // i.e. the next one that will be drawn with draw(), not the currently active one
        self.surface
            .set_buffer_scale(self.surface_properties.scale_factor)
            .expect("failed to set buffer scale");
    }

    fn frame(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        surface: &wl_surface::WlSurface,
        time: u32,
    ) {
        trace!(surface:?, time; "CompositorHandler::frame");
        // We do not request frame callbacks (since the only reason to redraw the screen for us is
        // if the rendered text is changed, which can only happen when user pressed some button), so
        // we do not do anything even if we received one (but this should not happen)
    }

    fn transform_changed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        _surface: &wl_surface::WlSurface,
        _new_transform: wl_output::Transform,
    ) {
    }

    fn surface_enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
        trace!(surface:?; "CompositorHandler::surface_enter");
    }

    fn surface_leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        surface: &wl_surface::WlSurface,
        _output: &wl_output::WlOutput,
    ) {
        trace!(surface:?; "CompositorHandler::surface_leave");
    }
}

impl output::OutputHandler for GuiState {
    fn output_state(&mut self) -> &mut output::OutputState {
        &mut self.output_state
    }

    fn new_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        trace!(output:?; "OutputHandler::new_output");
    }

    fn update_output(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        trace!(output:?; "OutputHandler::update_output");
    }

    fn output_destroyed(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        output: wl_output::WlOutput,
    ) {
        trace!(output:?; "OutputHandler::output_destroyed");
    }
}
delegate_compositor!(GuiState);

// Necessary for delegate_shm
impl ShmHandler for GuiState {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}
delegate_shm!(GuiState);

// Necessary for delegate_layer
impl LayerShellHandler for GuiState {
    fn closed(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, _layer: &LayerSurface) {
        trace!("LayerShellHandler::closed");
        self.done = true;
    }

    fn configure(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        _layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _serial: u32,
    ) {
        trace!(configure:?; "LayerShellHandler::configure");
        if configure.new_size.0 > 0 {
            self.surface_properties.width = configure.new_size.0;
        }
        // we ignore the height hint, our height is fixed

        // Re-draw the shell surface, useful in two cases: initial configure / draw and when
        // the surface size has changed. The latter should not really be happening, but hopefully we
        // won't get any/too many `configure` events in that case, so we just re-draw
        // unconditionally.
        self.draw(qh);
    }
}
delegate_layer!(GuiState);

delegate_output!(GuiState);

impl seat::SeatHandler for GuiState {
    fn seat_state(&mut self) -> &mut seat::SeatState {
        &mut self.seat_state
    }

    fn new_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, seat: wl_seat::WlSeat) {
        trace!(seat:?; "SeatHandler::new_seat");
    }

    fn new_capability(
        &mut self,
        _conn: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: seat::Capability,
    ) {
        trace!(seat:?, capability:?; "SeatHandler::new_capability");
        if capability == seat::Capability::Keyboard && self.kbd_state.keyboard.is_none() {
            // LATER: use get_keyboard_with_repeat()
            // The code below uses 'expect()' rather than e.g. logging a warning, because it's not
            // supposed to happen and if it DOES happen then we might end up in a situation when
            // the app/surface can't be closed normally (because it does not get keyboard input), in
            // this situation it's preferable to blow up right away.
            let keyboard = self
                .seat_state
                .get_keyboard(qh, &seat, None)
                .expect("failed to create a keyboard handler");
            self.kbd_state.keyboard = Some(keyboard);
        }
    }

    fn remove_capability(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: seat::Capability,
    ) {
        trace!(seat:?, capability:?; "SeatHandler::remove_capability");
        if capability == seat::Capability::Keyboard
            && let Some(kbd) = self.kbd_state.keyboard.take()
        {
            kbd.release();
        }
    }

    fn remove_seat(&mut self, _conn: &Connection, _qh: &QueueHandle<Self>, seat: wl_seat::WlSeat) {
        trace!(seat:?; "SeatHandler::remove_seat");
    }
}

delegate_seat!(GuiState);

impl keyboard::KeyboardHandler for GuiState {
    fn enter(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        kbd: &wl_keyboard::WlKeyboard,
        _surface: &wl_surface::WlSurface,
        _serial: u32,
        _raw: &[u32],
        keysyms: &[keyboard::Keysym],
    ) {
        trace!(kbd:?, keysyms:?; "KeyboardHandler::enter");
    }

    fn leave(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        kbd: &wl_keyboard::WlKeyboard,
        _surface: &wl_surface::WlSurface,
        _serial: u32,
    ) {
        trace!(kbd:?; "KeyboardHandler::leave");
    }

    fn press_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        kbd: &wl_keyboard::WlKeyboard,
        _serial: u32,
        event: keyboard::KeyEvent,
    ) {
        trace!(kbd:?, event:?; "KeyboardHandler::press_key");
        if event.keysym == keyboard::Keysym::Escape {
            self.done = true;
        }
        let kp = super::wayline::KeyPress {
            modifiers: self.kbd_state.modifiers,
            key_event: event,
        };
        let txt = self.editor.handle_key(&kp);
        trace!("current text: {txt}");
    }

    fn repeat_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        kbd: &wl_keyboard::WlKeyboard,
        _serial: u32,
        event: keyboard::KeyEvent,
    ) {
        trace!(kbd:?, event:?; "KeyboardHandler::repeat_key");
    }

    fn release_key(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        kbd: &wl_keyboard::WlKeyboard,
        _serial: u32,
        event: keyboard::KeyEvent,
    ) {
        trace!(kbd:?, event:?; "KeyboardHandler::release_key");
    }

    fn update_modifiers(
        &mut self,
        _conn: &Connection,
        _qh: &QueueHandle<Self>,
        kbd: &wl_keyboard::WlKeyboard,
        _serial: u32,
        modifiers: keyboard::Modifiers,
        _raw_modifiers: keyboard::RawModifiers,
        _layout: u32,
    ) {
        trace!(kbd:?, modifiers:?; "KeyboardHandler::update_modifiers");
        self.kbd_state.modifiers = modifiers;
    }
}

delegate_keyboard!(GuiState);

fn prepare_surface(
    globals: &GlobalList,
    qh: &QueueHandle<GuiState>,
) -> anyhow::Result<LayerSurface> {
    // Bind wl_compositor, the thing that will hand us a surface to use.
    // See https://wayland-book.com/surfaces/compositor.html
    let compositor =
        CompositorState::bind(globals, qh).context("wl_compositor is not available")?;
    // Bind zwlr_layer_shell (rather than xdg_shell), since our window is supposed to be on top of
    // everything and capturing all input, rather than a normal desktop window. See
    // https://docs.rs/smithay-client-toolkit/latest/smithay_client_toolkit/shell/index.html
    let layer_shell = LayerShell::bind(globals, qh).context("zwlr_layer_shell is not available")?;

    let surface = layer_shell.create_layer_surface(
        qh,
        compositor.create_surface(qh),
        Layer::Top,
        Some("wlprompt"),
        None,
    );

    surface.set_anchor(Anchor::LEFT | Anchor::TOP | Anchor::RIGHT);
    // Grab all keyboard input while we are active, which fits the nature of the app
    surface.set_keyboard_interactivity(KeyboardInteractivity::Exclusive);
    // See https://wayland.app/protocols/wlr-layer-shell-unstable-v1#zwlr_layer_surface_v1:request:set_exclusive_zone
    surface.set_exclusive_zone(-1);
    surface.commit();

    Ok(surface)
}
