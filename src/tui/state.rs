use crate::events::types::CoreEvent;

#[derive(Debug, PartialEq, Eq)]
pub enum UserInput {
    Insert(char),
    Backspace,
    Submit,
    Paste(String),
    Quit,
    Ignore,
}

#[derive(Debug, PartialEq, Eq)]
pub enum StateCommand {
    None,
    Submit(String),
    Quit,
}

pub struct TuiState {
    transcript: String,
    status: String,
    input: String,
    dirty: bool,
}

impl TuiState {
    pub fn new() -> Self {
        Self {
            transcript: String::new(),
            status: "Idle".to_string(),
            input: String::new(),
            dirty: true,
        }
    }

    pub fn transcript(&self) -> &str {
        &self.transcript
    }

    pub fn status(&self) -> &str {
        &self.status
    }

    pub fn input(&self) -> &str {
        &self.input
    }

    pub fn take_dirty(&mut self) -> bool {
        std::mem::take(&mut self.dirty)
    }

    pub fn handle_agent_event(&mut self, event: CoreEvent) {
        match event {
            CoreEvent::AgentTurnStart => {
                self.status = "Running".to_string();
                self.mark_dirty();
            }
            CoreEvent::AgentTextDelta(delta) => {
                self.transcript.push_str(&delta);
                self.mark_dirty();
            }
            CoreEvent::AgentTurnEnd => {
                self.status = "Idle".to_string();
                if !self.transcript.ends_with('\n') {
                    self.transcript.push('\n');
                }
                self.mark_dirty();
            }
            CoreEvent::Error(message) => {
                self.status = message;
                self.mark_dirty();
            }
            CoreEvent::Tick | CoreEvent::ShutdownRequested => {}
        }
    }

    pub fn handle_user_input(&mut self, input: UserInput) -> StateCommand {
        match input {
            UserInput::Insert(c) => {
                self.input.push(c);
                self.mark_dirty();
                StateCommand::None
            }
            UserInput::Backspace => {
                self.input.pop();
                self.mark_dirty();
                StateCommand::None
            }
            UserInput::Paste(pasted) => {
                self.input.push_str(&pasted);
                self.mark_dirty();
                StateCommand::None
            }
            UserInput::Submit => self.submit_input(),
            UserInput::Quit => StateCommand::Quit,
            UserInput::Ignore => StateCommand::None,
        }
    }

    fn submit_input(&mut self) -> StateCommand {
        let submitted = self.input.trim().to_string();
        self.input.clear();
        self.mark_dirty();

        if submitted.is_empty() {
            return StateCommand::None;
        }
        if submitted == "exit" {
            return StateCommand::Quit;
        }

        self.push_user_input(&submitted);
        self.status = "Running".to_string();
        self.mark_dirty();
        StateCommand::Submit(submitted)
    }

    fn push_user_input(&mut self, input: &str) {
        if !self.transcript.is_empty() && !self.transcript.ends_with('\n') {
            self.transcript.push('\n');
        }
        self.transcript.push_str("> ");
        self.transcript.push_str(input);
        self.transcript.push('\n');
    }

    const fn mark_dirty(&mut self) {
        self.dirty = true;
    }
}

#[cfg(test)]
mod tests {
    use super::{StateCommand, TuiState, UserInput};
    use crate::events::types::CoreEvent;

    #[test]
    fn appends_deltas_and_updates_status() {
        let mut state = TuiState::new();
        state.handle_agent_event(CoreEvent::AgentTurnStart);
        state.handle_agent_event(CoreEvent::AgentTextDelta("hello".to_string()));
        state.handle_agent_event(CoreEvent::AgentTextDelta(" world".to_string()));
        state.handle_agent_event(CoreEvent::AgentTurnEnd);
        assert_eq!(state.status(), "Idle");
        assert_eq!(state.transcript(), "hello world\n");
        assert_eq!(state.input(), "");
        assert!(state.take_dirty());
    }

    #[test]
    fn submit_creates_command_and_echoes_transcript() {
        let mut state = TuiState::new();
        state.handle_user_input(UserInput::Insert('h'));
        state.handle_user_input(UserInput::Insert('i'));

        let command = state.handle_user_input(UserInput::Submit);
        assert_eq!(command, StateCommand::Submit("hi".to_string()));
        assert_eq!(state.transcript(), "> hi\n");
        assert_eq!(state.status(), "Running");
        assert_eq!(state.input(), "");
    }

    #[test]
    fn submit_exit_returns_quit() {
        let mut state = TuiState::new();
        state.handle_user_input(UserInput::Insert('e'));
        state.handle_user_input(UserInput::Insert('x'));
        state.handle_user_input(UserInput::Insert('i'));
        state.handle_user_input(UserInput::Insert('t'));

        let command = state.handle_user_input(UserInput::Submit);
        assert_eq!(command, StateCommand::Quit);
    }

    #[test]
    fn take_dirty_resets_flag() {
        let mut state = TuiState::new();
        assert!(state.take_dirty());
        assert!(!state.take_dirty());
    }
}
