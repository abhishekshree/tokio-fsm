# Repository Guidelines

## Project Structure & Module Organization
`tokio-fsm` is a Rust workspace with two main crates. Runtime types live in `src/` (`core.rs`, `lib.rs`), while the proc-macro implementation lives in `tokio-fsm-macros/src/` with parsing, validation, and code generation split by responsibility. Integration tests live in `tests/`. Compile-fail macro coverage uses `tokio-fsm-macros/tests/ui/` plus the `trybuild` harness in `tokio-fsm-macros/tests/trybuild.rs`. Benchmarks live in `benches/`, and runnable examples live in `examples/`, including the separate `examples/axum_fsm` crate.

## Build, Test, and Development Commands
Prefer the `justfile` for standard workflows:

- `just build` builds the workspace with all features.
- `just test` runs workspace tests.
- `just test-example` runs tests for `examples/axum_fsm`.
- `just lint` runs `clippy` with `-D warnings`.
- `just fmt` and `just fmt-check` apply or verify formatting.

The `justfile` uses `cargo +nightly`. Direct equivalents are still useful, for example `cargo test --workspace --all-features` or `cargo clippy --workspace --all-targets --all-features -- -D warnings`.

## Coding Style & Naming Conventions
This repo targets Rust 2024. Follow `rustfmt.toml`: grouped imports, crate-granularity import merging, wrapped doc comments, and Unix newlines. Use `snake_case` for modules, files, functions, and test helpers. Use `CamelCase` for types, enums, and generated FSM names such as `WorkerFsm` and `WorkerFsmState`. Keep module boundaries aligned with responsibility, especially in `tokio-fsm-macros/src/codegen/` and `tokio-fsm-macros/src/validation/`.

## Rust Implementation Standards
When writing or refactoring Rust code in this repository, prefer simple, idiomatic Rust over clever abstractions. Match the existing module boundaries, naming, visibility, and error handling style before introducing a new pattern.

- Use the type system to encode domain invariants when it removes real invalid states or clarifies core FSM logic.
- Prefer enums, newtypes, and smart constructors for meaningful domain concepts such as states, events, IDs, validated values, modes, and capabilities.
- Avoid typestate, generics, traits, macros, or phantom types unless they clearly simplify the model or prevent bugs that the current API can realistically allow.
- Prefer typed errors in library code. Avoid `anyhow` outside binaries, examples, or tests.
- Avoid `.unwrap()` and `.expect()` in library code unless the invariant is local, obvious, and cannot be expressed cleanly in the type system.
- Avoid unnecessary clones, allocations, trait objects, and string conversions. Borrow when ownership is not needed.
- Prefer exhaustive `match` statements for closed sets of states, events, and return kinds.
- Avoid boolean parameters when an enum would make the call site clearer.
- Keep functions small enough to read linearly and keep abstraction levels consistent within each function.
- Keep comments sparse. Add comments only for non-obvious invariants, safety reasoning, generated-code constraints, or behavior that is surprising from the code alone.
- Do not add comments that merely restate the code.
- Preserve public API compatibility unless the task explicitly allows breaking changes.

When explaining work, be concrete. State assumptions explicitly, avoid speculation, avoid em dashes, and do not over-explain obvious Rust basics.

## Testing Guidelines
Add or update integration tests in `tests/` for runtime behavior and lifecycle changes. Add UI cases in `tokio-fsm-macros/tests/ui/` for compile-time diagnostics, with matching `.stderr` files. Prefer descriptive test names beginning with `test_`. Before opening a PR, run workspace tests and the axum example tests.

## Commit & Pull Request Guidelines
Match the existing Conventional Commits style: `fix(macros): ...`, `docs(axum): ...`, `test: ...`, `chore: ...`. Keep subjects imperative and scoped when useful. PRs should summarize the behavior change, link the issue when applicable, and call out any docs, example, or feature-flag impact. Include command results for the checks you ran.
