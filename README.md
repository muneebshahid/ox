# ox

A minimal CLI coding agent in Rust. Connects to OpenAI's Responses API with streaming, provides an interactive REPL, and executes tools autonomously in an agent loop.

## Setup

```bash
cp .env.example .env
# Add your OpenAI API key to .env
```

```
OPENAI_API_KEY=sk-...
```

## Usage

```bash
cargo run
```

```
> read src/main.rs and explain what it does
Calling read_file...
The main entry point sets up a REPL loop that...

> find all rust files and count the lines
Calling find...
Calling bash...
There are 250 lines across 9 Rust files.

> exit
```

## Tools

| Tool | Description |
|------|-------------|
| `read_file` | Read file contents |
| `write_file` | Create or overwrite files |
| `edit` | Search-and-replace edit (old_text must be unique) |
| `ls` | List directory contents |
| `grep` | Search file contents with `rg` (falls back to `grep`) |
| `find` | Find files by glob pattern with `fd` (falls back to `find`) |
| `bash` | Execute shell commands |

## Architecture

```
src/
  main.rs       REPL loop — reads input, manages history
  agent.rs      Agent loop — streams API response, dispatches tool calls
  api.rs        OpenAI HTTP client — sends requests, returns raw response
  prompt.rs     System prompt builder
  tools/
    mod.rs      Tool definitions and dispatch
    read_file.rs
    write_file.rs
    edit.rs
    bash.rs
    grep.rs
    find.rs
    ls.rs
```

**Flow:** User input -> history -> OpenAI Responses API (streaming) -> SSE events -> print text deltas live -> on tool call, execute and loop -> on message, push to history and wait for next input.

## Development

```bash
cargo build
make lint      # clippy with pedantic + nursery
```
