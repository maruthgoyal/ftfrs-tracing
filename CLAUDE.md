# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands
- Build: `cargo build`
- Run example: `cargo run --example run`
- Test: `cargo test`
- Test single test: `cargo test test_name`
- Format code: `cargo fmt`
- Lint: `cargo clippy`

## Code Style Guidelines
- **Formatting**: Follow Rust conventions with 4-space indentation
- **Imports**: Group standard library, then external crates, then local modules
- **Naming**: Use snake_case for variables/functions, CamelCase for types/traits
- **Types**: Prefer explicit type annotations for public interfaces
- **Error Handling**: Use Result<T, E> for recoverable errors
- **Unwrapping**: Avoid unwrap() in production code; handle errors explicitly
- **Documentation**: Document public APIs with /// comments
- **Testing**: Write unit tests in the same file as the code being tested

This is a Rust project for ftfrs-tracing, a library integrating the ftfrs crate with Rust's tracing ecosystem.