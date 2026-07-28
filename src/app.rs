// SPDX-License-Identifier:  GPL-3.0-or-later
//! The main business logic of the application: read the input and execute the command when user
//! finished typing.
mod readline;

pub use readline::{KeyCode, KeyPress, Modifiers};

pub enum HandleKeyResult {
    MoreInputNeeded,
}

pub struct App {
    editor: readline::Editor,
}

impl App {
    pub fn new() -> Self {
        App {
            editor: readline::Editor::new(),
        }
    }

    pub fn handle_key(&mut self, key: &KeyPress) -> HandleKeyResult {
        match self.editor.handle_key(key) {
            readline::HandleKeyResult::MoreInputNeeded => HandleKeyResult::MoreInputNeeded,
        }
    }

    pub fn current_text(&self) -> &str {
        self.editor.current_text()
    }
}
