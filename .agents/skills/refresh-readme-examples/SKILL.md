---
name: refresh-readme-examples
description: Use when asked to update README examples, refresh command output in docs, or sync readme with current CLI output for recu.
version: 1.1.0
---

# Refresh README Examples

Run `capture.sh`, then update `README.md` with verbatim output.

## Steps

1. Run: `bash .agents/skills/refresh-readme-examples/capture.sh` — does a clean release build and prints all command outputs. Abort if it exits non-zero.
2. Read `README.md`.
3. Update sections (replace verbatim, preserve all surrounding content):
   - **Usage**: `--help` output in a fenced block prefixed `$ recu --help`
   - **Example**: one fenced block per command — `help add`, `help edit`, `help rm`, `ls`, `timeline`, `treemap` — each prefixed `$ recu <cmd>`
