# recu

CLI tool for tracking recurring expenses.

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
  rm        Remove a recurring expense
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
$ recu ls
@    name                  amount  due          category
───  ────────────────────  ──────  ───────────  ──────────────
@13  iSmog+                 $2.99  in 1 day     Cloud
@2   Spookify               $9.99  in 2 days    Streaming
@14  Goggle One             $2.99  in 3 days    Cloud
@6   Adobo Creative Cloud  $54.99  in 4 days    Productivity
@1   Streamberry           $19.99  in 6 days    Streaming
@18  Web Hosting            $6.99  in 1 week    Infrastructure
@5   Ghibli+               $11.99  in 1 week    Streaming
@9   0Password              $4.99  in 1 week    Security
@4   Pear TV+              $12.99  in 2 weeks   Streaming
@3   ViewTube Premium      $15.99  in 2 weeks   Streaming
@16  Gym                   $45.00  in 2 weeks   Health
@8   Potion                $10.00  in 2 weeks   Productivity
@11  GitHug Pro             $4.00  in 3 weeks   Dev
@12  ChatGBT Plus          $20.00  in 3 weeks   Dev
@15  Amazoo Prime          $14.99  in 4 weeks   Shopping
@17  Domain                $15.00  in 6 months  Infrastructure
@7   Macrosoft 365         $99.99  in 7 months  Productivity
@10  FjordVPN              $47.88  in 9 months  Security

Total  $251.46/month  $3017.55/year
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
Total  $237.89
```

```
$ recu treemap
┌──────────────────────────────┐┌────────────────┐┌──────────────┐┌────────────┐
│Adobo Creative Cloud          ││ChatGBT Plus    ││Amazoo Prime  ││Pear TV+    │
│$55/mo                        ││$20/mo          ││$15/mo        ││$13/mo      │
│$660/yr                       ││$240/yr         ││$180/yr       ││$156/yr     │
│                              ││                ││              ││            │
│                              ││                │└──────────────┘└────────────┘
│                              │└────────────────┘┌────────┐┌─────────┐┌───────┐
│                              │┌────────────────┐│Ghibli+ ││Spookify ││Macros…│
│                              ││Streamberry     ││$12/mo  ││$10/mo   ││$8/mo  │
│                              ││$20/mo          ││$144/yr ││$120/yr  ││$100/yr│
└──────────────────────────────┘│$240/yr         ││        ││         ││       │
┌──────────────────────────────┐│                ││        │└─────────┘└───────┘
│Gym                           ││                ││        │┌───────┐┌────┌────┐
│$45/mo                        │└────────────────┘└────────┘│Web Ho…││Git…│Fjo…│
│$540/yr                       │┌────────────────┐┌────────┐│$7/mo  ││    │    │
│                              ││ViewTube Premium││Potion  ││$84/yr │└────└────┘
│                              ││$16/mo          ││$10/mo  │┌───────┐┌───┌─────┐
│                              ││$192/yr         ││$120/yr ││0Passw…││iS…│Gogg…│
│                              ││                ││        ││$5/mo  ││   └─────┘
└──────────────────────────────┘└────────────────┘└────────┘└───────┘└───┘
```
