.PHONY: check format lint test all npm-build-darwin-arm64 npm-publish-darwin-arm64 npm-publish-main

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

# --- npm packaging ---------------------------------------------------------
# Versions in npm/recu/package.json and npm/recu-*/package.json must match
# the Cargo.toml version. Bump them together.

npm-build-darwin-arm64:
	MACOSX_DEPLOYMENT_TARGET=11.0 $(CARGO) build --release --target aarch64-apple-darwin
	mkdir -p npm/recu-darwin-arm64/bin
	cp target/aarch64-apple-darwin/release/recu npm/recu-darwin-arm64/bin/recu
	chmod +x npm/recu-darwin-arm64/bin/recu

npm-publish-darwin-arm64: npm-build-darwin-arm64
	cd npm/recu-darwin-arm64 && npm publish --access=public

npm-publish-main:
	cd npm/recu && npm publish --access=public
