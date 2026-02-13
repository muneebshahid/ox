use crossterm::event::{Event as CEvent, KeyCode, KeyEvent, KeyModifiers};

use super::state::UserInput;

pub(super) fn to_user_input(event: CEvent) -> UserInput {
    match event {
        CEvent::Key(key) => to_user_input_from_key(key),
        CEvent::Paste(pasted) => UserInput::Paste(pasted),
        _ => UserInput::Ignore,
    }
}

pub(super) const fn is_quit_event(event: &CEvent) -> bool {
    matches!(event, CEvent::Key(key) if is_quit_key(key))
}

fn to_user_input_from_key(key: KeyEvent) -> UserInput {
    if is_quit_key(&key) {
        return UserInput::Quit;
    }

    match key.code {
        KeyCode::Enter => UserInput::Submit,
        KeyCode::Backspace => UserInput::Backspace,
        KeyCode::Char(c)
            if !key
                .modifiers
                .intersects(KeyModifiers::ALT | KeyModifiers::CONTROL) =>
        {
            UserInput::Insert(c)
        }
        _ => UserInput::Ignore,
    }
}

const fn is_quit_key(key: &KeyEvent) -> bool {
    match key.code {
        KeyCode::Esc => true,
        KeyCode::Char('c') => key.modifiers.contains(KeyModifiers::CONTROL),
        _ => false,
    }
}
