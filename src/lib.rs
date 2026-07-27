// SPDX-License-Identifier:  GPL-3.0-or-later
mod app;
mod gui;

/// Run the app, showing the UI and letting the user to type commands and execute them in a child
/// process.
///
/// # Panics
///
/// The code tries to avoid panics as much as practical, but in some cases where the failure is
/// really unlikely it does resort to panics. This is mostly about communicating with the display
/// server / compositor in places where bubbling the error up does not worth it.
///
/// # Errors
///
/// There are quite a few potential errors (communicating with the display server / compositor,
/// loading fonts, potentially accessing files, and so on), listing all of them is definitely not
/// practical.
pub fn run() -> anyhow::Result<()> {
    let ui_config = gui::wayland::Config {
        bg_color: gui::wayland::Color::from_rgba(0x20, 0x20, 0x20, 0xff),
        text_color: gui::wayland::Color::from_rgba(0xa0, 0xa0, 0xa0, 0xff),
        text_margin: 10,
        font: "monospace".to_string(),
        font_size: 12,
        height: 25,
    };
    gui::wayland::run_ui(&ui_config)
}
