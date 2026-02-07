CARGO := "cargo +nightly"

default: fmt lint test

build:
    {{ CARGO }} build

test:
    {{ CARGO }} test

lint:
    {{ CARGO }} clippy -- -D warnings

fmt:
    {{ CARGO }} fmt

clean:
    {{ CARGO }} clean

check:
    {{ CARGO }} check

release:
    {{ CARGO }} publish --dry-run

doc:
    {{ CARGO }} doc --no-deps --open
