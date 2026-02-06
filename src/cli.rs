use anyhow::Result;

pub struct CliArgs {
    pub session_name: String,
    pub list_sessions: bool,
}

fn print_usage() {
    println!("Usage: ox [--session <name>] [--list-sessions]");
}

pub fn parse_args() -> Result<CliArgs> {
    let mut args = std::env::args().skip(1);
    let mut session_name: Option<String> = None;
    let mut list_sessions = false;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--session" => {
                let Some(name) = args.next() else {
                    anyhow::bail!("missing value for --session");
                };
                session_name = Some(name);
            }
            "--list-sessions" => {
                list_sessions = true;
            }
            "--help" | "-h" => {
                print_usage();
                std::process::exit(0);
            }
            _ => anyhow::bail!("unknown argument: {arg}"),
        }
    }

    Ok(CliArgs {
        session_name: session_name.unwrap_or_else(crate::session::create_session_name),
        list_sessions,
    })
}
