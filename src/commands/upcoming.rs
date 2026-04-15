use std::collections::{BTreeMap, HashMap};
use std::io::Write;

use chrono::{Datelike, NaiveDate};
use clap::Args;
use colored::Colorize;
use rusty_money::{Findable, iso};

use crate::config::{self, Config};
use crate::expense::{self, Expense, convert_amount, format_amount};
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
    out: &mut impl Write,
    all: &[Occurrence],
    by_month: &BTreeMap<(i32, u32), Vec<usize>>,
    target_cur: Option<&'static iso::Currency>,
) -> std::io::Result<()> {
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

    if show_totals {
        let cur = target_cur.expect("show_totals implies target_cur is Some");
        let grand_str = format_amount(cur, grand_total);
        writeln!(out, "{}", format!("Total  {grand_str}").bold())?;
    }

    Ok(())
}

pub(crate) fn execute_with(
    out: &mut impl Write,
    today: NaiveDate,
    cfg: &Config,
    expenses: &[(String, Expense)],
    days: u32,
) -> std::io::Result<()> {
    if expenses.is_empty() {
        writeln!(out, "No recurring expenses found.")?;
        return Ok(());
    }

    let target: Option<&str> = cfg.currency.as_deref();
    let exchange_rates: Option<HashMap<String, f64>> = target.map(rates::get_rates).transpose()?;
    let target_cur: Option<&'static iso::Currency> = target
        .and_then(iso::Currency::find)
        .or_else(|| expense::uniform_currency(expenses));

    let end = today + chrono::Days::new(u64::from(days));

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
        writeln!(out, "No upcoming expenses in the next {days} days.")?;
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

    print_timeline(out, &all, &by_month, target_cur)?;

    Ok(())
}

pub fn execute(args: &UpcomingArgs) -> std::io::Result<()> {
    let expenses = store::list()?;
    let cfg = config::load()?;
    let today = chrono::Local::now().date_naive();
    execute_with(&mut std::io::stdout(), today, &cfg, &expenses, args.days)
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

    fn run(expenses: &[(String, Expense)], days: u32) -> String {
        let mut buf = Vec::new();
        execute_with(&mut buf, today(), &Config::default(), expenses, days).expect("execute_with");
        String::from_utf8(buf).expect("utf8")
    }

    fn monthly_usd(amount: f64, next_due: NaiveDate) -> Expense {
        Expense {
            amount: Some(amount),
            currency: Some("usd".to_string()),
            next_due: Some(next_due),
            interval: Some(Interval::Monthly),
            ..Default::default()
        }
    }

    #[test]
    fn upcoming() {
        let mut s = insta::Settings::clone_current();
        s.add_filter(r"\x1b\[[0-9;]*m", "");
        let _guard = s.bind_to_scope();

        let mut out = String::new();

        out += "=== empty store ===\n";
        out += &run(&[], 30);

        // Due June 1 = 47 days away, outside 30-day window
        out += "\n=== no occurrences in window ===\n";
        out += &run(
            &[("Netflix".to_string(), monthly_usd(15.99, d(2026, 6, 1)))],
            30,
        );

        // April 20 = 5 days away, within 30-day window
        out += "\n=== single in range ===\n";
        out += &run(
            &[("Netflix".to_string(), monthly_usd(15.99, d(2026, 4, 20)))],
            30,
        );

        // 60-day window: April 20 + May 20 both appear
        out += "\n=== monthly spans two months ===\n";
        out += &run(
            &[("Netflix".to_string(), monthly_usd(15.99, d(2026, 4, 20)))],
            60,
        );

        // Two expenses, 60-day window → entries across April, May, June
        out += "\n=== multiple across months ===\n";
        out += &run(
            &[
                ("Netflix".to_string(), monthly_usd(15.99, d(2026, 4, 20))),
                ("Spotify".to_string(), monthly_usd(9.99, d(2026, 5, 1))),
            ],
            60,
        );

        // All USD → uniform_currency → total line shown
        out += "\n=== totals with uniform currency ===\n";
        out += &run(
            &[
                ("Netflix".to_string(), monthly_usd(15.99, d(2026, 4, 20))),
                ("Spotify".to_string(), monthly_usd(9.99, d(2026, 4, 25))),
            ],
            30,
        );

        insta::assert_snapshot!(out);
    }
}
