---
name: recu-cli
description: Track recurring expenses (subscriptions, bills) with the `recu` CLI — add, edit, rename, remove, list and visualize charges, audit monthly/yearly spend, convert currencies, and import from bank exports, app exports, or screenshots. Edits are reversible via undo. Use whenever the user mentions recu, or asks to track, audit, import, clean up, or adjust subscriptions, bills, or recurring charges.
---

## Discovery

Don't guess flags. The CLI is self-documenting: `recu help` lists commands, `recu help <command>` gives exact flags, allowed values, and examples. Check it before building any command you're unsure about.

Globals: `--json` for machine-readable output, `-f/--file` (or `RECU_FILE`) to target a non-default CSV (default `recu.csv`).

Commands: `list` `add` `edit` `rename` `remove` `treemap` `calendar` `category {list,remove,rename}` `config {list,set}` `undo`.

## Bundled scripts

Paths are relative to the skill root; run with `python3`, don't rely on the executable bit. All are Python 3 stdlib only — no `pip install`. Prefer running them over generating equivalent code: they're tested and deterministic.

- `scripts/calendar_to_ics.py` — convert `recu calendar --json` (stdin) into an RFC 5545 `.ics`. Flags: `-o <file>` (else stdout). Stdout is the ICS; a `Wrote N event(s)...` summary goes to stderr. See the ICS recipe below.

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

`recu list` for the table — it already prints monthly/yearly totals and a per-category breakdown, in the configured display currency (`recu config list` to check). Use `recu list --json` for raw per-item fields, not totals (it has none, and skips currency conversion). `recu treemap`/`recu calendar` to visualize. Filter any of them with `--category`, `--min`, `--max`. Ended expenses are hidden unless `--all`; mark a stopped subscription with `--end <date>` instead of removing it to keep history.

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

## Recipe: monthly spending overview

Answer "what am I paying this month?" — total, what's already due vs. still upcoming, and the biggest hits.

1. `recu calendar` for the visual grid, or `recu calendar --json` for the numbers: `{ month, currency, total, paid, remaining, days: [{ date, total, charges: [{ id, name, amount }] }] }`. Each day carries its own `total` (sum of that day's charges). `--next`/`--month YYYY-MM` for other months; amounts are in the configured display currency (see the currency note in the audit recipe).
2. Summarize: `total` for the month, `paid` (charges dated on/before today) vs. `remaining`, and the few largest charges (use each day's `total` or the per-charge `amount`).

## Recipe: find cancellation candidates via audit

Surface subscriptions worth cutting or downgrading, ranked by annual savings. The CSV has **no usage data**, so never claim something is "unused", flag candidates by cost and redundancy, then let the user confirm what they actually use.

1. First `recu config list` for a display currency:
   - **Set** → the plain `recu list` table already converts every amount to it, normalizes to monthly, and prints per-category and grand totals. Read that instead of recomputing.
   - **Not set** → totals across mixed currencies are meaningless. Ask the user which currency to report in, then `recu config set currency <iso>` (confirm the command first — it's a write). Never guess exchange rates.
   - **`--json` is raw** regardless: amounts stay at their stored currency and `interval`, no conversion or totals.
2. `recu list --json` only when you need per-item fields. Amounts are raw at their `interval`; normalize to monthly to compare (weekly ×52/12, quarterly ÷3, yearly ÷12), or lean on `--min`/`--max` which already filter by monthly cost.
3. Flag candidates from signals the data _does_ support:
   - **Redundant** — several entries serving the same purpose.
   - **Expensive** — biggest monthly/annual hits; small per-charge yearly bills add up.
   - **Stale-looking** — old `start_date` on something the user may have forgotten.
4. Ask the user which flagged ones they still use — don't guess. Pair the question with each item's annualized cost so the trade-off is concrete.
5. Present a ranked table: name, monthly, **annual savings if cut**, and the reason flagged. Total the savings.
6. Apply only confirmed choices:
   - Cancelled → `recu edit <target> --end <today>` to stop it but keep history (prefer over `remove`).
   - Downgraded to a cheaper tier → `recu edit <target> -a <amount>`.
     Confirm the exact commands first, run them, then `recu list` to show the new total. Mention `recu undo` reverses the last change.

## Recipe: export the calendar to an .ics file

Turn upcoming charges into calendar events the user can import into Apple/Google/Outlook calendars. Run `scripts/calendar_to_ics.py` (above) — it emits valid RFC 5545 (all-day events, CRLF, proper escaping, stable UIDs for clean re-imports). Don't hand-write ICS.

```sh
recu calendar --json | python3 scripts/calendar_to_ics.py -o recu-2026-06.ics
```

1. Pick the month: `recu calendar --json` (current), `--next`, or `--month YYYY-MM`. JSON shape is `{ month, currency, total, paid, remaining, days: [{ date, total, charges: [{ id, name, amount }] }] }`.
2. Pipe it into the script with `-o <file>.ics` (omit `-o` to write to stdout). Default the filename to `recu-<month>.ics` using the `month` field (e.g. `recu-2026-06.ics`). Each charge becomes one all-day `VEVENT`.
3. Fallback only if Python is unavailable: read `scripts/calendar_to_ics.py` and port its logic (header, VEVENT shape, UID scheme, escaping, line folding) to Node/Bash so output stays in sync. Don't reconstruct the format from memory.
4. Read-only export — no CSV touched, no confirmation needed. Report the output path and event count (from stderr).
