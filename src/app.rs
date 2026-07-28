// SPDX-License-Identifier:  GPL-3.0-or-later
//! The main business logic of the application: read the input and execute the command when user
//! finished typing.

use std::env;
use std::process;

mod readline;

pub use readline::{KeyCode, KeyPress, Modifiers};

pub enum HandleKeyResult {
    MoreInputNeeded,
    Exit,
}

pub struct App {
    editor: readline::Editor,
}

// Shell to use if expanding SHELL env var fails
const FALLBACK_SHELL: &str = "sh";

impl App {
    pub fn new() -> Self {
        App {
            editor: readline::Editor::new(),
        }
    }

    pub fn handle_key(&mut self, key: &KeyPress) -> HandleKeyResult {
        match self.editor.handle_key(key) {
            readline::HandleKeyResult::MoreInputNeeded => HandleKeyResult::MoreInputNeeded,
            readline::HandleKeyResult::Cancel => HandleKeyResult::Exit,
            readline::HandleKeyResult::Done => {
                let shell = env::var("SHELL").unwrap_or(FALLBACK_SHELL.to_string());
                let _spawn_result = process::Command::new(shell)
                    .arg("-c")
                    .arg(self.editor.current_text())
                    .spawn();
                // LATER: check _spawn_result for whether the shell at least has been started OK,
                // and report to the upper layer if it failed.

                // Exit the launcher once the target app is started
                HandleKeyResult::Exit
            }
        }
    }

    pub fn current_text(&self) -> &str {
        self.editor.current_text()
    }
}
