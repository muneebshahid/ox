use super::ui_action::UiAction;

pub(in crate::tui) const fn allow_ui_action_during_active_turn(action: &UiAction) -> bool {
    match action {
        UiAction::Submit | UiAction::Ignore => false,
        UiAction::Quit
        | UiAction::Insert(_)
        | UiAction::Backspace
        | UiAction::Delete
        | UiAction::MoveCursorLeft
        | UiAction::MoveCursorRight
        | UiAction::MoveCursorUp
        | UiAction::MoveCursorDown
        | UiAction::MoveCursorWordLeft
        | UiAction::MoveCursorWordRight
        | UiAction::MoveCursorLineStart
        | UiAction::MoveCursorLineEnd
        | UiAction::DeleteToLineStart
        | UiAction::DeleteToLineEnd
        | UiAction::DeleteWordLeft
        | UiAction::Paste(_)
        | UiAction::ScrollUp { .. }
        | UiAction::ScrollDown { .. }
        | UiAction::OutputSelectStart { .. }
        | UiAction::OutputSelectDrag { .. }
        | UiAction::OutputSelectEnd { .. }
        | UiAction::ViewportChanged => true,
    }
}

#[cfg(test)]
mod tests {
    use super::allow_ui_action_during_active_turn;
    use crate::tui::ui_action::UiAction;

    #[test]
    fn blocks_submit_and_ignore_during_active_turn() {
        assert!(!allow_ui_action_during_active_turn(&UiAction::Submit));
        assert!(!allow_ui_action_during_active_turn(&UiAction::Ignore));
    }

    #[test]
    fn allows_typing_and_selection_during_active_turn() {
        assert!(allow_ui_action_during_active_turn(&UiAction::Insert('x')));
        assert!(allow_ui_action_during_active_turn(&UiAction::Paste(
            "hello".to_string()
        )));
        assert!(allow_ui_action_during_active_turn(
            &UiAction::OutputSelectStart { col: 0, row: 0 }
        ));
    }
}
