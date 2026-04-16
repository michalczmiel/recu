---
name: refresh-readme-examples
description: Use when asked to update README examples, refresh command output in docs, or sync readme with current CLI output for recu.
version: 1.0.0
---

# Refresh README Examples

Run `--help`, `help add`, `ls`, `timeline`, and `treemap` against `examples/recu.csv`, then update `README.md` with their verbatim output.

## Steps

1. Run all commands, capturing stdout:
   ```
   RECU_FILE=examples/recu.csv cargo run -- --help
   RECU_FILE=examples/recu.csv cargo run -- help add
   RECU_FILE=examples/recu.csv cargo run -- ls
   RECU_FILE=examples/recu.csv cargo run -- timeline
   RECU_FILE=examples/recu.csv cargo run -- treemap
   ```
2. Read `README.md`.
3. Update sections:
   - **Usage**: replace command table and description with `--help` output (fenced block, prefixed `$ recu --help`)
   - **Example**: replace fenced blocks with `help add`, `ls`, `timeline`, `treemap` output — one block each, prefixed `$ recu <cmd>`
4. Preserve all other README content unchanged.
