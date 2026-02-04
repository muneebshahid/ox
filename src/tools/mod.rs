mod bash;
mod edit;
mod find;
mod grep;
mod ls;
mod read_file;
mod truncate;
mod write_file;

pub fn definitions() -> Vec<serde_json::Value> {
    vec![
        read_file::definition(),
        ls::definition(),
        write_file::definition(),
        edit::definition(),
        grep::definition(),
        find::definition(),
        bash::definition(),
    ]
}

pub fn execute(name: &str, arguments: &str) -> String {
    let args: serde_json::Value = match serde_json::from_str(arguments) {
        Ok(v) => v,
        Err(e) => return format!("Error parsing arguments: {e}"),
    };

    match name {
        "read_file" => read_file::run(&args),
        "ls" => ls::run(&args),
        "write_file" => write_file::run(&args),
        "edit" => edit::run(&args),
        "grep" => grep::run(&args),
        "find" => find::run(&args),
        "bash" => bash::run(&args),
        _ => format!("Unknown tool: {name}"),
    }
}
