# recu

[![Crates.io](https://img.shields.io/crates/v/recu.svg)](https://crates.io/crates/recu)
[![npm](https://img.shields.io/npm/v/@michalczmiel/recu.svg)](https://www.npmjs.com/package/@michalczmiel/recu)
[![CI](https://github.com/michalczmiel/recu/actions/workflows/ci.yml/badge.svg)](https://github.com/michalczmiel/recu/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/crates/l/recu.svg)](LICENSE)

`recu` is a cli for managing and visualizing recurring expenses using CSV.

[Demo](#demo) • [Features](#features) • [Installation](#installation) • [Examples](#examples) • [Tips](#tips) • [Schema](#schema)

## Demo

![recu demo](https://raw.githubusercontent.com/michalczmiel/recu/main/docs/demo.gif)

```sh
$ recu list
@    name                     amount  due     category
───  ────────────────────  ─────────  ──────  ──────────────
@15  Amazoo Prime          $14.99/mo  Fri     Shopping
@13  iSmog+                 $2.99/mo  Sun     Cloud
@2   Spookify               $9.99/mo  Mon     Streaming
@14  Goggle One             $2.99/mo  Tue     Cloud
@6   Adobo Creative Cloud  $54.99/mo  May 20  Productivity
@1   Streamberry           $19.99/mo  May 22  Streaming
@18  Web Hosting            $6.99/mo  May 24  Infrastructure
@5   Ghibli+               $11.99/mo  May 25  Streaming
@9   0Password              $4.99/mo  May 28  Security
@4   Pear TV+              $12.99/mo  May 30  Streaming
@3   ViewTube Premium      $15.99/mo  Jun 1   Streaming
@16  Gym                   $45.00/mo  Jun 1   Health
@8   Potion                $10.00/mo  Jun 3   Productivity
@11  GitHug Pro             $4.00/mo  Jun 7   Dev
@12  ChatGBT Plus          $20.00/mo  Jun 10  Dev
@17  Domain                $15.00/yr  Nov 8   Infrastructure
@7   Macrosoft 365         $99.99/yr  Dec 1   Productivity

17 expenses  $247.47/month  $2969.67/year
  Productivity    $73.32/mo   30%
  Streaming       $70.95/mo   29%
  Health          $45.00/mo   18%
  Dev             $24.00/mo   10%
  Shopping        $14.99/mo    6%
  Infrastructure   $8.24/mo    3%
  Cloud            $5.98/mo    2%
  Security         $4.99/mo    2%
+ 1 ended (recu list --all)
```

## Features

- You own your CSV, edit by hand, version with git
- List, add, edit, rename, remove expenses
- Categories with merge/rename, spend breakdown and percentages
- Visualizations: month calendar, treemap
- Multi-currency with auto-conversion to a display currency
- Filter by min/max monthly cost; `text` or `json` output for scripting
- Undo for the last mutating command
- Supports custom CSV columns
- Shell completion generation

## Installation

Install globally with your preferred method

```sh
npm install -g @michalczmiel/recu
```

```sh
cargo install recu
```

```sh
cargo binstall recu
```

## Tips

- Keep `recu.csv` in a git repo (e.g. `~/.finances`) for free history and diffs. Gitignore the working files recu creates: `*.undo`, `*.seq`.
- Set a default file with `export RECU_FILE=~/.finances/recu.csv`, or target any file with `-f`. Separate datasets (personal, biz, household) are just separate files — alias each: `alias recu-biz='recu -f ~/.finances/biz.csv'`.
- Set a display currency with `recu config set currency pln` — multi-currency entries auto-convert on display.
- Set `--end` when a subscription stops to keep it in history instead of removing it; `recu list --all` shows ended ones.
- Pipe to scripts with `recu list --format json | jq ...` — null fields are omitted so the shape stays compact.
- Add your own columns to the CSV (e.g. `vendor`, `notes`) — recu preserves them and shows them as extra columns in `recu list`.
- Reference expenses by `@id` or name (case-insensitive) in any mutating command, and batch them: `recu rm @1,@3,Spookify` removes several at once.
- Flags `--category`, `--min`, and `--max` work on `list`, `calendar`, and `treemap` alike — slice any view, e.g. `recu list -c streaming,dev` or `recu treemap --min 10`.
- Tidy up categories without touching each row: `recu category rename streaming,subs Streaming` merges and renames in one go.
- Plan ahead with `recu calendar --next` or `recu calendar --month 2026-12` to see charges in a future month.
- Hand grunt work to an LLM: point it at `recu --help` and let it do the tedious stuff — bulk cleanup, importing expenses from a bank export or another file, normalizing categories, etc.

## Schema

`recu` stores one expense per CSV row. See [examples/recu.csv](examples/recu.csv) for a full sample.

| Column       | Required | Format / values                         | Notes                                         |
| ------------ | -------- | --------------------------------------- | --------------------------------------------- |
| `id`         | yes      | positive integer, unique                | auto-assigned on `add`; referenced as `@id`   |
| `name`       | yes      | text                                    |                                               |
| `amount`     | no       | decimal, `.` or `,` separator (`9.99`)  | empty = unset, fill in later via `edit`       |
| `currency`   | no       | ISO 4217 code (`usd`, `eur`, `pln`)     | converted to display currency when configured |
| `start_date` | no       | `YYYY-MM-DD`                            | when the subscription began                   |
| `interval`   | no       | `weekly` `monthly` `quarterly` `yearly` | drives monthly/yearly normalization           |
| `category`   | no       | text                                    | free-form label; managed via `recu category`  |
| `end_date`   | no       | `YYYY-MM-DD`                            | set = ended; hidden unless `--all`            |

Any extra columns are preserved and `recu` leaves them untouched. Trailing empty cells may be omitted.

## Currency

When you set a display currency, recu converts amounts using exchange rates fetched from the external Frankfurter API (api.frankfurter.dev). Rates are cached locally at ~/.cache/recu/rates.json and refreshed once a day, so conversion works offline between fetches.

## Examples

```sh
$ recu calendar
                    May 2026

    Mon    Tue    Wed    Thu    Fri    Sat    Sun
                                  1      2      3
                              61(2)            10
      4      5      6      7      8      9     10
                           4                   20
     11     12     13     14     15     16     17
                                 15             3
     18     19     20     21     22     23     24
     10      3     55            20             7
     25     26     27     28     29     30     31
     12                    5            13

15 charges   $237.89   paid $94.99, remaining $142.90
+ 1 ended (recu calendar --all)
```

```sh
$ recu treemap
┌──────────────────────────────┐┌───────────┐┌────────────────┐┌───────────────┐
│Adobo Creative Cloud          ││ChatGBT Pl…││ViewTube Premium││Amazoo Prime   │
│$55/mo                        ││$20/mo     ││$16/mo          ││$15/mo         │
│$660/yr                       ││$240/yr    ││$192/yr         ││$180/yr        │
│                              ││           ││                ││               │
│                              ││           │└────────────────┘└───────────────┘
│                              ││           │┌─────────┐ ┌──────────┌──────────┐
│                              ││           ││Pear TV+ │ │Potion    │Spookify  │
│                              ││           ││$13/mo   │ │$10/mo    │$10/mo    │
│                              │└───────────┘│$156/yr  │ │$120/yr   │$120/yr   │
└──────────────────────────────┘┌───────────┐│         │ │          │          │
┌──────────────────────────────┐│Streamberry││         │ ┌─────────┐┌─────┐┌───┐
│Gym                           ││$20/mo     │└─────────┘ │Macrosof…││0Pas…││Gi…│
│$45/mo                        ││$240/yr    │┌─────────┐ │$8/mo    ││$5/mo││   │
│$540/yr                       ││           ││Ghibli+  │ │$100/yr  ││$60/…││   │
│                              ││           ││$12/mo   │ └─────────┘└─────┘└───┘
│                              ││           ││$144/yr  │ ┌─────────┐┌───┐┌─────┐
│                              ││           ││         │ │Web Host…││iS…││Gogg…│
│                              ││           ││         │ │$7/mo    ││   │└─────┘
└──────────────────────────────┘└───────────┘└─────────┘ └─────────┘└───┘
```

```sh
$ recu --help
Track recurring expenses

Usage: recu [OPTIONS] [COMMAND]

Commands:
  list        List recurring expenses. Amounts converted to display currency when configured [aliases: ls]
  add         Add a recurring expense
  edit        Edit a recurring expense
  rename      Rename a recurring expense
  remove      Remove one or more recurring expenses [aliases: rm]
  treemap     Visualize expenses as a treemap
  config      Manage configuration
  category    Manage expense categories
  calendar    Show recurring expenses on a month grid
  undo        Undo the last add, edit, rename, or remove
  completion  Generate shell completion script
  help        Print this message or the help of the given subcommand(s)

Options:
  -f, --file <FILE>      Path to the CSV storage file [env: RECU_FILE=examples/recu.csv] [default: recu.csv]
  -a, --all              Include ended expenses (only used when no subcommand is given; equivalent to `recu list --all`)
      --format <FORMAT>  Output format (only used when no subcommand is given; equivalent to `recu list --format <FORMAT>`) [possible values: text, json]
      --min <MIN>        Only show expenses costing at least this much per month
      --max <MAX>        Only show expenses costing at most this much per month
  -h, --help             Print help
  -V, --version          Print version
```
