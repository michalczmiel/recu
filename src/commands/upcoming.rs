use std::collections::{BTreeMap, HashMap};

use chrono::{Datelike, NaiveDate};
use clap::Args;
use colored::Colorize;
use rusty_money::{Findable, iso};

use crate::config;
use crate::expense::{self, Expense, convert_amount};
use crate::rates;
use crate::store;

#[derive(Args, Debug)]
#[command(after_help = "Examples:
  recu upcoming
  recu upcoming --days 60 [default: 30]")]
pub struct UpcomingArgs {
    /// Number of days to look ahead [default: 30]
    #[arg(short, long, default_value_t = 30)]
    pub days: u32,
}

struct Occurrence {
    date: NaiveDate,
    name: String,
    display_amount: String,
    converted_amount: f64,
}

fn format_amount(cur: &iso::Currency, amount: f64) -> String {
    if cur.symbol_first {
        format!("{}{:.2}", cur.symbol, amount)
    } else {
        format!("{:.2} {}", amount, cur.symbol)
    }
}

fn occurrences_in_range(
    name: &str,
    expense: &Expense,
    today: NaiveDate,
    end: NaiveDate,
    rates: Option<&HashMap<String, f64>>,
    target: Option<&str>,
    target_cur: Option<&'static iso::Currency>,
) -> Vec<Occurrence> {
    let (Some(first), Some(interval), Some(amount)) =
        (expense.next_due, expense.interval.as_ref(), expense.amount)
    else {
        return vec![];
    };

    let display_cur = expense
        .currency
        .as_deref()
        .and_then(|c| iso::Currency::find(&c.to_uppercase()));

    let display_amount = match display_cur {
        Some(c) => format_amount(c, amount),
        None => format!("{amount:.2}"),
    };

    let (converted, _) = convert_amount(
        amount,
        expense.currency.as_deref(),
        rates,
        target,
        target_cur,
    );

    let mut result = vec![];
    let mut d = interval.next_payment(first, today);
    while d <= end {
        result.push(Occurrence {
            date: d,
            name: name.to_string(),
            display_amount: display_amount.clone(),
            converted_amount: converted,
        });
        d = interval.next_payment(first, d + chrono::Days::new(1));
    }
    result
}

fn print_timeline(
    all: &[Occurrence],
    by_month: &BTreeMap<(i32, u32), Vec<usize>>,
    target_cur: Option<&'static iso::Currency>,
) {
    let show_totals = target_cur.is_some();
    let grand_total: f64 = all.iter().map(|o| o.converted_amount).sum();

    // "Mmm YYYY" is always 8 chars; wider than header label "date" (4)
    let date_w: usize = 8;
    let name_w = all
        .iter()
        .map(|o| o.name.len())
        .max()
        .unwrap_or(0)
        .max("name".len());
    let amount_w = all
        .iter()
        .map(|o| o.display_amount.len())
        .max()
        .unwrap_or(6)
        .max("amount".len());

    // headers + separator
    println!(
        "{:<date_w$}  {:<name_w$}  {:>amount_w$}",
        "date", "name", "amount"
    );
    println!("{:─<date_w$}  {:─<name_w$}  {:─<amount_w$}", "", "", "");

    for ((year, month), idxs) in by_month {
        let month_str = NaiveDate::from_ymd_opt(*year, *month, 1)
            .map(|d| d.format("%b %Y").to_string())
            .unwrap_or_default();

        println!("{}", month_str.bold());

        for &i in idxs {
            let occ = &all[i];
            println!(
                "{:>date_w$}  {:<name_w$}  {:>amount_w$}",
                occ.date.day(),
                occ.name,
                occ.display_amount
            );
        }
    }

    if show_totals {
        let cur = target_cur.expect("show_totals implies target_cur is Some");
        let grand_str = format_amount(cur, grand_total);
        println!("{}", format!("Total  {grand_str}").bold());
    }
}

pub fn execute(args: &UpcomingArgs) -> std::io::Result<()> {
    let expenses = store::list()?;
    if expenses.is_empty() {
        println!("No recurring expenses found.");
        return Ok(());
    }

    let cfg = config::load()?;
    let target: Option<&str> = cfg.currency.as_deref();
    let exchange_rates: Option<HashMap<String, f64>> = target.map(rates::get_rates).transpose()?;
    let target_cur: Option<&'static iso::Currency> = target
        .and_then(iso::Currency::find)
        .or_else(|| expense::uniform_currency(&expenses));

    let today = chrono::Local::now().date_naive();
    let end = today + chrono::Days::new(u64::from(args.days));

    let mut all: Vec<Occurrence> = expenses
        .iter()
        .flat_map(|(name, exp)| {
            occurrences_in_range(
                name,
                exp,
                today,
                end,
                exchange_rates.as_ref(),
                target,
                target_cur,
            )
        })
        .collect();

    if all.is_empty() {
        println!("No upcoming expenses in the next {} days.", args.days);
        return Ok(());
    }

    all.sort_by_key(|o| o.date);

    let mut by_month: BTreeMap<(i32, u32), Vec<usize>> = BTreeMap::new();
    for (i, occ) in all.iter().enumerate() {
        by_month
            .entry((occ.date.year(), occ.date.month()))
            .or_default()
            .push(i);
    }

    print_timeline(&all, &by_month, target_cur);

    Ok(())
}
