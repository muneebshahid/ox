# Contributing to ox

Thanks for contributing.

## Before You Start

- Be respectful and collaborative (see `CODE_OF_CONDUCT.md`).
- For larger changes, open an issue first so we can align on architecture and scope.
- Keep module boundaries explicit and avoid broad cross-cutting mutations.

## Local Setup

```bash
git clone https://github.com/muneebshahid/ox.git
cd ox
cp .env.example .env
cargo build
```

## Development Workflow

1. Create a branch from `main`.
2. Make focused changes with clear ownership by module.
3. Run checks locally:

```bash
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets --all-features
```

4. Update docs when behavior, config, or APIs change.
5. Open a pull request with a clear problem statement, approach, and verification.

## Pull Request Guidelines

- Keep PRs small enough to review quickly.
- Explain tradeoffs and boundary decisions when touching architecture.
- Include tests for behavior changes and regressions.
- Avoid unrelated cleanup in the same PR.

## Commit Guidance

Use clear, imperative commit messages, for example:

- `tui: simplify viewport resize handling`
- `events: make hub dispatch non-blocking`

## Reporting Bugs

Use the bug report template and include:

- steps to reproduce
- expected vs actual behavior
- environment details (`rustc --version`, OS)
- relevant logs or screenshots

## Security Issues

Do not file public issues for security vulnerabilities.
Follow `SECURITY.md` for private reporting.
