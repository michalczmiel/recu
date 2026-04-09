# recu

CLI tool for tracking recurring expenses. Built with Rust (edition 2024).

## Key files

- `src/main.rs` - CLI entry point, subcommands (ls, add, rm)
- `src/add.rs` - add expense logic
- `src/rm.rs` - remove expense logic
- `src/storage.rs` - data persistence

## Storage

Expenses stored as markdown files with YAML frontmatter in `~/.cache/recu/`. Filenames are slugified expense names (e.g. `netflix.md`).
