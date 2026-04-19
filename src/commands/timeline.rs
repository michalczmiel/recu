use std::collections::{BTreeMap, HashMap};
use std::io::Write;

use chrono::{Datelike, NaiveDate};
use clap::Args;
use colored::Colorize;
use rusty_money::iso;

use crate::config::{self, Config};
use crate::expense::{self, Expense, convert, find_currency, format_amount};
use crate::rates;
use crate::store::Store;

#[derive(Args, Debug)]
#[command(after_help = "Examples:
  recu timeline
  recu timeline --days 60 [default: 30]
  recu timeline --past-days 30
  recu timeline --past-days 14 --days 60")]
pub struct TimelineArgs {
    /// Number of days to look ahead [default: 30]
    #[arg(short, long, default_value_t = 30)]
    pub days: u32,
    /// Number of days to look back [default: 0]
    #[arg(short, long, default_value_t = 0)]
    pub past_days: u32,
}

struct Occurrence {
    date: NaiveDate,
    name: String,
    display_amount: String,
    converted_amount: f64,
}

fn occurrences_in_range(
    name: &str,
    expense: &Expense,
    start: NaiveDate,
    end: NaiveDate,
    rates: Option<&HashMap<String, f64>>,
    target: Option<&str>,
) -> Vec<Occurrence> {
    let (Some(first), Some(interval), Some(amount)) = (
        expense.start_date,
        expense.interval.as_ref(),
        expense.amount,
    ) else {
        return vec![];
    };

    let display_cur = expense.currency.as_deref().and_then(find_currency);

    let display_amount = match display_cur {
        Some(c) => format_amount(c, amount),
        None => format!("{amount:.2}"),
    };

    let converted = convert(amount, expense.currency.as_deref(), rates, target);

    let mut result = vec![];
    let mut d = interval.next_payment(first, start);
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
    out: &mut impl Write,
    all: &[Occurrence],
    by_month: &BTreeMap<(i32, u32), Vec<usize>>,
    target_cur: Option<&'static iso::Currency>,
    today: NaiveDate,
    show_future_total: bool,
) -> std::io::Result<()> {
    let future_total: f64 = all
        .iter()
        .filter(|o| o.date >= today)
        .map(|o| o.converted_amount)
        .sum();

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
    writeln!(
        out,
        "{:<date_w$}  {:<name_w$}  {:>amount_w$}",
        "date", "name", "amount"
    )?;
    writeln!(
        out,
        "{:─<date_w$}  {:─<name_w$}  {:─<amount_w$}",
        "", "", ""
    )?;

    for ((year, month), idxs) in by_month {
        let month_str = NaiveDate::from_ymd_opt(*year, *month, 1)
            .map(|d| d.format("%b %Y").to_string())
            .unwrap_or_default();

        writeln!(out, "{}", month_str.bold())?;

        for &i in idxs {
            let occ = &all[i];
            writeln!(
                out,
                "{:>date_w$}  {:<name_w$}  {:>amount_w$}",
                occ.date.day(),
                occ.name,
                occ.display_amount
            )?;
        }
    }

    if show_future_total {
        let cur = target_cur.expect("show_future_total implies target_cur is Some");
        let total_str = format_amount(cur, future_total);
        writeln!(out, "{}", format!("Total  {total_str}").bold())?;
    }

    Ok(())
}

pub(crate) fn execute_with(
    out: &mut impl Write,
    today: NaiveDate,
    cfg: &Config,
    expenses: &[Expense],
    days: u32,
    past_days: u32,
) -> std::io::Result<()> {
    if expenses.is_empty() {
        writeln!(out, "No recurring expenses found.")?;
        return Ok(());
    }

    let target: Option<&str> = cfg.currency.as_deref();
    let exchange_rates: Option<HashMap<String, f64>> = target.map(rates::get_rates).transpose()?;
    let target_cur: Option<&'static iso::Currency> = target
        .and_then(find_currency)
        .or_else(|| expense::uniform_currency(expenses));

    let start = today - chrono::Days::new(u64::from(past_days));
    let end = today + chrono::Days::new(u64::from(days));

    let mut all: Vec<Occurrence> = expenses
        .iter()
        .flat_map(|exp| {
            occurrences_in_range(&exp.name, exp, start, end, exchange_rates.as_ref(), target)
        })
        .collect();

    if all.is_empty() {
        writeln!(out, "No expenses in timeline.")?;
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

    let show_future_total = target_cur.is_some();
    print_timeline(out, &all, &by_month, target_cur, today, show_future_total)?;

    Ok(())
}

pub fn execute(args: &TimelineArgs, store: &Store) -> std::io::Result<()> {
    let expenses = store.list()?;
    let cfg = config::load()?;
    let today = chrono::Local::now().date_naive();
    execute_with(
        &mut std::io::stdout(),
        today,
        &cfg,
        &expenses,
        args.days,
        args.past_days,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expense::Interval;

    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 4, 15).expect("valid date")
    }

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).expect("valid date")
    }

    fn run(expenses: &[Expense], days: u32, past_days: u32) -> String {
        let mut buf = Vec::new();
        execute_with(
            &mut buf,
            today(),
            &Config::default(),
            expenses,
            days,
            past_days,
        )
        .expect("execute_with");
        String::from_utf8(buf).expect("utf8")
    }

    fn monthly_usd(name: &str, amount: f64, start_date: NaiveDate) -> Expense {
        Expense {
            name: name.to_string(),
            amount: Some(amount),
            currency: Some("usd".to_string()),
            start_date: Some(start_date),
            interval: Some(Interval::Monthly),
            ..Default::default()
        }
    }

    #[test]
    fn timeline() {
        let mut s = insta::Settings::clone_current();
        s.add_filter(r"\x1b\[[0-9;]*m", "");
        let _guard = s.bind_to_scope();

        let mut out = String::new();

        out += "=== empty store ===\n";
        out += &run(&[], 30, 0);

        // Due June 1 = 47 days away, outside 30-day window
        out += "\n=== no occurrences in window ===\n";
        out += &run(&[monthly_usd("Netflix", 15.99, d(2026, 6, 1))], 30, 0);

        // April 20 = 5 days away, within 30-day window
        out += "\n=== single in range ===\n";
        out += &run(&[monthly_usd("Netflix", 15.99, d(2026, 4, 20))], 30, 0);

        // 60-day window: April 20 + May 20 both appear
        out += "\n=== monthly spans two months ===\n";
        out += &run(&[monthly_usd("Netflix", 15.99, d(2026, 4, 20))], 60, 0);

        // Two expenses, 60-day window → entries across April, May, June
        out += "\n=== multiple across months ===\n";
        out += &run(
            &[
                monthly_usd("Netflix", 15.99, d(2026, 4, 20)),
                monthly_usd("Spotify", 9.99, d(2026, 5, 1)),
            ],
            60,
            0,
        );

        // All USD → uniform_currency → total line shown
        out += "\n=== totals with uniform currency ===\n";
        out += &run(
            &[
                monthly_usd("Netflix", 15.99, d(2026, 4, 20)),
                monthly_usd("Spotify", 9.99, d(2026, 4, 25)),
            ],
            30,
            0,
        );

        // Past 14 days: April 1 is in range (today is April 15, past=14 → start=April 1)
        out += "\n=== past occurrences ===\n";
        out += &run(&[monthly_usd("Netflix", 15.99, d(2026, 4, 1))], 0, 14);

        // Past + future: April 1 (past) + May 1 (future) both appear
        out += "\n=== past and future combined ===\n";
        out += &run(&[monthly_usd("Netflix", 15.99, d(2026, 4, 1))], 30, 14);

        insta::assert_snapshot!(out);
    }
}
