.PHONY: check format lint test all

CARGO ?= rtk cargo

check:
	$(CARGO) check
format:
	$(CARGO) fmt

lint:
	$(CARGO) clippy --all-targets -- -D warnings

test:
	$(CARGO) test

all: format lint test
