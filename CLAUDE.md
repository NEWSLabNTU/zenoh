# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build/Test Commands
- Build: `cargo build --profile fast --all-targets`
- Run tests: `cargo nextest run --no-fail-fast`
- Run a single test: `cargo test --profile fast -- test_name --nocapture`
- Doc tests: `cargo test --profile fast --doc`
- Formatting check: `cargo +nightly fmt --check`
- Linting: `cargo clippy --all-targets --features test -- --deny warnings`

## Code Style Guidelines
- Rust edition: 2021
- Use try shorthand (`?`) and field init shorthand when possible
- Imports: Organize by crate, prefer explicit imports over glob imports
- Error handling: Use `zenoh-result` for error types, propagate errors with `?`
- Naming: Follow Rust conventions (snake_case for functions/variables, CamelCase for types)
- Types: Prefer strong typing, use Rust's type system for safety
- Documentation: All public APIs must be documented with doc comments
- Tests: Place in module's `tests/` directory, one file per feature
- Features: Use feature flags for optional functionality
- Multiplatform: Ensure code works on all supported platforms (Linux, macOS, Windows)
- If the work on Rust code is not done yet, leave a todo!() and TODO comments. Avoid using dummy values or any form of silent errors.
- Follow Rust naming conventions. For example, id() is preferred over get_id().
- In Rust, it's preferred to initialize struct fields first and then construct the struct. It avoids mutable initial structs.
- Always build and test the Rust code whenever modification work is made on Rust.

## Workflow Tips
- When tasks are completed, notify GNU Screen with a bell: `printf '\a'; echo "[Task Complete] <task description>"`.