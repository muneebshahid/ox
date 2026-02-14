use crossterm::event::{
    Event as CEvent, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind,
};

use super::action::UiAction;

const KEY_SCROLL_LINES: u16 = 1;
const PAGE_SCROLL_LINES: u16 = 8;
const MOUSE_SCROLL_LINES: u16 = 3;

pub(super) fn to_ui_action(event: CEvent) -> UiAction {
    match event {
        CEvent::Key(key) => to_ui_action_from_key(key),
        CEvent::Paste(pasted) => UiAction::Paste(pasted),
        CEvent::Mouse(mouse) => to_ui_action_from_mouse(mouse),
        CEvent::Resize(_, _) => UiAction::ViewportChanged,
        _ => UiAction::Ignore,
    }
}

pub(super) const fn is_quit_event(event: &CEvent) -> bool {
    matches!(event, CEvent::Key(key) if is_quit_key(key))
}

fn to_ui_action_from_key(key: KeyEvent) -> UiAction {
    if is_quit_key(&key) {
        return UiAction::Quit;
    }

    match key.code {
        KeyCode::Enter => UiAction::Submit,
        KeyCode::Backspace => UiAction::Backspace,
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
    }
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
    use crate::tui::action::UiAction;
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
    fn maps_arrow_and_page_keys_to_scroll_inputs() {
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
            to_ui_action(CEvent::Paste("hello".to_string())),
            UiAction::Paste("hello".to_string())
        );
    }

    #[test]
    fn ignores_modified_char_input() {
        assert_eq!(
            to_ui_action(CEvent::Key(key_with_modifiers(
                KeyCode::Char('a'),
                KeyModifiers::CONTROL
            ))),
            UiAction::Ignore
        );
        assert_eq!(
            to_ui_action(CEvent::Key(key_with_modifiers(
                KeyCode::Char('a'),
                KeyModifiers::ALT
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
