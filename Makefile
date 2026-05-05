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
# Version is read from Cargo.toml by scripts/build-npm.mjs. Platform package
# directories under npm/recu-*/ are generated and gitignored — bump only
# Cargo.toml.

npm-build-darwin-arm64:
	MACOSX_DEPLOYMENT_TARGET=11.0 $(CARGO) build --release --target aarch64-apple-darwin
	node scripts/build-npm.mjs darwin-arm64

npm-publish-darwin-arm64: npm-build-darwin-arm64
	cd npm/recu-darwin-arm64 && npm publish --access=public

npm-publish-main:
	node scripts/build-npm.mjs --sync-main
	cd npm/recu && npm publish --access=public
