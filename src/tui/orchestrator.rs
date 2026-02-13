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
use futures::StreamExt;
use tokio::time::{self, MissedTickBehavior};

use super::{
    input,
    render::RenderMeta,
    state::{StateCommand, TuiState},
    terminal::UiRenderer,
};

const REDRAW_INTERVAL_MS: u64 = 33;

struct UiRuntime {
    renderer: UiRenderer,
    state: TuiState,
    subscription: Subscription,
    input_events: EventStream,
    ticker: time::Interval,
}

impl UiRuntime {
    fn new(app: &AppContext) -> Result<Self> {
        let cwd = current_dir_for_banner();
        let git_branch = current_git_branch();
        let render_meta = RenderMeta::new(
            app.auth.model().to_string(),
            app.auth.reasoning_setting().to_string(),
            app.auth.mode_name().to_string(),
            cwd,
            git_branch,
        );
        Ok(Self {
            renderer: UiRenderer::new(render_meta)?,
            state: TuiState::new(),
            subscription: app.agent_bridge.subscribe(),
            input_events: EventStream::new(),
            ticker: create_ticker(),
        })
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
        .ok()
        .filter(|o| o.status.success())?;

    let branch = String::from_utf8(output.stdout).ok()?.trim().to_string();
    Some(branch).filter(|b| !b.is_empty() && b != "HEAD")
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
                if handle_main_input_event(app, session_state, ui, maybe_event).await? {
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
        Ok(event) => ui.state.handle_user_input(input::to_user_input(event)),
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
                if should_quit_during_active_turn(&mut ui.state, event_result) {
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

fn should_quit_during_active_turn(
    state: &mut TuiState,
    event_result: Result<CEvent, std::io::Error>,
) -> bool {
    match event_result {
        Ok(event) => input::is_quit_event(&event),
        Err(err) => {
            state.handle_agent_event(CoreEvent::Error(format!("Input error: {err}")));
            false
        }
    }
}
