#!/usr/bin/env bash
set -euo pipefail

cargo clean && cargo build --release 2>&1 | grep -E "^error" && exit 1 || true

E=examples/recu.csv
BIN=./target/release/recu

echo "=== --help ===" && RECU_FILE=$E $BIN --help
echo "=== ls ===" && RECU_FILE=$E $BIN ls
echo "=== timeline ===" && RECU_FILE=$E $BIN timeline
echo "=== treemap ===" && RECU_FILE=$E $BIN treemap
echo "=== help add ===" && RECU_FILE=$E $BIN help add
echo "=== help category ===" && RECU_FILE=$E $BIN help category
