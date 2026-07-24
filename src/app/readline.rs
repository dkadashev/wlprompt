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

// The result of handling a key press. For now the only outcome is that the editor needs more
// input, but later this will grow variants for e.g. "input finished" or "cancelled".
#[derive(Debug, PartialEq)]
pub enum HandleKeyResult {
    MoreDataNeeded,
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
    #[must_use]
    pub fn handle_key(&mut self, key: &KeyPress) -> HandleKeyResult {
        match keypress_to_action(key) {
            Action::AppendChar(ch) => self.text.push(ch),
            Action::NoOp => {}
        }
        HandleKeyResult::MoreDataNeeded
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
        assert_eq!(e.handle_key(&key!('h')), HandleKeyResult::MoreDataNeeded);
        assert_eq!(e.current_text(), "h");
        assert_eq!(e.handle_key(&key!('i')), HandleKeyResult::MoreDataNeeded);
        assert_eq!(e.current_text(), "hi");
    }

    #[test]
    fn test_unknown_combinations_ignored() {
        let mut e = Editor::new();
        assert_eq!(e.handle_key(&key!('a')), HandleKeyResult::MoreDataNeeded);
        assert_eq!(e.current_text(), "a");
        assert_eq!(e.handle_key(&key!('b')), HandleKeyResult::MoreDataNeeded);
        assert_eq!(e.current_text(), "ab");
        assert_eq!(
            e.handle_key(&key!('c'; ctrl)),
            HandleKeyResult::MoreDataNeeded
        );
        assert_eq!(e.current_text(), "ab");
        assert_eq!(
            e.handle_key(&key!('c'; alt)),
            HandleKeyResult::MoreDataNeeded
        );
        assert_eq!(e.current_text(), "ab");
    }

    #[test]
    fn test_shift_not_filtered() {
        let mut e = Editor::new();
        assert_eq!(e.handle_key(&key!('a')), HandleKeyResult::MoreDataNeeded);
        assert_eq!(e.current_text(), "a");
        assert_eq!(
            e.handle_key(&key!('B'; shift)),
            HandleKeyResult::MoreDataNeeded
        );
        assert_eq!(e.current_text(), "aB");
    }

    #[test]
    fn test_multiple_modifiers_ignored() {
        let mut e = Editor::new();
        assert_eq!(e.handle_key(&key!('a')), HandleKeyResult::MoreDataNeeded);
        assert_eq!(e.current_text(), "a");
        assert_eq!(
            e.handle_key(&key!('x'; ctrl, alt)),
            HandleKeyResult::MoreDataNeeded
        );
        assert_eq!(e.current_text(), "a");
    }

    #[test]
    fn test_special_characters() {
        let mut e = Editor::new();
        assert_eq!(e.handle_key(&key!(' ')), HandleKeyResult::MoreDataNeeded);
        assert_eq!(e.current_text(), " ");
        assert_eq!(e.handle_key(&key!('!')), HandleKeyResult::MoreDataNeeded);
        assert_eq!(e.current_text(), " !");
        assert_eq!(e.handle_key(&key!('ñ')), HandleKeyResult::MoreDataNeeded);
        assert_eq!(e.current_text(), " !ñ");
        assert_eq!(e.handle_key(&key!('日')), HandleKeyResult::MoreDataNeeded);
        assert_eq!(e.current_text(), " !ñ日");
    }

    #[test]
    fn test_new_editor_is_empty() {
        let e = Editor::new();
        assert_eq!(e.current_text(), "");
    }

    #[test]
    fn test_control_characters_ignored() {
        let mut e = Editor::new();
        assert_eq!(e.handle_key(&key!('a')), HandleKeyResult::MoreDataNeeded);
        assert_eq!(e.current_text(), "a");
        // Keys like Enter, Tab, Escape and Backspace deliver a control character via utf8;
        // none of them must end up in the buffer.
        assert_eq!(e.handle_key(&key!('\r')), HandleKeyResult::MoreDataNeeded); // Enter / Return
        assert_eq!(e.handle_key(&key!('\n')), HandleKeyResult::MoreDataNeeded); // Line feed
        assert_eq!(e.handle_key(&key!('\t')), HandleKeyResult::MoreDataNeeded); // Tab
        assert_eq!(
            e.handle_key(&key!('\u{1b}')),
            HandleKeyResult::MoreDataNeeded
        ); // Escape
        assert_eq!(
            e.handle_key(&key!('\u{8}')),
            HandleKeyResult::MoreDataNeeded
        ); // Backspace
        assert_eq!(
            e.handle_key(&key!('\u{7f}')),
            HandleKeyResult::MoreDataNeeded
        ); // Delete
        assert_eq!(e.current_text(), "a");
        // Printable input still works afterwards.
        assert_eq!(e.handle_key(&key!('b')), HandleKeyResult::MoreDataNeeded);
        assert_eq!(e.current_text(), "ab");
    }
}
