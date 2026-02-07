# ox

A minimal CLI coding agent in Rust. Connects to OpenAI's Responses API with streaming, provides an interactive REPL, and executes tools autonomously in an agent loop.

## Setup

```bash
cp .env.example .env
```

Choose one auth mode via `AUTH_MODE`:

### API key mode (`AUTH_MODE=api`)

Use the OpenAI API directly (`https://api.openai.com/v1/responses`):

```text
AUTH_MODE=api
OPENAI_API_KEY=sk-...
# optional override; default in api mode is gpt-4.1-mini
OPENAI_MODEL=gpt-4.1-mini
```

### Subscription mode (`AUTH_MODE=subscription`)

Use your Codex/ChatGPT auth tokens (`https://chatgpt.com/backend-api/codex/responses`):

```bash
codex login
```

```text
AUTH_MODE=subscription
# optional override; default in subscription mode is gpt-5.3-codex
OPENAI_MODEL=gpt-5.3-codex
```

Notes:

- Token file is loaded from `CODEX_HOME/auth.json` if `CODEX_HOME` is set.
- Otherwise it loads from `~/.codex/auth.json`.
- If the subscription access token is expired, ox refreshes it and writes updated tokens back to the same file.

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

CLI flags:

```text
ox [--session <name>] [--list-sessions]
```

## Tools

| Tool         | Description                                                 |
| ------------ | ----------------------------------------------------------- |
| `read_file`  | Read file contents                                          |
| `write_file` | Create or overwrite files                                   |
| `edit`       | Search-and-replace edit (old_text must be unique)           |
| `ls`         | List directory contents                                     |
| `grep`       | Search file contents with `rg` (falls back to `grep`)       |
| `find`       | Find files by glob pattern with `fd` (falls back to `find`) |
| `bash`       | Execute shell commands                                      |

## Development

```bash
cargo build
make lint      # clippy with pedantic + nursery
cargo test
```
