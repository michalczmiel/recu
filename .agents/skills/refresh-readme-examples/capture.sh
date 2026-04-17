#!/usr/bin/env bash
set -euo pipefail

cargo clean && cargo build --release 2>&1 | grep -E "^error" && exit 1 || true

E=examples/recu.csv
BIN=./target/release/recu

echo "=== --help ===" && RECU_FILE=$E $BIN --help
echo "=== help add ===" && RECU_FILE=$E $BIN help add
echo "=== help edit ===" && RECU_FILE=$E $BIN help edit
echo "=== help rm ===" && RECU_FILE=$E $BIN help rm
echo "=== ls ===" && RECU_FILE=$E $BIN ls
echo "=== timeline ===" && RECU_FILE=$E $BIN timeline
echo "=== treemap ===" && RECU_FILE=$E $BIN treemap
