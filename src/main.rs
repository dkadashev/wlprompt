// SPDX-License-Identifier:  GPL-3.0-or-later

fn main() {
    if let Err(e) = env_logger::try_init() {
        eprintln!("Failed to initialize logging, no logs will be available: {e}");
    }
    wlprompt::run();
}
