# recu

CLI tool for tracking recurring expenses. Built with Rust (edition 2024).

## Key files

- `src/main.rs` - entry point
- `src/cli.rs` - CLI definition and `run()`
- `src/commands/` - subcommand implementations (ls, add, rm, edit, treemap, config)
- `src/expense.rs` - expense data model
- `src/exchange.rs` - currency exchange
- `src/storage.rs` - data persistence

## After coding changes

Run `make all` (formats, lints, tests).
