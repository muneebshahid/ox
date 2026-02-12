# Agent Guidelines

## Goals

- Keep module boundaries clear.
- Minimize public APIs.
- Prefer architecture-first changes over ad-hoc implementation.

## Architecture-First Workflow

Before implementing a new major component (for example TUI, agent services, event systems):

1. Propose the data model first (structs/enums and key fields).
2. Propose responsibility split (what each module owns and does not own).
3. Propose external API surface (what functions are public vs private).
4. Propose event/control flow at a high level.
5. Confirm the design with the user before coding.

## Module Interface Rules

- Each major module should expose a minimal, intentional API.
- Prefer a single entrypoint for top-level workflows (for example `run`) unless there is a strong reason not to.
- Keep internal helpers private by default.
- Do not leak implementation-specific types across boundaries when an internal enum/model is clearer.

## Implementation Rules

- Start with a thin skeleton that matches approved architecture.
- Add behavior incrementally in small steps.
- Re-check boundaries before adding new fields/functions.
- If a change introduces cross-cutting mutations, pause and revisit ownership.

## Review Checklist (Before Finalizing)

1. Is the public API minimal and clear?
2. Are responsibilities clearly separated?
3. Is there one obvious place for each mutation?
4. Are module names and file names aligned with responsibilities?
