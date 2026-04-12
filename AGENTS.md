# recu

CLI tool for tracking recurring expenses. Built with Rust (edition 2024).

## Key files

- `src/main.rs` - CLI entry point, subcommands (ls, add, rm, edit)
- `src/add.rs` - add expense logic
- `src/rm.rs` - remove expense logic
- `src/edit.rs` - edit expense logic
- `src/ls.rs` - list expenses logic
- `src/expense.rs` - expense data model
- `src/storage.rs` - data persistence

## After coding changes

Run `make all` (formats, lints, tests).
