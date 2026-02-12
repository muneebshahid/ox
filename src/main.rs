mod agent;
mod api;
mod app_context;
mod auth;
mod cli;
mod client;
mod events;
mod prompt;
mod session;
mod tools;
mod tui;

use anyhow::Result;
use app_context::AppContext;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let cli = cli::parse_args()?;
    if cli.list_sessions {
        return session::list_sessions();
    }
    let mut session_state = session::open_session(&cli.session_name)?;
    let app = AppContext::new();
    if is_debug_events_enabled() {
        spawn_event_debug_logger(&app);
    }
    eprintln!(
        "Auth mode: {} | model: {} | reasoning: {}",
        app.auth.mode_name(),
        app.auth.model(),
        app.auth.reasoning_setting(),
    );

    if let Err(e) = tui::run(&app, &mut session_state).await {
        eprintln!("Error: {e}");
    }

    Ok(())
}

fn is_debug_events_enabled() -> bool {
    std::env::var("OX_DEBUG_EVENTS").is_ok_and(|value| value != "0")
}

fn spawn_event_debug_logger(app: &AppContext) {
    let mut subscription = app.agent_bridge.subscribe();
    tokio::spawn(async move {
        loop {
            match subscription.recv().await {
                Ok(event) => eprintln!("[core-event] {event:?}"),
                Err(events::hub::RecvError::Lagged(dropped)) => {
                    eprintln!("[core-event] lagged, dropped {dropped} events");
                }
                Err(events::hub::RecvError::Closed) => break,
            }
        }
    });
}
