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

release LEVEL="patch":
    {{ CARGO }} release --workspace --execute {{ LEVEL }}

release-dry-run LEVEL="patch":
    {{ CARGO }} release --workspace {{ LEVEL }}

doc:
    {{ CARGO }} doc --no-deps --open
