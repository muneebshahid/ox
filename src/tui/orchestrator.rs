use std::time::{Duration, Instant};

use crate::{
    agent,
    app_context::AppContext,
    events::{
        hub::{RecvError, Subscription},
        types::CoreEvent,
    },
    session::SessionManager,
};
use anyhow::Result;
use crossterm::event::{Event as CEvent, EventStream, KeyCode, KeyEvent, KeyModifiers};
use futures::StreamExt;
use tokio::time::{self, MissedTickBehavior};

use super::{
    render::RenderMeta,
    state::{StateCommand, TuiState, UserInput},
    terminal::TerminalGuard,
};

const REDRAW_INTERVAL_MS: u64 = 33;
const RUNNING_STATUS_REFRESH_INTERVAL: Duration = Duration::from_millis(250);

struct UiRuntime {
    terminal: TerminalGuard,
    state: TuiState,
    render_meta: RenderMeta,
    subscription: Subscription,
    input_events: EventStream,
    ticker: time::Interval,
    running_status_last_draw_at: Option<Instant>,
}

impl UiRuntime {
    fn new(app: &AppContext) -> Result<Self> {
        let cwd = current_dir_for_banner();
        let git_branch = current_git_branch();
        Ok(Self {
            terminal: TerminalGuard::new()?,
            state: TuiState::new(),
            render_meta: RenderMeta::new(
                app.auth.model().to_string(),
                app.auth.reasoning_setting().to_string(),
                app.auth.mode_name().to_string(),
                cwd,
                git_branch,
            ),
            subscription: app.agent_bridge.subscribe(),
            input_events: EventStream::new(),
            ticker: create_ticker(),
            running_status_last_draw_at: None,
        })
    }

    fn draw_if_dirty(&mut self) -> Result<()> {
        let now = Instant::now();
        let is_running = self.state.status_is_running();
        let refresh_due = is_running
            && self
                .running_status_last_draw_at
                .is_none_or(|last| now.duration_since(last) >= RUNNING_STATUS_REFRESH_INTERVAL);
        let dirty = self.state.take_dirty();

        if dirty || refresh_due {
            self.terminal.draw(&self.state, &self.render_meta)?;
            if is_running {
                self.running_status_last_draw_at = Some(now);
            } else {
                self.running_status_last_draw_at = None;
            }
        }
        Ok(())
    }
}

fn current_dir_for_banner() -> String {
    let path =
        std::env::current_dir().map_or_else(|_| ".".to_string(), |path| path.display().to_string());

    if let Ok(home) = std::env::var("HOME")
        && path.starts_with(&home)
    {
        return format!("~{}", &path[home.len()..]);
    }

    path
}

fn current_git_branch() -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let branch = String::from_utf8(output.stdout).ok()?.trim().to_string();
    if branch.is_empty() || branch == "HEAD" {
        return None;
    }
    Some(branch)
}

pub async fn run(app: &AppContext, session_state: &mut SessionManager) -> Result<()> {
    let mut ui = UiRuntime::new(app)?;
    run_main_loop(app, session_state, &mut ui).await
}

fn create_ticker() -> time::Interval {
    let mut ticker = time::interval(Duration::from_millis(REDRAW_INTERVAL_MS));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    ticker
}

async fn run_main_loop(
    app: &AppContext,
    session_state: &mut SessionManager,
    ui: &mut UiRuntime,
) -> Result<()> {
    loop {
        tokio::select! {
            maybe_event = ui.input_events.next() => {
                if handle_main_input_event(app, session_state, ui, maybe_event).await? {
                    break;
                }
            }
            event = ui.subscription.recv() => {
                if handle_core_event(&mut ui.state, event) {
                    break;
                }
                ui.draw_if_dirty()?;
            }
            _ = ui.ticker.tick() => {
                ui.draw_if_dirty()?;
            }
            interrupt = tokio::signal::ctrl_c() => {
                handle_interrupt(ui, interrupt)?;
                break;
            }
        }
    }

    Ok(())
}

async fn handle_main_input_event(
    app: &AppContext,
    session_state: &mut SessionManager,
    ui: &mut UiRuntime,
    maybe_event: Option<Result<CEvent, std::io::Error>>,
) -> Result<bool> {
    let Some(event_result) = maybe_event else {
        return Ok(true);
    };

    let command = match event_result {
        Ok(event) => ui.state.handle_user_input(to_user_input(event)),
        Err(err) => {
            ui.state
                .handle_agent_event(CoreEvent::Error(format!("Input error: {err}")));
            StateCommand::None
        }
    };

    match command {
        StateCommand::None => Ok(false),
        StateCommand::Quit => Ok(true),
        StateCommand::Submit(input) => run_active_turn(app, session_state, &input, ui).await,
    }
}

fn handle_core_event(state: &mut TuiState, event: Result<CoreEvent, RecvError>) -> bool {
    match event {
        Ok(event) => state.handle_agent_event(event),
        Err(RecvError::Lagged(dropped)) => {
            state.handle_agent_event(CoreEvent::Error(format!(
                "Lagged: dropped {dropped} events"
            )));
        }
        Err(RecvError::Closed) => return true,
    }
    false
}

fn handle_interrupt(ui: &mut UiRuntime, interrupt: Result<(), std::io::Error>) -> Result<()> {
    let message = match interrupt {
        Ok(()) => "Interrupted".to_string(),
        Err(err) => format!("Ctrl+C error: {err}"),
    };
    ui.state.handle_agent_event(CoreEvent::Error(message));
    ui.draw_if_dirty()?;
    Ok(())
}

async fn run_active_turn(
    app: &AppContext,
    session_state: &mut SessionManager,
    input: &str,
    ui: &mut UiRuntime,
) -> Result<bool> {
    let mut run_future = Box::pin(agent::run(app, session_state, input));

    let agent_result = loop {
        tokio::select! {
            result = &mut run_future => break Some(result),
            maybe_event = ui.input_events.next() => {
                let Some(event_result) = maybe_event else {
                    break None;
                };
                if should_quit_during_active_turn(&mut ui.state, event_result) {
                    break None;
                }
            }
            event = ui.subscription.recv() => {
                if handle_core_event(&mut ui.state, event) {
                    break None;
                }
                ui.draw_if_dirty()?;
            }
            _ = ui.ticker.tick() => {
                ui.draw_if_dirty()?;
            }
            interrupt = tokio::signal::ctrl_c() => {
                handle_interrupt(ui, interrupt)?;
                break None;
            }
        }
    };

    match agent_result {
        Some(Ok(())) => Ok(false),
        Some(Err(err)) => {
            ui.state
                .handle_agent_event(CoreEvent::Error(format!("Error: {err}")));
            Ok(false)
        }
        None => Ok(true),
    }
}

fn should_quit_during_active_turn(
    state: &mut TuiState,
    event_result: Result<CEvent, std::io::Error>,
) -> bool {
    match event_result {
        Ok(CEvent::Key(key)) => matches!(to_user_input_from_key(key), UserInput::Quit),
        Ok(_) => false,
        Err(err) => {
            state.handle_agent_event(CoreEvent::Error(format!("Input error: {err}")));
            false
        }
    }
}

fn to_user_input(event: CEvent) -> UserInput {
    match event {
        CEvent::Key(key) => to_user_input_from_key(key),
        CEvent::Paste(pasted) => UserInput::Paste(pasted),
        _ => UserInput::Ignore,
    }
}

fn to_user_input_from_key(key: KeyEvent) -> UserInput {
    match key.code {
        KeyCode::Esc => UserInput::Quit,
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => UserInput::Quit,
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
