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
    Backspace,
    Escape,
}

pub struct KeyPress {
    pub key: KeyCode,
    pub modifiers: Modifiers,
}

// The result of handling a key press. For now the only outcome is that the editor needs more
// input, but later this will grow variants for e.g. "input finished" or "cancelled".
#[derive(Debug, PartialEq)]
pub enum HandleKeyResult {
    MoreInputNeeded,
    Cancel,
}

pub struct Editor {
    text: String, // LATER: this is probably not a great idea for perf reasons, but for now it'll do
}

// LATER: add docs
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
            Action::Backspace => {
                self.text.pop();
            }
            Action::Cancel => {
                return HandleKeyResult::Cancel;
            }
            Action::NoOp => {}
        }
        HandleKeyResult::MoreInputNeeded
    }

    pub fn current_text(&self) -> &str {
        self.text.as_ref()
    }
}

enum Action {
    NoOp,
    AppendChar(char),
    Backspace,
    Cancel,
}

fn keypress_to_action(kp: &KeyPress) -> Action {
    match kp.key {
        // The dedicated Backspace key always means backspace, regardless of modifiers.
        KeyCode::Backspace => Action::Backspace,
        KeyCode::Escape => Action::Cancel,
        KeyCode::Char(ch) => {
            // Ctrl+H is the traditional terminal binding for backspace, so treat it exactly like
            // the dedicated Backspace key. This must be checked before the generic ctrl/alt
            // filtering below, which would otherwise discard the combination as an unknown
            // shortcut.
            if kp.modifiers.ctrl && !kp.modifiers.alt && ch == 'h' {
                return Action::Backspace;
            }
            // Backspace
            if ch == '\u{8}' {
                return Action::Backspace;
            }
            // Ctrl-C
            if ch == '\u{3}' {
                return Action::Cancel;
            }
            // Escape
            if ch == '\u{1b}' {
                return Action::Cancel;
            }
            if kp.modifiers.ctrl || kp.modifiers.alt {
                return Action::NoOp;
            }
            // Some keys (Enter, Tab, Escape, Delete, ...) deliver a value that is a control
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

    /// Build a `KeyPress` carrying the dedicated `Backspace` key code, optionally with modifiers.
    macro_rules! backspace {
        ($( $mod:ident ),*) => {
            KeyPress {
                modifiers: Modifiers{
                    $($mod: true,)*
                    ..Modifiers::default()
                },
                key: KeyCode::Backspace,
            }
        };
    }

    /// Build a `KeyPress` carrying the dedicated `Escape` key code, optionally with modifiers.
    macro_rules! escape {
        ($( $mod:ident ),*) => {
            KeyPress {
                modifiers: Modifiers{
                    $($mod: true,)*
                    ..Modifiers::default()
                },
                key: KeyCode::Escape,
            }
        };
    }

    #[test]
    fn test_basic_text_appended() {
        let mut e = Editor::new();
        assert_eq!(e.handle_key(&key!('h')), HandleKeyResult::MoreInputNeeded);
        assert_eq!(e.current_text(), "h");
        assert_eq!(e.handle_key(&key!('i')), HandleKeyResult::MoreInputNeeded);
        assert_eq!(e.current_text(), "hi");
    }

    #[test]
    fn test_unknown_combinations_ignored() {
        let mut e = Editor::new();
        assert_eq!(e.handle_key(&key!('a')), HandleKeyResult::MoreInputNeeded);
        assert_eq!(e.current_text(), "a");
        assert_eq!(e.handle_key(&key!('b')), HandleKeyResult::MoreInputNeeded);
        assert_eq!(e.current_text(), "ab");
        assert_eq!(
            e.handle_key(&key!('c'; ctrl)),
            HandleKeyResult::MoreInputNeeded
        );
        assert_eq!(e.current_text(), "ab");
        assert_eq!(
            e.handle_key(&key!('c'; alt)),
            HandleKeyResult::MoreInputNeeded
        );
        assert_eq!(e.current_text(), "ab");
    }

    #[test]
    fn test_shift_not_filtered() {
        let mut e = Editor::new();
        assert_eq!(e.handle_key(&key!('a')), HandleKeyResult::MoreInputNeeded);
        assert_eq!(e.current_text(), "a");
        assert_eq!(
            e.handle_key(&key!('B'; shift)),
            HandleKeyResult::MoreInputNeeded
        );
        assert_eq!(e.current_text(), "aB");
    }

    #[test]
    fn test_multiple_modifiers_ignored() {
        let mut e = Editor::new();
        assert_eq!(e.handle_key(&key!('a')), HandleKeyResult::MoreInputNeeded);
        assert_eq!(e.current_text(), "a");
        assert_eq!(
            e.handle_key(&key!('x'; ctrl, alt)),
            HandleKeyResult::MoreInputNeeded
        );
        assert_eq!(e.current_text(), "a");
    }

    #[test]
    fn test_special_characters() {
        let mut e = Editor::new();
        assert_eq!(e.handle_key(&key!(' ')), HandleKeyResult::MoreInputNeeded);
        assert_eq!(e.current_text(), " ");
        assert_eq!(e.handle_key(&key!('!')), HandleKeyResult::MoreInputNeeded);
        assert_eq!(e.current_text(), " !");
        assert_eq!(e.handle_key(&key!('ñ')), HandleKeyResult::MoreInputNeeded);
        assert_eq!(e.current_text(), " !ñ");
        assert_eq!(e.handle_key(&key!('日')), HandleKeyResult::MoreInputNeeded);
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
        assert_eq!(e.handle_key(&key!('a')), HandleKeyResult::MoreInputNeeded);
        assert_eq!(e.current_text(), "a");
        // Keys like Enter, Tab and Delete deliver a control character via utf8; none of them must
        // end up in the buffer.
        assert_eq!(e.handle_key(&key!('\r')), HandleKeyResult::MoreInputNeeded); // Enter / Return
        assert_eq!(e.handle_key(&key!('\n')), HandleKeyResult::MoreInputNeeded); // Line feed
        assert_eq!(e.handle_key(&key!('\t')), HandleKeyResult::MoreInputNeeded); // Tab
        assert_eq!(
            e.handle_key(&key!('\u{7f}')),
            HandleKeyResult::MoreInputNeeded
        ); // Delete
        assert_eq!(e.current_text(), "a");
        // Printable input still works afterwards.
        assert_eq!(e.handle_key(&key!('b')), HandleKeyResult::MoreInputNeeded);
        assert_eq!(e.current_text(), "ab");
    }

    #[test]
    fn test_bs_control_char_acts_as_backspace() {
        // The BS control character (`\u{8}`) delivered as a `Char` must delete the last character,
        // just like the dedicated Backspace key.
        let mut e = Editor::new();
        assert_eq!(e.handle_key(&key!('a')), HandleKeyResult::MoreInputNeeded);
        assert_eq!(e.handle_key(&key!('b')), HandleKeyResult::MoreInputNeeded);
        assert_eq!(e.current_text(), "ab");
        assert_eq!(
            e.handle_key(&key!('\u{8}')),
            HandleKeyResult::MoreInputNeeded
        );
        assert_eq!(e.current_text(), "a");
        assert_eq!(
            e.handle_key(&key!('\u{8}')),
            HandleKeyResult::MoreInputNeeded
        );
        assert_eq!(e.current_text(), "");
    }

    #[test]
    fn test_bs_control_char_on_empty_is_noop() {
        // Applying the BS control character to an empty buffer must not panic and leaves it empty.
        let mut e = Editor::new();
        assert_eq!(
            e.handle_key(&key!('\u{8}')),
            HandleKeyResult::MoreInputNeeded
        );
        assert_eq!(e.current_text(), "");
    }

    #[test]
    fn test_escape_key_cancels() {
        // The dedicated Escape key requests cancellation.
        let mut e = Editor::new();
        assert_eq!(e.handle_key(&escape!()), HandleKeyResult::Cancel);
    }

    #[test]
    fn test_escape_key_cancels_with_text() {
        // Escape cancels regardless of what has already been typed, and it must not mutate the
        // buffer.
        let mut e = Editor::new();
        assert_eq!(e.handle_key(&key!('a')), HandleKeyResult::MoreInputNeeded);
        assert_eq!(e.handle_key(&key!('b')), HandleKeyResult::MoreInputNeeded);
        assert_eq!(e.current_text(), "ab");
        assert_eq!(e.handle_key(&escape!()), HandleKeyResult::Cancel);
        assert_eq!(e.current_text(), "ab");
    }

    #[test]
    fn test_escape_key_cancels_with_modifiers() {
        // The dedicated Escape key cancels regardless of modifiers held alongside it.
        let mut e = Editor::new();
        assert_eq!(e.handle_key(&escape!(ctrl)), HandleKeyResult::Cancel);
        assert_eq!(e.handle_key(&escape!(alt)), HandleKeyResult::Cancel);
        assert_eq!(e.handle_key(&escape!(shift)), HandleKeyResult::Cancel);
        assert_eq!(e.handle_key(&escape!(ctrl, alt)), HandleKeyResult::Cancel);
    }

    #[test]
    fn test_escape_control_char_cancels() {
        // The Escape control character (`\u{1b}`) delivered as a `Char` must request cancellation,
        // just like the dedicated Escape key.
        let mut e = Editor::new();
        assert_eq!(e.handle_key(&key!('a')), HandleKeyResult::MoreInputNeeded);
        assert_eq!(e.current_text(), "a");
        assert_eq!(e.handle_key(&key!('\u{1b}')), HandleKeyResult::Cancel);
        // The buffer is left untouched by the cancel request.
        assert_eq!(e.current_text(), "a");
    }

    #[test]
    fn test_ctrl_c_control_char_cancels() {
        // Ctrl-C is delivered as the ETX control character (`\u{3}`) and must request
        // cancellation.
        let mut e = Editor::new();
        assert_eq!(e.handle_key(&key!('a')), HandleKeyResult::MoreInputNeeded);
        assert_eq!(e.current_text(), "a");
        assert_eq!(e.handle_key(&key!('\u{3}')), HandleKeyResult::Cancel);
        // The buffer is left untouched by the cancel request.
        assert_eq!(e.current_text(), "a");
    }

    #[test]
    fn test_ctrl_c_control_char_cancels_on_empty() {
        // Cancelling an empty buffer must work and must not panic.
        let mut e = Editor::new();
        assert_eq!(e.handle_key(&key!('\u{3}')), HandleKeyResult::Cancel);
        assert_eq!(e.current_text(), "");
    }

    #[test]
    fn test_backspace_key_removes_last_char() {
        let mut e = Editor::new();
        assert_eq!(e.handle_key(&key!('a')), HandleKeyResult::MoreInputNeeded);
        assert_eq!(e.handle_key(&key!('b')), HandleKeyResult::MoreInputNeeded);
        assert_eq!(e.current_text(), "ab");
        assert_eq!(
            e.handle_key(&backspace!()),
            HandleKeyResult::MoreInputNeeded
        );
        assert_eq!(e.current_text(), "a");
        assert_eq!(
            e.handle_key(&backspace!()),
            HandleKeyResult::MoreInputNeeded
        );
        assert_eq!(e.current_text(), "");
    }

    #[test]
    fn test_backspace_on_empty_is_noop() {
        // Backspacing an empty buffer must not panic and must leave it empty.
        let mut e = Editor::new();
        assert_eq!(
            e.handle_key(&backspace!()),
            HandleKeyResult::MoreInputNeeded
        );
        assert_eq!(e.current_text(), "");
    }

    #[test]
    fn test_backspace_removes_multibyte_char() {
        // `String::pop()` removes a whole char, so multi-byte scalars are removed in one press.
        let mut e = Editor::new();
        assert_eq!(e.handle_key(&key!('a')), HandleKeyResult::MoreInputNeeded);
        assert_eq!(e.handle_key(&key!('日')), HandleKeyResult::MoreInputNeeded);
        assert_eq!(e.current_text(), "a日");
        assert_eq!(
            e.handle_key(&backspace!()),
            HandleKeyResult::MoreInputNeeded
        );
        assert_eq!(e.current_text(), "a");
    }

    #[test]
    fn test_ctrl_h_acts_as_backspace() {
        // Ctrl+H is the traditional terminal binding for backspace.
        let mut e = Editor::new();
        assert_eq!(e.handle_key(&key!('a')), HandleKeyResult::MoreInputNeeded);
        assert_eq!(e.handle_key(&key!('b')), HandleKeyResult::MoreInputNeeded);
        assert_eq!(e.current_text(), "ab");
        assert_eq!(
            e.handle_key(&key!('h'; ctrl)),
            HandleKeyResult::MoreInputNeeded
        );
        assert_eq!(e.current_text(), "a");
    }

    #[test]
    fn test_plain_h_is_inserted() {
        // Without ctrl, 'h' is an ordinary character and must be inserted, not treated as backspace.
        let mut e = Editor::new();
        assert_eq!(e.handle_key(&key!('a')), HandleKeyResult::MoreInputNeeded);
        assert_eq!(e.handle_key(&key!('h')), HandleKeyResult::MoreInputNeeded);
        assert_eq!(e.current_text(), "ah");
    }

    #[test]
    fn test_ctrl_alt_h_is_not_backspace() {
        // Ctrl+Alt+H is a different combination and must not delete anything.
        let mut e = Editor::new();
        assert_eq!(e.handle_key(&key!('a')), HandleKeyResult::MoreInputNeeded);
        assert_eq!(
            e.handle_key(&key!('h'; ctrl, alt)),
            HandleKeyResult::MoreInputNeeded
        );
        assert_eq!(e.current_text(), "a");
    }

    #[test]
    fn test_backspace_with_modifiers_still_deletes() {
        // The dedicated Backspace key deletes regardless of modifiers held alongside it.
        let mut e = Editor::new();
        assert_eq!(e.handle_key(&key!('a')), HandleKeyResult::MoreInputNeeded);
        assert_eq!(e.handle_key(&key!('b')), HandleKeyResult::MoreInputNeeded);
        assert_eq!(e.current_text(), "ab");
        assert_eq!(
            e.handle_key(&backspace!(shift)),
            HandleKeyResult::MoreInputNeeded
        );
        assert_eq!(e.current_text(), "a");
    }

    #[test]
    fn test_append_backspace_append() {
        // Test that append works correctly after backspace
        let mut e = Editor::new();
        assert_eq!(e.handle_key(&key!('a')), HandleKeyResult::MoreInputNeeded);
        assert_eq!(e.handle_key(&key!('b')), HandleKeyResult::MoreInputNeeded);
        assert_eq!(e.handle_key(&key!('d')), HandleKeyResult::MoreInputNeeded);
        assert_eq!(e.current_text(), "abd");
        assert_eq!(
            e.handle_key(&backspace!()),
            HandleKeyResult::MoreInputNeeded
        );
        assert_eq!(e.current_text(), "ab");
        assert_eq!(e.handle_key(&key!('c')), HandleKeyResult::MoreInputNeeded);
        assert_eq!(e.current_text(), "abc");
    }
}
