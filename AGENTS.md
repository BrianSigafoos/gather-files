# AGENTS.md

## Project Overview

gather-files is a Rust CLI (`gf`) that gathers file contents (with README first), copies them to the clipboard, and reports stats. 

## Development Commands

```bash
cargo fmt
cargo test
cargo run -- <args>
```

## Formatting

Run `cargo fmt` after code changes.

## Testing

Add or update tests for every code change. Run `cargo test` before finishing.

## Rust Code Principles

- Keep functions small and focused; extract helpers instead of long flows.
- Prefer immutability and explicit clones.
- Add context to fallible operations with `anyhow::Context`; avoid `unwrap`/`expect` outside tests.
- Use clear naming and predictable control flow; return early on errors.
