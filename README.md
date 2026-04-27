# recu

CLI tool for tracking recurring expenses. Parses expenses data and outputs it in one of three formats (list, timeline or treemap). Data is stored in a single CSV file

> This project is still in development, the interfaces and features may change.

## Install

Via npm (macOS arm64 prebuilt binary):

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
  rm        Remove one or more recurring expenses
  treemap   Visualize expenses as a treemap
  config    Manage configuration
  category  Manage expense categories
  timeline  Show expenses as a timeline. Supports past and future date ranges
  undo      Undo the last add, edit, or rm
  help      Print this message or the help of the given subcommand(s)

Options:
  -f, --file <FILE>  Path to the CSV storage file [env: RECU_FILE=examples/recu.csv] [default: recu.csv]
  -h, --help         Print help
  -V, --version      Print version
```

## Tips

### Version your data

Keep `recu.csv` in a git repo for free history and diffs. An example is a dedicated folder like `~/.finances` pointed to via `RECU_FILE` in your `~/.bashrc` or `~/.zshrc`:

```
export RECU_FILE=~/.finances/recu.csv
```

### Set a display currency

Set a default currency and `recu` auto-converts multi-currency entries to it on display:

```
recu config set currency usd
```

### Let an LLM agent do the grunt work

Point any coding agent (Pi, OpenCode, Claude Code, Codex etc.) at your shell and ask it to "import my subscriptions into recu, suggest categories, and find overlapping subscriptions". It can discover the interface via `recu help` and each subcommand's `--help`.

## Example

```
$ recu ls
@    name                  amount  due          category
───  ────────────────────  ──────  ───────────  ──────────────
@1   Streamberry           $19.99  in 1 day     Streaming
@18  Web Hosting            $6.99  in 3 days    Infrastructure
@5   Ghibli+               $11.99  in 4 days    Streaming
@9   0Password              $4.99  in 1 week    Security
@4   Pear TV+              $12.99  in 1 week    Streaming
@3   ViewTube Premium      $15.99  in 1 week    Streaming
@16  Gym                   $45.00  in 1 week    Health
@8   Potion                $10.00  in 1 week    Productivity
@11  GitHug Pro             $4.00  in 2 weeks   Dev
@12  ChatGBT Plus          $20.00  in 2 weeks   Dev
@15  Amazoo Prime          $14.99  in 3 weeks   Shopping
@13  iSmog+                 $2.99  in 3 weeks   Cloud
@2   Spookify               $9.99  in 3 weeks   Streaming
@14  Goggle One             $2.99  in 4 weeks   Cloud
@6   Adobo Creative Cloud  $54.99  in 4 weeks   Productivity
@17  Domain                $15.00  in 6 months  Infrastructure
@7   Macrosoft 365         $99.99  in 7 months  Productivity
@10  FjordVPN              $47.88  in 8 months  Security

18 expenses  904.54 zł/month  10854.50 zł/year
```

```
$ recu timeline
date      name                  amount
────────  ────────────────────  ──────
Apr 2026
      22  Streamberry           $19.99
      24  Web Hosting            $6.99
      25  Ghibli+               $11.99
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
Total  855.72 zł
```

```
$ recu treemap
┌──────────────────────────────┐┌────────────────┐┌──────────────┐┌────────────┐
│Adobo Creative Cloud          ││ChatGBT Plus    ││Amazoo Prime  ││Pear TV+    │
│198 zł/mo                     ││72 zł/mo        ││54 zł/mo      ││47 zł/mo    │
│2374 zł/yr                    ││863 zł/yr       ││647 zł/yr     ││561 zł/yr   │
│                              ││                ││              ││            │
│                              ││                │└──────────────┘└────────────┘
│                              │└────────────────┘┌────────┐┌─────────┐┌───────┐
│                              │┌────────────────┐│Ghibli+ ││Spookify ││Macros…│
│                              ││Streamberry     ││43 zł/mo││36 zł/mo ││30 zł/…│
│                              ││72 zł/mo        ││518 zł/…││431 zł/yr││360 zł…│
└──────────────────────────────┘│863 zł/yr       ││        ││         ││       │
┌──────────────────────────────┐│                ││        │└─────────┘└───────┘
│Gym                           ││                ││        │┌───────┐┌────┌────┐
│162 zł/mo                     │└────────────────┘└────────┘│Web Ho…││Git…│Fjo…│
│1942 zł/yr                    │┌────────────────┐┌────────┐│25 zł/…││    │    │
│                              ││ViewTube Premium││Potion  ││302 zł…│└────└────┘
│                              ││58 zł/mo        ││36 zł/mo│┌───────┐┌───┌─────┐
│                              ││690 zł/yr       ││432 zł/…││0Passw…││iS…│Gogg…│
│                              ││                ││        ││18 zł/…││   └─────┘
└──────────────────────────────┘└────────────────┘└────────┘└───────┘└───┘
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
      --ca <CATEGORY>        Category label (e.g. streaming, utilities)
  -h, --help                 Print help

Examples:
  recu add -n Netflix -a 9.99 -c usd -d 2026-05-01 -i monthly
  recu add -n Netflix          # partial — just the name
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
