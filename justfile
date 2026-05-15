CARGO := "cargo +nightly"

default: fmt-check lint lint-example test test-example

build:
    {{ CARGO }} build --workspace --all-features

test:
    {{ CARGO }} test --workspace --all-features

test-example:
    {{ CARGO }} test --manifest-path examples/axum_fsm/Cargo.toml

lint:
    {{ CARGO }} clippy --workspace --all-targets --all-features -- -D warnings

lint-example:
    {{ CARGO }} clippy --manifest-path examples/axum_fsm/Cargo.toml --all-targets --all-features -- -D warnings

lint-all: lint lint-example

fmt:
    {{ CARGO }} fmt --all

fmt-check:
    {{ CARGO }} fmt --all -- --check

clean:
    {{ CARGO }} clean

check:
    {{ CARGO }} check --workspace --all-features

release LEVEL="patch":
    {{ CARGO }} release --workspace --execute {{ LEVEL }}

release-dry-run LEVEL="patch":
    {{ CARGO }} release --workspace {{ LEVEL }}

doc:
    {{ CARGO }} doc --no-deps --open

run-axum:
    RUSTFLAGS="--cfg tokio_unstable" {{ CARGO }} run --manifest-path examples/axum_fsm/Cargo.toml
