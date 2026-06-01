---
name: recu-cli
description: Track recurring expenses (subscriptions, bills) with the `recu` CLI — add, edit, rename, remove, list and visualize charges, audit monthly/yearly spend, convert currencies, and import from bank exports, app exports, or screenshots. Edits are reversible via undo. Use whenever the user mentions recu, or asks to track, audit, import, clean up, or adjust subscriptions, bills, or recurring charges.
---

## Discovery

Don't guess flags. The CLI is self-documenting: `recu help` lists commands, `recu help <command>` gives exact flags, allowed values, and examples. Check it before building any command you're unsure about.

Globals: `--json` for machine-readable output, `-f/--file` (or `RECU_FILE`) to target a non-default CSV (default `recu.csv`).

Commands: `list` `add` `edit` `rename` `remove` `treemap` `calendar` `category {list,remove,rename}` `config {list,set}` `undo`.

## Safety rules

- **Confirm before writing.** `add`, `edit`, `rename`, `remove`, `category remove/rename`, `config set` mutate the CSV. Show the exact command(s) and get approval first, so the user catches wrong targets or amounts before they land. Read-only commands (`list`, `treemap`, `calendar`, `category list`, `config list`) need no confirmation.
- **Always pass explicit flags.** `recu add` with no name opens an interactive prompt that will hang a non-interactive run — never invoke the bare form.
- **`recu undo`** reverses the last add/edit/rename/remove — mention it after mutations.

## Mutating an expense

Indices resolve against the _current_ list order, so `recu list` (or `--json`) first to get the live `@id` or name, then act. Targets accept `@id` or name (case-insensitive) and batch with commas; prefer `@id` when touching several.

- Edit: `recu edit @1 -a 12.99` — pass only the fields that change.
- Rename: `recu rename @1 "New Name"`.
- Remove: `recu remove @2` or `recu remove @1,@3,Spotify`.

## Reading & auditing spend

`recu list` for the table, `recu list --json` to compute totals, `recu treemap`/`recu calendar` to visualize. Filter any of them with `--category`, `--min`, `--max`. Ended expenses are hidden unless `--all`; mark a stopped subscription with `--end <date>` instead of removing it to keep history.

## Recipe: import from an external source

Source may be a bank statement, an app export (CSV/JSON), or a screenshot.

1. `recu help add` for the exact fields (name, amount, currency, start date, interval, category, end).
2. Parse the file directly, or read the image and extract each recurring charge.
3. Map fields onto recu's: infer `interval` from cadence, normalize currency to ISO 4217, skip one-off transactions.
4. Show the full parsed list as a table, flag anything ambiguous or missing, and get approval.
5. Run one `recu add ...` per expense, then `recu list` to show the result.

## Recipe: refresh prices against market rates

1. `recu list --json` for current amounts and currencies.
2. Web-search each subscription's current published price (match plan/tier and currency).
3. Summarize mismatches: name, stored amount, current price, source.
4. Present proposed changes; let the user pick which to apply — never edit unprompted.
5. Run `recu edit <target> -a <amount>` per confirmed change, then `recu list`.
