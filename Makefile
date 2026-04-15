.PHONY: check format lint test all

check:
	rtk cargo check

format:
	rtk cargo fmt

lint:
	rtk cargo clippy -- -D warnings

test:
	rtk cargo test

all: format lint test
