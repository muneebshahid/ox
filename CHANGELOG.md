# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.1] - 2026-02-15

### Added

- Open-source governance baseline (`LICENSE`, `CONTRIBUTING`, `CODE_OF_CONDUCT`, `SECURITY`)
- CI workflow and dependency update automation
- Issue and pull request templates
- Project metadata and toolchain pinning
- Release automation for Homebrew tap updates on tag push

### Changed

- `grep` tool now requires `ripgrep (rg)` explicitly (no partial `grep` fallback)
- CI now installs `ripgrep` before running checks
- Dependency updates: `anyhow` 1.0.101, `crossterm` 0.29.0, `ratatui` 0.30.0, `reqwest` 0.13.2, `tempfile` 3.25.0
