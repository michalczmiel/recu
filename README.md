# recu

CLI tool for tracking recurring expenses.

> This project is still in development, the interfaces and features may change.

## Install

```
cargo install recu
```

## Usage

```
$ recu --help
Track recurring expenses. Uses ./recu.csv by default, or RECU_FILE to override the storage file path.

Usage: recu [COMMAND]

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
  -h, --help
          Print help (see a summary with '-h')

  -V, --version
          Print version
```

### Tip: version your data

Keep `recu.csv` in a git repo for free history and diffs. An example is a dedicated folder like `~/.finances` pointed to via `RECU_FILE` in your `~/.bashrc` or `~/.zshrc`:

```
export RECU_FILE=~/.finances/recu.csv
```

### Example

```
$ recu help add
Add a recurring expense

Usage: recu add [OPTIONS]

Options:
  -n, --name <NAME>          Expense name
  -a, --amount <AMOUNT>      Amount (e.g. 9.99 or 9,99)
  -c, --currency <CURRENCY>  ISO 4217 currency code (e.g. usd, eur)
  -d, --date <DATE>          Start date (YYYY-MM-DD)
  -i, --interval <INTERVAL>  Billing interval [possible values: weekly, monthly, quarterly, yearly]
      --ca <CATEGORY>        Category label (e.g. streaming, utilities)
  -h, --help                 Print help

Examples:
  recu add -n Netflix -a 9.99 -c usd -d 2026-05-01 -i monthly
  recu add --name Netflix --amount 9.99 --currency usd --date 2026-05-01 --interval monthly
  recu add          # interactive mode
```

```
$ recu help edit
Edit a recurring expense

Usage: recu edit [OPTIONS] <TARGET>

Arguments:
  <TARGET>  Expense to edit: @id or name (case-insensitive)

Options:
  -n, --name <NAME>          Expense name
  -a, --amount <AMOUNT>      Amount (e.g. 9.99 or 9,99)
  -c, --currency <CURRENCY>  ISO 4217 currency code (e.g. usd, eur)
  -d, --date <DATE>          Start date (YYYY-MM-DD)
  -i, --interval <INTERVAL>  Billing interval [possible values: weekly, monthly, quarterly, yearly]
      --ca <CATEGORY>        Category label (e.g. streaming, utilities)
  -h, --help                 Print help

Examples:
  recu edit @1 -a 12.99
  recu edit Netflix --interval yearly
  recu edit Netflix          # interactive mode
```

```
$ recu help rm
Remove one or more recurring expenses

Usage: recu rm [TARGETS]...

Arguments:
  [TARGETS]...  Expense(s) to remove: @id or name (case-insensitive), comma-separated. When using @id, run 'recu ls' first to see current indices. For multiple targets, prefer @id to avoid ambiguity

Options:
  -h, --help  Print help

Examples:
  recu rm Netflix
  recu rm netflix              (case-insensitive)
  recu rm @2                   (run 'recu ls' first to see indices)
  recu rm @3,@1                (indices resolved before any removal; use 'recu ls' first)
  recu rm Netflix,Spotify      (comma-separated; prefer @id when mixing with index targets)
```

```
$ recu ls
@    name                  amount  due          category
───  ────────────────────  ──────  ───────────  ──────────────
@13  iSmog+                 $2.99  today        Cloud
@2   Spookify               $9.99  in 1 day     Streaming
@14  Goggle One             $2.99  in 2 days    Cloud
@6   Adobo Creative Cloud  $54.99  in 3 days    Productivity
@1   Streamberry           $19.99  in 5 days    Streaming
@18  Web Hosting            $6.99  in 1 week    Infrastructure
@5   Ghibli+               $11.99  in 1 week    Streaming
@9   0Password              $4.99  in 1 week    Security
@4   Pear TV+              $12.99  in 1 week    Streaming
@3   ViewTube Premium      $15.99  in 2 weeks   Streaming
@16  Gym                   $45.00  in 2 weeks   Health
@8   Potion                $10.00  in 2 weeks   Productivity
@11  GitHug Pro             $4.00  in 2 weeks   Dev
@12  ChatGBT Plus          $20.00  in 3 weeks   Dev
@15  Amazoo Prime          $14.99  in 4 weeks   Shopping
@17  Domain                $15.00  in 6 months  Infrastructure
@7   Macrosoft 365         $99.99  in 7 months  Productivity
@10  FjordVPN              $47.88  in 9 months  Security

Total  905.16 zł/month  10861.92 zł/year
```

```
$ recu timeline
date      name                  amount
────────  ────────────────────  ──────
Apr 2026
      17  iSmog+                 $2.99
      18  Spookify               $9.99
      19  Goggle One             $2.99
      20  Adobo Creative Cloud  $54.99
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
Total  867.07 zł
```

```
$ recu treemap
┌──────────────────────────────┐┌────────────────┐┌──────────────┐┌────────────┐
│Adobo Creative Cloud          ││ChatGBT Plus    ││Amazoo Prime  ││Pear TV+    │
│198 zł/mo                     ││72 zł/mo        ││54 zł/mo      ││47 zł/mo    │
│2375 zł/yr                    ││864 zł/yr       ││647 zł/yr     ││561 zł/yr   │
│                              ││                ││              ││            │
│                              ││                │└──────────────┘└────────────┘
│                              │└────────────────┘┌────────┐┌─────────┐┌───────┐
│                              │┌────────────────┐│Ghibli+ ││Spookify ││Macros…│
│                              ││Streamberry     ││43 zł/mo││36 zł/mo ││30 zł/…│
│                              ││72 zł/mo        ││518 zł/…││432 zł/yr││360 zł…│
└──────────────────────────────┘│863 zł/yr       ││        ││         ││       │
┌──────────────────────────────┐│                ││        │└─────────┘└───────┘
│Gym                           ││                ││        │┌───────┐┌────┌────┐
│162 zł/mo                     │└────────────────┘└────────┘│Web Ho…││Git…│Fjo…│
│1944 zł/yr                    │┌────────────────┐┌────────┐│25 zł/…││    │    │
│                              ││ViewTube Premium││Potion  ││302 zł…│└────└────┘
│                              ││58 zł/mo        ││36 zł/mo│┌───────┐┌───┌─────┐
│                              ││691 zł/yr       ││432 zł/…││0Passw…││iS…│Gogg…│
│                              ││                ││        ││18 zł/…││   └─────┘
└──────────────────────────────┘└────────────────┘└────────┘└───────┘└───┘
```
