use crossterm::event::{
    Event as CEvent, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind,
};

use super::UiAction;

const KEY_SCROLL_LINES: u16 = 1;
const PAGE_SCROLL_LINES: u16 = 8;
const MOUSE_SCROLL_LINES: u16 = 3;

pub(in crate::tui) fn to_ui_action(event: CEvent) -> UiAction {
    match event {
        CEvent::Key(key) => to_ui_action_from_key(key),
        CEvent::Paste(pasted) => UiAction::Paste(pasted),
        CEvent::Mouse(mouse) => to_ui_action_from_mouse(mouse),
        CEvent::Resize(_, _) => UiAction::ViewportChanged,
        _ => UiAction::Ignore,
    }
}

pub(in crate::tui) const fn is_quit_event(event: &CEvent) -> bool {
    matches!(event, CEvent::Key(key) if is_quit_key(key))
}

fn to_ui_action_from_key(key: KeyEvent) -> UiAction {
    if is_quit_key(&key) {
        let action = UiAction::Quit;
        debug_log_key_mapping(&key, &action);
        return action;
    }

    if let Some(shortcut_action) = cursor_and_delete_shortcut(key) {
        debug_log_key_mapping(&key, &shortcut_action);
        return shortcut_action;
    }

    if let Some(word_action) = option_word_shortcut(key) {
        debug_log_key_mapping(&key, &word_action);
        return word_action;
    }

    let action = match key.code {
        KeyCode::Enter => UiAction::Submit,
        KeyCode::Backspace => UiAction::Backspace,
        KeyCode::Delete => UiAction::Delete,
        KeyCode::Left => UiAction::MoveCursorLeft,
        KeyCode::Right => UiAction::MoveCursorRight,
        KeyCode::Home => UiAction::MoveCursorHome,
        KeyCode::End => UiAction::MoveCursorEnd,
        KeyCode::Up => UiAction::ScrollUp {
            lines: KEY_SCROLL_LINES,
        },
        KeyCode::Down => UiAction::ScrollDown {
            lines: KEY_SCROLL_LINES,
        },
        KeyCode::PageUp => UiAction::ScrollUp {
            lines: PAGE_SCROLL_LINES,
        },
        KeyCode::PageDown => UiAction::ScrollDown {
            lines: PAGE_SCROLL_LINES,
        },
        KeyCode::Char(c)
            if !key
                .modifiers
                .intersects(KeyModifiers::ALT | KeyModifiers::CONTROL) =>
        {
            UiAction::Insert(c)
        }
        _ => UiAction::Ignore,
    };
    debug_log_key_mapping(&key, &action);
    action
}

const fn cursor_and_delete_shortcut(key: KeyEvent) -> Option<UiAction> {
    if !key.modifiers.contains(KeyModifiers::CONTROL) || key.modifiers.contains(KeyModifiers::ALT) {
        return None;
    }

    match key.code {
        KeyCode::Char('a' | 'A') => Some(UiAction::MoveCursorHome),
        KeyCode::Char('e' | 'E') => Some(UiAction::MoveCursorEnd),
        KeyCode::Char('u' | 'U') => Some(UiAction::DeleteToStart),
        KeyCode::Char('k' | 'K') => Some(UiAction::DeleteToEnd),
        _ => None,
    }
}

const fn option_word_shortcut(key: KeyEvent) -> Option<UiAction> {
    if !key.modifiers.contains(KeyModifiers::ALT) || key.modifiers.contains(KeyModifiers::CONTROL) {
        return None;
    }

    // Terminal variance:
    // - Some terminals send Alt+Left/Alt+Right directly.
    // - Others (notably macOS Option-word shortcuts) send Alt+B / Alt+F.
    // We support both so Option word navigation works consistently.
    match key.code {
        // Many terminals emit Option+Left/Right as Alt+B / Alt+F.
        KeyCode::Left | KeyCode::Char('b' | 'B') => Some(UiAction::MoveCursorWordLeft),
        KeyCode::Right | KeyCode::Char('f' | 'F') => Some(UiAction::MoveCursorWordRight),
        KeyCode::Backspace => Some(UiAction::DeleteWordLeft),
        _ => None,
    }
}

fn debug_log_key_mapping(key: &KeyEvent, action: &UiAction) {
    if std::env::var_os("OX_DEBUG_KEYS").is_none() {
        return;
    }

    // Debug logs intentionally go to a file instead of stderr to avoid corrupting
    // the active terminal UI. Override path with OX_DEBUG_KEYS_FILE.
    let path =
        std::env::var("OX_DEBUG_KEYS_FILE").unwrap_or_else(|_| "/tmp/ox-debug-keys.log".to_string());
    let Ok(mut file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    else {
        return;
    };

    let _ = std::io::Write::write_fmt(
        &mut file,
        format_args!(
            "[ox-keys] code={:?} modifiers={:?} kind={:?} state={:?} -> {:?}\n",
            key.code, key.modifiers, key.kind, key.state, action
        ),
    );
}

const fn to_ui_action_from_mouse(mouse: MouseEvent) -> UiAction {
    match mouse.kind {
        MouseEventKind::ScrollUp => UiAction::ScrollUp {
            lines: MOUSE_SCROLL_LINES,
        },
        MouseEventKind::ScrollDown => UiAction::ScrollDown {
            lines: MOUSE_SCROLL_LINES,
        },
        _ => UiAction::Ignore,
    }
}

const fn is_quit_key(key: &KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => true,
        KeyCode::Char('c') => key.modifiers.contains(KeyModifiers::CONTROL),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{is_quit_event, to_ui_action};
    use crate::tui::ui_action::UiAction;
    use crossterm::event::{
        Event as CEvent, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers, MouseButton,
        MouseEvent, MouseEventKind,
    };

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    fn key_with_modifiers(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn maps_resize_event_to_viewport_changed() {
        assert_eq!(
            to_ui_action(CEvent::Resize(80, 24)),
            UiAction::ViewportChanged
        );
    }

    #[test]
    fn maps_arrows_to_scroll_and_cursor_inputs() {
        assert_eq!(
            to_ui_action(CEvent::Key(key(KeyCode::Home))),
            UiAction::MoveCursorHome
        );
        assert_eq!(
            to_ui_action(CEvent::Key(key(KeyCode::End))),
            UiAction::MoveCursorEnd
        );
        assert_eq!(
            to_ui_action(CEvent::Key(key(KeyCode::Left))),
            UiAction::MoveCursorLeft
        );
        assert_eq!(
            to_ui_action(CEvent::Key(key(KeyCode::Right))),
            UiAction::MoveCursorRight
        );
        assert_eq!(
            to_ui_action(CEvent::Key(key(KeyCode::Up))),
            UiAction::ScrollUp { lines: 1 }
        );
        assert_eq!(
            to_ui_action(CEvent::Key(key(KeyCode::Down))),
            UiAction::ScrollDown { lines: 1 }
        );
        assert_eq!(
            to_ui_action(CEvent::Key(key(KeyCode::PageUp))),
            UiAction::ScrollUp { lines: 8 }
        );
        assert_eq!(
            to_ui_action(CEvent::Key(key(KeyCode::PageDown))),
            UiAction::ScrollDown { lines: 8 }
        );
    }

    #[test]
    fn maps_option_word_navigation_and_delete_shortcuts() {
        assert_eq!(
            to_ui_action(CEvent::Key(key_with_modifiers(
                KeyCode::Left,
                KeyModifiers::ALT
            ))),
            UiAction::MoveCursorWordLeft
        );
        assert_eq!(
            to_ui_action(CEvent::Key(key_with_modifiers(
                KeyCode::Right,
                KeyModifiers::ALT
            ))),
            UiAction::MoveCursorWordRight
        );
        assert_eq!(
            to_ui_action(CEvent::Key(key_with_modifiers(
                KeyCode::Backspace,
                KeyModifiers::ALT
            ))),
            UiAction::DeleteWordLeft
        );
        assert_eq!(
            to_ui_action(CEvent::Key(key_with_modifiers(
                KeyCode::Char('b'),
                KeyModifiers::ALT
            ))),
            UiAction::MoveCursorWordLeft
        );
        assert_eq!(
            to_ui_action(CEvent::Key(key_with_modifiers(
                KeyCode::Char('f'),
                KeyModifiers::ALT
            ))),
            UiAction::MoveCursorWordRight
        );
    }

    #[test]
    fn maps_mouse_wheel_to_scroll_inputs() {
        let up = MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        };
        let down = MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::NONE,
        };

        assert_eq!(
            to_ui_action(CEvent::Mouse(up)),
            UiAction::ScrollUp { lines: 3 }
        );
        assert_eq!(
            to_ui_action(CEvent::Mouse(down)),
            UiAction::ScrollDown { lines: 3 }
        );
    }

    #[test]
    fn quit_event_only_matches_quit_keys() {
        assert!(is_quit_event(&CEvent::Key(KeyEvent {
            code: KeyCode::Esc,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        })));
        assert!(is_quit_event(&CEvent::Key(KeyEvent {
            code: KeyCode::Char('c'),
            modifiers: KeyModifiers::CONTROL,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        })));
        assert!(!is_quit_event(&CEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 1,
            row: 1,
            modifiers: KeyModifiers::NONE,
        })));
    }

    #[test]
    fn maps_submit_backspace_and_paste() {
        assert_eq!(
            to_ui_action(CEvent::Key(key(KeyCode::Enter))),
            UiAction::Submit
        );
        assert_eq!(
            to_ui_action(CEvent::Key(key(KeyCode::Backspace))),
            UiAction::Backspace
        );
        assert_eq!(
            to_ui_action(CEvent::Key(key(KeyCode::Delete))),
            UiAction::Delete
        );
        assert_eq!(
            to_ui_action(CEvent::Paste("hello".to_string())),
            UiAction::Paste("hello".to_string())
        );
    }

    #[test]
    fn maps_ctrl_cursor_and_delete_shortcuts() {
        assert_eq!(
            to_ui_action(CEvent::Key(key_with_modifiers(
                KeyCode::Char('a'),
                KeyModifiers::CONTROL
            ))),
            UiAction::MoveCursorHome
        );
        assert_eq!(
            to_ui_action(CEvent::Key(key_with_modifiers(
                KeyCode::Char('e'),
                KeyModifiers::CONTROL
            ))),
            UiAction::MoveCursorEnd
        );
        assert_eq!(
            to_ui_action(CEvent::Key(key_with_modifiers(
                KeyCode::Char('u'),
                KeyModifiers::CONTROL
            ))),
            UiAction::DeleteToStart
        );
        assert_eq!(
            to_ui_action(CEvent::Key(key_with_modifiers(
                KeyCode::Char('k'),
                KeyModifiers::CONTROL
            ))),
            UiAction::DeleteToEnd
        );
    }

    #[test]
    fn ignores_other_modified_char_input() {
        assert_eq!(
            to_ui_action(CEvent::Key(key_with_modifiers(
                KeyCode::Char('a'),
                KeyModifiers::ALT
            ))),
            UiAction::Ignore
        );
        assert_eq!(
            to_ui_action(CEvent::Key(key_with_modifiers(
                KeyCode::Char('z'),
                KeyModifiers::CONTROL
            ))),
            UiAction::Ignore
        );
    }

    #[test]
    fn ignores_non_scroll_mouse_and_non_quit_keys_for_quit_check() {
        assert_eq!(
            to_ui_action(CEvent::Mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column: 0,
                row: 0,
                modifiers: KeyModifiers::NONE,
            })),
            UiAction::Ignore
        );
        assert!(!is_quit_event(&CEvent::Key(key(KeyCode::Char('q')))));
    }
}
