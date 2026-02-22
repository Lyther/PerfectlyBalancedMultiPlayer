.PHONY: dev build test lint fmt clean

dev:
	cargo run

build:
	cargo build --release

test:
	cargo test

lint:
	cargo fmt --check
	cargo clippy -- -D warnings

fmt:
	cargo fmt

clean:
	cargo clean
