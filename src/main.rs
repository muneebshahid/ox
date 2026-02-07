mod agent;
mod api;
mod app_context;
mod auth;
mod cli;
mod prompt;
mod session;
mod tools;

use app_context::AppContext;
use anyhow::Result;
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
    let stdin = io::stdin();
    eprintln!("Auth mode: {} | model: {}", app.auth.mode_name(), app.auth.model());

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

        let persist_start = session_state.history_len();

        tokio::select! {
            run_result = agent::run(&app, session_state.history_mut()) => {
                if let Err(e) = run_result {
                    eprintln!("Error: {e}");
                }
            }
            signal_result = signal::ctrl_c() => {
                match signal_result {
                    Ok(()) => {
                        eprintln!("\nInterrupted. Returning to prompt.");
                    }
                    Err(e) => {
                        eprintln!("\nError waiting for Ctrl+C signal: {e}");
                    }
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
