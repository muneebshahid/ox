use std::time::Duration;

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
use crossterm::event::{Event as CEvent, EventStream};
use futures::{FutureExt, StreamExt};
use tokio::time::{self, MissedTickBehavior};

use super::{
    render_meta::build_render_meta,
    state::{StateCommand, TuiState},
    terminal::UiRenderer,
    ui_action,
};

const REDRAW_INTERVAL_MS: u64 = 33;
const MAX_DRAINED_INPUT_EVENTS_PER_LOOP: usize = 64;

struct UiRuntime {
    renderer: UiRenderer,
    state: TuiState,
    subscription: Subscription,
    input_events: EventStream,
    ticker: time::Interval,
}

impl UiRuntime {
    fn new(app: &AppContext) -> Result<Self> {
        let render_meta = build_render_meta(app);
        Ok(Self {
            renderer: UiRenderer::new(render_meta)?,
            state: TuiState::new(),
            subscription: app.agent_bridge.subscribe(),
            input_events: EventStream::new(),
            ticker: create_ticker(),
        })
    }
}

fn create_ticker() -> time::Interval {
    let mut ticker = time::interval(Duration::from_millis(REDRAW_INTERVAL_MS));
    ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
    ticker
}

pub async fn run(app: &AppContext, session_state: &mut SessionManager) -> Result<()> {
    let mut ui = UiRuntime::new(app)?;
    run_main_loop(app, session_state, &mut ui).await
}

async fn run_main_loop(
    app: &AppContext,
    session_state: &mut SessionManager,
    ui: &mut UiRuntime,
) -> Result<()> {
    loop {
        let mut should_exit = false;
        tokio::select! {
            maybe_event = ui.input_events.next() => {
                let should_exit_from_input =
                    process_input_burst(app, session_state, ui, maybe_event).await?;
                if should_exit_from_input {
                    should_exit = true;
                }
            }
            event = ui.subscription.recv() => {
                if handle_core_event(&mut ui.state, event) {
                    should_exit = true;
                }
            }
            _ = ui.ticker.tick() => {
            }
            interrupt = tokio::signal::ctrl_c() => {
                handle_interrupt(ui, interrupt);
                should_exit = true;
            }
        }

        ui.renderer.draw_if_needed(&mut ui.state)?;
        if should_exit {
            break;
        }
    }

    Ok(())
}

async fn process_input_burst(
    app: &AppContext,
    session_state: &mut SessionManager,
    ui: &mut UiRuntime,
    maybe_event: Option<Result<CEvent, std::io::Error>>,
) -> Result<bool> {
    if handle_main_input_event(app, session_state, ui, maybe_event).await? {
        return Ok(true);
    }

    drain_pending_main_input_events(app, session_state, ui).await
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
        Ok(event) => ui
            .state
            .handle_ui_action(ui_action::adapter::to_ui_action(event)),
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

async fn drain_pending_main_input_events(
    app: &AppContext,
    session_state: &mut SessionManager,
    ui: &mut UiRuntime,
) -> Result<bool> {
    for _ in 0..MAX_DRAINED_INPUT_EVENTS_PER_LOOP {
        let Some(ready_event) = ui.input_events.next().now_or_never() else {
            break;
        };
        let should_exit = handle_main_input_event(app, session_state, ui, ready_event).await?;
        if should_exit {
            return Ok(true);
        }
    }

    Ok(false)
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

fn handle_interrupt(ui: &mut UiRuntime, interrupt: Result<(), std::io::Error>) {
    let message = match interrupt {
        Ok(()) => "Interrupted".to_string(),
        Err(err) => format!("Ctrl+C error: {err}"),
    };
    ui.state.handle_agent_event(CoreEvent::Error(message));
}

async fn run_active_turn(
    app: &AppContext,
    session_state: &mut SessionManager,
    input: &str,
    ui: &mut UiRuntime,
) -> Result<bool> {
    let mut run_future = Box::pin(agent::run(app, session_state, input));
    let mut agent_result = None;
    let mut should_exit = false;

    while agent_result.is_none() && !should_exit {
        tokio::select! {
            result = &mut run_future => {
                agent_result = Some(result);
            }
            maybe_event = ui.input_events.next() => {
                let Some(event_result) = maybe_event else {
                    should_exit = true;
                    continue;
                };
                let should_exit_from_input =
                    process_active_turn_input_burst(ui, event_result);
                if should_exit_from_input {
                    should_exit = true;
                }
            }
            event = ui.subscription.recv() => {
                if handle_core_event(&mut ui.state, event) {
                    should_exit = true;
                }
            }
            _ = ui.ticker.tick() => {
            }
            interrupt = tokio::signal::ctrl_c() => {
                handle_interrupt(ui, interrupt);
                should_exit = true;
            }
        }
        ui.renderer.draw_if_needed(&mut ui.state)?;
    }

    match agent_result {
        Some(Ok(())) if !should_exit => Ok(false),
        Some(Err(err)) if !should_exit => {
            ui.state
                .handle_agent_event(CoreEvent::Error(format!("Error: {err}")));
            Ok(false)
        }
        _ => Ok(true),
    }
}

fn handle_input_during_active_turn(
    state: &mut TuiState,
    event_result: Result<CEvent, std::io::Error>,
) -> bool {
    match event_result {
        Ok(event) => matches!(
            state.handle_ui_action_during_active_turn(ui_action::adapter::to_ui_action(event)),
            StateCommand::Quit
        ),
        Err(err) => {
            state.handle_agent_event(CoreEvent::Error(format!("Input error: {err}")));
            false
        }
    }
}

fn process_active_turn_input_burst(
    ui: &mut UiRuntime,
    event_result: Result<CEvent, std::io::Error>,
) -> bool {
    if handle_input_during_active_turn(&mut ui.state, event_result) {
        return true;
    }

    drain_pending_active_turn_input_events(ui)
}

fn drain_pending_active_turn_input_events(ui: &mut UiRuntime) -> bool {
    for _ in 0..MAX_DRAINED_INPUT_EVENTS_PER_LOOP {
        let Some(ready_event) = ui.input_events.next().now_or_never() else {
            break;
        };
        let Some(event_result) = ready_event else {
            return true;
        };
        if handle_input_during_active_turn(&mut ui.state, event_result) {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::{handle_core_event, handle_input_during_active_turn};
    use crate::{
        events::{hub::RecvError, types::CoreEvent},
        tui::{
            state::TuiState,
            ui_action::{self, UiAction},
        },
    };
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

    #[test]
    fn core_event_closed_requests_exit() {
        let mut state = TuiState::new();
        let should_exit = handle_core_event(&mut state, Err(RecvError::Closed));
        assert!(should_exit);
    }

    #[test]
    fn core_event_lagged_records_error_and_keeps_running() {
        let mut state = TuiState::new();
        let should_exit = handle_core_event(&mut state, Err(RecvError::Lagged(7)));

        assert!(!should_exit);
        assert_eq!(state.status(), "Lagged: dropped 7 events");
    }

    #[test]
    fn core_event_ok_is_applied_to_state() {
        let mut state = TuiState::new();
        let should_exit = handle_core_event(
            &mut state,
            Ok(CoreEvent::AgentTextDelta("hello".to_string())),
        );

        assert!(!should_exit);
        assert_eq!(state.output_log(), "hello");
    }

    #[test]
    fn active_turn_quit_key_exits_immediately() {
        let mut state = TuiState::new();
        let should_exit = handle_input_during_active_turn(
            &mut state,
            Ok(CEvent::Key(KeyEvent {
                code: KeyCode::Esc,
                modifiers: KeyModifiers::NONE,
                kind: KeyEventKind::Press,
                state: KeyEventState::NONE,
            })),
        );

        assert!(should_exit);
    }

    #[test]
    fn active_turn_only_allows_scroll_selection_or_viewport_actions() {
        let mut state = TuiState::new();
        let _ = state.take_dirty();
        let _ = handle_input_during_active_turn(&mut state, Ok(CEvent::Key(key(KeyCode::PageUp))));
        assert_eq!(state.output_scroll_lines_from_bottom(), 8);
        assert!(state.take_dirty());

        let _ = handle_input_during_active_turn(
            &mut state,
            Ok(CEvent::Key(KeyEvent {
                code: KeyCode::Char('x'),
                modifiers: KeyModifiers::NONE,
                kind: KeyEventKind::Press,
                state: KeyEventState::NONE,
            })),
        );
        assert_eq!(state.input(), "");
        assert_eq!(
            ui_action::adapter::to_ui_action(CEvent::Key(key(KeyCode::Char('x')))),
            UiAction::Insert('x')
        );
    }

    #[test]
    fn active_turn_allows_output_selection_actions() {
        let mut state = TuiState::new();
        state.apply_render_sync(
            0,
            u16::MAX,
            crate::tui::state::OutputViewport {
                x: 0,
                y: 0,
                width: 4,
                height: 2,
            },
            vec![
                "abcd".chars().map(|ch| ch.to_string()).collect(),
                "efgh".chars().map(|ch| ch.to_string()).collect(),
            ],
        );

        let down = CEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Down(MouseButton::Left),
            column: 1,
            row: 0,
            modifiers: KeyModifiers::NONE,
        });
        let drag = CEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Drag(MouseButton::Left),
            column: 2,
            row: 1,
            modifiers: KeyModifiers::NONE,
        });
        let up = CEvent::Mouse(MouseEvent {
            kind: MouseEventKind::Up(MouseButton::Left),
            column: 2,
            row: 1,
            modifiers: KeyModifiers::NONE,
        });

        assert!(!handle_input_during_active_turn(&mut state, Ok(down)));
        assert!(!handle_input_during_active_turn(&mut state, Ok(drag)));
        assert!(!handle_input_during_active_turn(&mut state, Ok(up)));
        assert_eq!(state.take_pending_copy_text(), Some("bcd\nefg".to_string()));
    }

    #[test]
    fn active_turn_input_errors_are_reported_to_state() {
        let mut state = TuiState::new();
        let io_err = std::io::Error::other("boom");

        let should_exit = handle_input_during_active_turn(&mut state, Err(io_err));
        assert!(!should_exit);
        assert_eq!(state.status(), "Input error: boom");
    }
}
