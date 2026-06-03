# xtask

`xtask` provides repository-local developer checks for the standalone
`cli-release` workspace.

## Responsibilities

- Run the required formatting, tests, clippy, and file-length checks.
- Provide a `review` entrypoint that sends committed, staged, unstaged, and
  untracked local change context to a read-only nested Codex reviewer.

## What This Crate Does

The crate is a small binary task runner invoked through Cargo's alias support.
It intentionally stays focused on this repository instead of carrying
application-specific checks from the source Juno workspace.

## Quick Start

```bash
cargo xtask check
cargo xtask review
```

## Development

```bash
cargo test -p xtask
cargo clippy -p xtask --all-targets --all-features -- -D warnings
```

### Key Code

- `src/main.rs` - command dispatch and subprocess execution.

### Related Docs

- [`../plans/README.md`](../plans/README.md)
