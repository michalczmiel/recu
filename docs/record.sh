#!/usr/bin/env bash
# Render docs/demo.gif from docs/demo.tape with the freshly built recu binary.
set -euo pipefail

cd "$(dirname "$0")/.."

cargo build
PATH="$PWD/target/debug:$PATH" vhs docs/demo.tape
