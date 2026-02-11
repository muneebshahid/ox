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

use anyhow::Result;
use app_context::AppContext;
use std::io::{self, BufRead, Write};
use tokio::signal;

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let cli = cli::parse_args()?;
    if cli.list_sessions {
        return session::list_sessions();
    }
    let mut session_state = session::open_session(&cli.session_name)?;
    let app = AppContext::new();
    let agent_bridge =
        events::agent_bridge::AgentEventBridge::new(events::hub::EventHub::new(1024));
    let stdin = io::stdin();
    let reasoning = app.auth.reasoning_setting();
    eprintln!(
        "Auth mode: {} | model: {} | reasoning: {}",
        app.auth.mode_name(),
        app.auth.model(),
        reasoning
    );

    loop {
        print!("> ");
        io::stdout().flush()?;

        let mut input = String::new();
        stdin.lock().read_line(&mut input)?;
        let input = input.trim();

        if input == "exit" || input.is_empty() {
            break;
        }

        session_state.append(serde_json::json!({
            "role": "user",
            "content": input
        }))?;

        let session_id = session_state.session_name().to_string();
        let persist_start = session_state.history_len();

        tokio::select! {
            run_result = agent::run(&app, session_state.history_mut(), &session_id, &agent_bridge) => {
                if let Err(e) = run_result {
                    eprintln!("Error: {e}");
                }
            }
            result = signal::ctrl_c() => {
                if let Err(e) = result {
                    eprintln!("\nError waiting for Ctrl+C signal: {e}");
                } else {
                    eprintln!("\nInterrupted. Returning to prompt.");
                }
            }
        }

        if let Err(e) = session_state.persist_from(persist_start) {
            eprintln!(
                "Warning: failed to persist session entries for {}: {e}",
                session_state.session_name()
            );
        }
    }
    Ok(())
}
