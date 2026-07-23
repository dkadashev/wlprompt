// SPDX-License-Identifier:  GPL-3.0-or-later
//! Readline-like utilities not tied to TTYs.

// LATER: it would be nice to make this a standalone library that provides readline-like behavior
// for wayland or other environments

#[derive(Default, Copy, Clone)]
pub struct Modifiers {
    pub ctrl: bool,
    pub alt: bool,
    #[allow(dead_code)]
    pub shift: bool,
}

pub enum KeyCode {
    Char(char),
}

pub struct KeyPress {
    pub key: KeyCode,
    pub modifiers: Modifiers,
}

pub struct Editor {
    text: String, // LATER: this is probably not a great idea for perf reasons, but for now it'll do
}

// LATER: add docs
// LATER: add backspace support
// LATER: add navigation (cursor movement) support
// LATER: add history support
// LATER: add tab completion
impl Editor {
    pub fn new() -> Editor {
        Editor {
            text: String::new(),
        }
    }

    // For now this is very crude, just builds the string, no completion, no history, no hotkeys /
    // navigation / editing.
    pub fn handle_key(&mut self, key: &KeyPress) -> &str {
        match keypress_to_action(key) {
            Action::AppendChar(ch) => self.text.push(ch),
            Action::NoOp => {}
        }
        self.text.as_ref()
    }

    pub fn current_text(&self) -> &str {
        self.text.as_ref()
    }
}

enum Action {
    NoOp,
    AppendChar(char),
}

fn keypress_to_action(kp: &KeyPress) -> Action {
    if kp.modifiers.ctrl || kp.modifiers.alt {
        return Action::NoOp;
    }
    match kp.key {
        KeyCode::Char(ch) => {
            // Some keys (Enter, Tab, Escape, Backspace, ...) deliver a value that is a control
            // character. We must not insert those into the buffer; they will get dedicated handling
            // later.
            if ch.is_control() {
                Action::NoOp
            } else {
                Action::AppendChar(ch)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! key {
        ($ch:expr) => { key!($ch; ) };
        ($ch:expr; $( $mod:ident ),*) => {
            KeyPress {
                modifiers: Modifiers{
                    $($mod: true,)*
                    ..Modifiers::default()
                },
                key: KeyCode::Char($ch),
            }
        };
    }

    #[test]
    fn test_basic_text_appended() {
        let mut e = Editor::new();
        assert_eq!(e.handle_key(&key!('h')), "h");
        assert_eq!(e.handle_key(&key!('i')), "hi");
    }

    #[test]
    fn test_unknown_combinations_ignored() {
        let mut e = Editor::new();
        assert_eq!(e.handle_key(&key!('a')), "a");
        assert_eq!(e.handle_key(&key!('b')), "ab");
        assert_eq!(e.handle_key(&key!('c'; ctrl)), "ab");
        assert_eq!(e.handle_key(&key!('c'; alt)), "ab");
    }

    #[test]
    fn test_shift_not_filtered() {
        let mut e = Editor::new();
        assert_eq!(e.handle_key(&key!('a')), "a");
        assert_eq!(e.handle_key(&key!('B'; shift)), "aB");
    }

    #[test]
    fn test_multiple_modifiers_ignored() {
        let mut e = Editor::new();
        assert_eq!(e.handle_key(&key!('a')), "a");
        assert_eq!(e.handle_key(&key!('x'; ctrl, alt)), "a");
    }

    #[test]
    fn test_special_characters() {
        let mut e = Editor::new();
        assert_eq!(e.handle_key(&key!(' ')), " ");
        assert_eq!(e.handle_key(&key!('!')), " !");
        assert_eq!(e.handle_key(&key!('ñ')), " !ñ");
        assert_eq!(e.handle_key(&key!('日')), " !ñ日");
    }

    #[test]
    fn test_new_editor_is_empty() {
        let e = Editor::new();
        assert_eq!(e.current_text(), "");
    }

    #[test]
    fn test_control_characters_ignored() {
        let mut e = Editor::new();
        assert_eq!(e.handle_key(&key!('a')), "a");
        // Keys like Enter, Tab, Escape and Backspace deliver a control character via utf8;
        // none of them must end up in the buffer.
        assert_eq!(e.handle_key(&key!('\r')), "a"); // Enter / Return
        assert_eq!(e.handle_key(&key!('\n')), "a"); // Line feed
        assert_eq!(e.handle_key(&key!('\t')), "a"); // Tab
        assert_eq!(e.handle_key(&key!('\u{1b}')), "a"); // Escape
        assert_eq!(e.handle_key(&key!('\u{8}')), "a"); // Backspace
        assert_eq!(e.handle_key(&key!('\u{7f}')), "a"); // Delete
        // Printable input still works afterwards.
        assert_eq!(e.handle_key(&key!('b')), "ab");
    }
}
