# recu

CLI tool for tracking recurring expenses. Built with Rust (edition 2024).

## Key files

- `src/main.rs` - entry point
- `src/cli.rs` - CLI definition and `run()`
- `src/commands/` - subcommand implementations (ls, add, rm, edit, treemap, config, category, timeline, undo)
- `src/prompt.rs` - interactive prompts (inquire wrappers)
- `src/ui.rs` - terminal UI primitives (text layout, day humanization, semantic styling wrapping `colored`)
- `src/expense.rs` - expense data model (`Expense`, `Interval`, `ExpenseInput`)
- `src/config.rs` - config model and load/save logic
- `src/rates.rs` - currency exchange rates (fetch, cache, convert)
- `src/store.rs` - data persistence

## Module conventions

Each module wraps one concern so commands speak domain vocabulary, not library APIs:

- `prompt` is the only place that imports `inquire`.
- `store` is the only place that imports `csv` and touches the data file.
- `ui` is the only place that imports `colored`. Commands use `ui::dim`, `ui::heading`, `ui::due`, etc. instead of `.red()` / `.bold()` / `.dimmed()`.
- `rates` is the only place that performs network I/O or touches the rates cache.

When adding a new cross-cutting helper, extend the matching module rather than importing the underlying library directly in a command.

## After coding changes

Run `make all` (formats, lints, tests).
