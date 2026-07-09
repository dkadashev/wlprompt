// SPDX-License-Identifier:  GPL-3.0-or-later
mod gui;

pub fn run() {
    let ui_config = gui::wayland::Config {
        bg_color: [0x20, 0x20, 0x20, 0xff],
        height: 25,
    };
    gui::wayland::run_ui(ui_config);
}
