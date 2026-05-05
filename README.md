# recu

CLI tool for tracking recurring expenses. Parses expenses data and outputs it in one of three formats (list, timeline or treemap). Data is stored in a single CSV file

> This project is still in development, the interfaces and features may change.

## Install

Via npm (currently only macOS):

```
npm install -g @michalczmiel/recu
```

Via cargo (any platform):

```
cargo install recu
```

## Usage

```
$ recu --help
Track recurring expenses

Usage: recu [OPTIONS] [COMMAND]

Commands:
  ls        List recurring expenses. Amounts converted to display currency when configured
  add       Add a recurring expense
  edit      Edit a recurring expense
  rename    Rename a recurring expense
  rm        Remove one or more recurring expenses
  treemap   Visualize expenses as a treemap
  config    Manage configuration
  category  Manage expense categories
  timeline  Show expenses as a timeline. Supports past and future date ranges
  undo      Undo the last add, edit, rename, or rm
  help      Print this message or the help of the given subcommand(s)

Options:
  -f, --file <FILE>  Path to the CSV storage file [env: RECU_FILE=examples/recu.csv] [default: recu.csv]
  -a, --all          Include ended expenses (only used when no subcommand is given; equivalent to `recu ls --all`)
  -h, --help         Print help
  -V, --version      Print version
```

## Tips

### Version your data

Keep `recu.csv` in a git repo for free history and diffs. A dedicated folder like `~/.finances` works well. Add a `.gitignore` to exclude the working files recu creates alongside the CSV:

```gitignore
*.undo
*.seq
```

### Set a default file

Point `RECU_FILE` at your main file in `~/.bashrc` or `~/.zshrc`:

```
export RECU_FILE=~/.finances/recu.csv
```

### Multiple accounts via multiple files

`recu` is just one CSV per dataset — keep separate files for personal, business, shared household etc. Add bash aliases pointing each to its own file:

```
alias recu-biz='recu -f ~/.finances/biz.csv'
alias recu-home='recu -f ~/.finances/home.csv'
```

### Set a display currency

Set a default currency and `recu` auto-converts multi-currency entries to it on display:

```
recu config set currency pln
```

### End-date instead of removing

When a subscription stops, set `--end` instead of `rm` to keep it in history. `recu ls --all` shows ended ones.

### Let an LLM agent do the grunt work

Point any coding agent (Pi, OpenCode, Claude Code, Codex etc.) at your shell and ask it to "import my subscriptions into recu, suggest categories, and find overlapping subscriptions". It can discover the interface via `recu help` and each subcommand's `--help`.

## Example

```
$ recu ls
@    name                  amount  due          category
───  ────────────────────  ──────  ───────────  ──────────────
@9   0Password              $4.99  today        Security
@4   Pear TV+              $12.99  in 2 days    Streaming
@3   ViewTube Premium      $15.99  in 3 days    Streaming
@16  Gym                   $45.00  in 3 days    Health
@8   Potion                $10.00  in 5 days    Productivity
@11  GitHug Pro             $4.00  in 1 week    Dev
@12  ChatGBT Plus          $20.00  in 1 week    Dev
@15  Amazoo Prime          $14.99  in 2 weeks   Shopping
@13  iSmog+                 $2.99  in 2 weeks   Cloud
@2   Spookify               $9.99  in 2 weeks   Streaming
@14  Goggle One             $2.99  in 3 weeks   Cloud
@6   Adobo Creative Cloud  $54.99  in 3 weeks   Productivity
@1   Streamberry           $19.99  in 3 weeks   Streaming
@18  Web Hosting            $6.99  in 3 weeks   Infrastructure
@5   Ghibli+               $11.99  in 3 weeks   Streaming
@17  Domain                $15.00  in 6 months  Infrastructure
@7   Macrosoft 365         $99.99  in 7 months  Productivity

17 expenses  896.15 zł/month  10753.83 zł/year
+ 1 ended (recu ls --all)
```

```
$ recu timeline
date      name                  amount
────────  ────────────────────  ──────
Apr 2026
      28  0Password              $4.99
      30  Pear TV+              $12.99
May 2026
       1  ViewTube Premium      $15.99
       1  Gym                   $45.00
       3  Potion                $10.00
       7  GitHug Pro             $4.00
      10  ChatGBT Plus          $20.00
      15  Amazoo Prime          $14.99
      17  iSmog+                 $2.99
      18  Spookify               $9.99
      19  Goggle One             $2.99
      20  Adobo Creative Cloud  $54.99
      22  Streamberry           $19.99
      24  Web Hosting            $6.99
      25  Ghibli+               $11.99
      28  0Password              $4.99
Total  879.52 zł
```

```
$ recu treemap
┌──────────────────────────────┐┌───────────┐┌────────────────┐┌───────────────┐
│Adobo Creative Cloud          ││ChatGBT Pl…││ViewTube Premium││Amazoo Prime   │
│199 zł/mo                     ││72 zł/mo   ││58 zł/mo        ││54 zł/mo       │
│2390 zł/yr                    ││869 zł/yr  ││695 zł/yr       ││651 zł/yr      │
│                              ││           ││                ││               │
│                              ││           │└────────────────┘└───────────────┘
│                              ││           │┌─────────┐ ┌──────────┌──────────┐
│                              ││           ││Pear TV+ │ │Potion    │Spookify  │
│                              ││           ││47 zł/mo │ │36 zł/mo  │36 zł/mo  │
│                              │└───────────┘│564 zł/yr│ │435 zł/yr │434 zł/yr │
└──────────────────────────────┘┌───────────┐│         │ │          │          │
┌──────────────────────────────┐│Streamberry││         │ ┌─────────┐┌─────┐┌───┐
│Gym                           ││72 zł/mo   │└─────────┘ │Macrosof…││0Pas…││Gi…│
│163 zł/mo                     ││869 zł/yr  │┌─────────┐ │30 zł/mo ││18 z…││   │
│1955 zł/yr                    ││           ││Ghibli+  │ │362 zł/yr││217 …││   │
│                              ││           ││43 zł/mo │ └─────────┘└─────┘└───┘
│                              ││           ││521 zł/yr│ ┌─────────┐┌───┐┌─────┐
│                              ││           ││         │ │Web Host…││iS…││Gogg…│
│                              ││           ││         │ │25 zł/mo ││   │└─────┘
└──────────────────────────────┘└───────────┘└─────────┘ └─────────┘└───┘
```

```
$ recu help add
Add a recurring expense

Usage: recu add [OPTIONS]

Options:
  -f, --file <FILE>          Path to the CSV storage file [env: RECU_FILE=examples/recu.csv] [default: recu.csv]
  -n, --name <NAME>          Expense name
  -a, --amount <AMOUNT>      Amount (e.g. 9.99 or 9,99)
  -c, --currency <CURRENCY>  ISO 4217 currency code (e.g. usd, eur)
  -d, --date <DATE>          Start date (YYYY-MM-DD)
  -i, --interval <INTERVAL>  Billing interval [possible values: weekly, monthly, quarterly, yearly]
      --category <CATEGORY>  Category label (e.g. streaming, utilities)
      --end <END_DATE>       End date — when the subscription stops (YYYY-MM-DD)
  -h, --help                 Print help

Examples:
  recu add -n Netflix -a 9.99 -c usd -d 2026-05-01 -i monthly
  recu add -n Netflix          # stored with name only, fill in later via 'recu edit'
  recu add          # interactive mode
```

```
$ recu help category
Manage expense categories

Usage: recu category [OPTIONS] <COMMAND>

Commands:
  list    List categories currently used by expenses
  rm      Remove categories from all matching expenses
  rename  Rename one or more categories into a destination (merges if dst already exists)
  help    Print this message or the help of the given subcommand(s)

Options:
  -f, --file <FILE>  Path to the CSV storage file [env: RECU_FILE=examples/recu.csv] [default: recu.csv]
  -h, --help         Print help

Examples:
  recu category list
  recu category rm streaming
  recu category rename streaming Streaming
  recu category rename streaming,subs Streaming
```
