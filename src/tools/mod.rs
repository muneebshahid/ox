mod bash;
mod edit;
mod find;
mod grep;
mod ls;
mod read_file;
mod truncate;
mod write_file;

/// Register all tools in one place. Each entry maps a tool name to its module.
/// This ensures `definitions()` and `execute()` stay in sync automatically.
macro_rules! tools {
    ($($name:literal => $module:ident),+ $(,)?) => {
        pub fn definitions() -> Vec<serde_json::Value> {
            vec![$($module::definition()),+]
        }

        pub fn execute(name: &str, arguments: &str) -> String {
            let args: serde_json::Value = match serde_json::from_str(arguments) {
                Ok(v) => v,
                Err(e) => return format!("Error parsing arguments: {e}"),
            };

            match name {
                $($name => $module::run(&args),)+
                _ => format!("Unknown tool: {name}"),
            }
        }
    };
}

tools! {
    "read_file"  => read_file,
    "ls"         => ls,
    "write_file" => write_file,
    "edit"       => edit,
    "grep"       => grep,
    "find"       => find,
    "bash"       => bash,
}
