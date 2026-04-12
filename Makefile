.PHONY: check format lint test all

check:
	cargo check

format:
	cargo fmt

lint:
	cargo clippy -- -D warnings

test:
	cargo test

all: format lint test
