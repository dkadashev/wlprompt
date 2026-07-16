// SPDX-License-Identifier:  GPL-3.0-or-later
//! Readline-like utilities for Wayland.

// LATER: ideally I'd like this not to depend on SCTK, but rather be some internal thing, and then
// there could be a layer that translates from SCTK to this internal thing. This way there could be
// different "drivers" instead of hard dependency on the SCTK - to the point where this could be not
// a wayland-specific library, but rather generic implementation that can work with pretty much any
// input. But for the MVP dependency on the SCTK will do.
use smithay_client_toolkit::seat::keyboard;

pub struct KeyPress {
    pub modifiers: keyboard::Modifiers,
    pub key_event: keyboard::KeyEvent,
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
            Action::AppendText(text) => self.text.push_str(text),
            Action::NoOp => {}
        }
        self.text.as_ref()
    }

    pub fn current_text(&self) -> &str {
        self.text.as_ref()
    }
}

enum Action<'a> {
    NoOp,
    AppendText(&'a str),
}

fn keypress_to_action(key: &KeyPress) -> Action<'_> {
    if key.modifiers.ctrl || key.modifiers.alt || key.modifiers.logo {
        return Action::NoOp;
    }
    if let Some(ch) = &key.key_event.utf8 {
        // Some keys (Enter, Tab, Escape, Backspace, ...) deliver a utf8 value that is a control
        // character. We must not insert those into the buffer; they will get dedicated handling
        // later.
        if ch.chars().any(char::is_control) {
            return Action::NoOp;
        }
        return Action::AppendText(ch);
    }
    Action::NoOp
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! key {
        ($ch:expr) => { key!($ch; ) };
        ($ch:expr; $( $mod:ident ),*) => {
            KeyPress {
                modifiers: keyboard::Modifiers{
                    $($mod: true,)*
                    ..keyboard::Modifiers::default()
                },
                key_event: keyboard::KeyEvent {
                    utf8: Some($ch.to_string()),
                    time: 42,                      // Not used anyway
                    raw_code: 42,                  // Not used
                    keysym: keyboard::Keysym::Tab, // Not used
                },
            }
        };
    }

    #[test]
    fn test_basic_text_appended() {
        let mut e = Editor::new();
        assert_eq!(e.handle_key(&key!("h")), "h");
        assert_eq!(e.handle_key(&key!("i")), "hi");
    }

    #[test]
    fn test_unknown_combinations_ignored() {
        let mut e = Editor::new();
        assert_eq!(e.handle_key(&key!("a")), "a");
        assert_eq!(e.handle_key(&key!("b")), "ab");
        assert_eq!(e.handle_key(&key!("c"; ctrl)), "ab");
        assert_eq!(e.handle_key(&key!("c"; alt)), "ab");
        assert_eq!(e.handle_key(&key!("c"; logo)), "ab");
    }

    #[test]
    fn test_non_character_keys_ignored() {
        let mut e = Editor::new();
        e.handle_key(&key!("a"));
        let press = KeyPress {
            modifiers: keyboard::Modifiers::default(),
            key_event: keyboard::KeyEvent {
                utf8: None,
                time: 42,
                raw_code: 42,
                keysym: keyboard::Keysym::Left,
            },
        };
        assert_eq!(e.handle_key(&press), "a");
    }

    #[test]
    fn test_shift_not_filtered() {
        let mut e = Editor::new();
        assert_eq!(e.handle_key(&key!("a")), "a");
        assert_eq!(e.handle_key(&key!("B"; shift)), "aB");
    }

    #[test]
    fn test_multiple_modifiers_ignored() {
        let mut e = Editor::new();
        assert_eq!(e.handle_key(&key!("a")), "a");
        assert_eq!(e.handle_key(&key!("x"; ctrl, alt)), "a");
        assert_eq!(e.handle_key(&key!("x"; ctrl, logo)), "a");
        assert_eq!(e.handle_key(&key!("x"; ctrl, alt, logo)), "a");
    }

    #[test]
    fn test_special_characters() {
        let mut e = Editor::new();
        assert_eq!(e.handle_key(&key!(" ")), " ");
        assert_eq!(e.handle_key(&key!("!")), " !");
        assert_eq!(e.handle_key(&key!("ñ")), " !ñ");
        assert_eq!(e.handle_key(&key!("日")), " !ñ日");
    }

    #[test]
    fn test_new_editor_is_empty() {
        let mut e = Editor::new();
        let press = KeyPress {
            modifiers: keyboard::Modifiers::default(),
            key_event: keyboard::KeyEvent {
                utf8: None,
                time: 42,
                raw_code: 42,
                keysym: keyboard::Keysym::Tab,
            },
        };
        assert_eq!(e.handle_key(&press), "");
    }

    #[test]
    fn test_multi_char_utf8_event() {
        let mut e = Editor::new();
        assert_eq!(e.handle_key(&key!("abc")), "abc");
        assert_eq!(e.handle_key(&key!("d")), "abcd");
    }

    #[test]
    fn test_control_characters_ignored() {
        let mut e = Editor::new();
        assert_eq!(e.handle_key(&key!("a")), "a");
        // Keys like Enter, Tab, Escape and Backspace deliver a control character via utf8;
        // none of them must end up in the buffer.
        assert_eq!(e.handle_key(&key!("\r")), "a"); // Enter / Return
        assert_eq!(e.handle_key(&key!("\n")), "a"); // Line feed
        assert_eq!(e.handle_key(&key!("\t")), "a"); // Tab
        assert_eq!(e.handle_key(&key!("\u{1b}")), "a"); // Escape
        assert_eq!(e.handle_key(&key!("\u{8}")), "a"); // Backspace
        assert_eq!(e.handle_key(&key!("\u{7f}")), "a"); // Delete
        // Printable input still works afterwards.
        assert_eq!(e.handle_key(&key!("b")), "ab");
    }
}
