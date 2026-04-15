use std::collections::HashMap;

use colored::Colorize;

use crate::config;
use crate::expense::{DueStatus, Expense, Interval};
use crate::rates;
use crate::store;
use rusty_money::{Findable, iso};

fn colorize_row(row: &[String; 5], status: &DueStatus) -> [String; 5] {
    let apply = |s: &String| -> String {
        match status {
            DueStatus::Overdue => s.red().to_string(),
            DueStatus::DueSoon => s.yellow().to_string(),
            DueStatus::Distant => s.dimmed().to_string(),
            DueStatus::Normal | DueStatus::Unknown => s.clone(),
        }
    };
    std::array::from_fn(|i| apply(&row[i]))
}

fn format_days(days: i64) -> String {
    match days {
        0 => "today".to_string(),
        1..=6 => format!("in {} day{}", days, if days == 1 { "" } else { "s" }),
        7..=29 => {
            let w = days / 7;
            format!("in {} week{}", w, if w == 1 { "" } else { "s" })
        }
        30..=364 => {
            let m = days / 30;
            format!("in {} month{}", m, if m == 1 { "" } else { "s" })
        }
        _ => {
            let y = days / 365;
            format!("in {} year{}", y, if y == 1 { "" } else { "s" })
        }
    }
}

fn interval_label(interval: &Interval) -> &'static str {
    match interval {
        Interval::Weekly => "/week",
        Interval::Monthly => "/month",
        Interval::Quarterly => "/quarter",
        Interval::Yearly => "/year",
    }
}

fn format_amount(cur: &iso::Currency, amount: f64) -> String {
    if cur.symbol_first {
        format!("{}{:.2}", cur.symbol, amount)
    } else {
        format!("{:.2} {}", amount, cur.symbol)
    }
}

fn format_rate(cur: &iso::Currency, interval: &Interval) -> String {
    let lbl = interval_label(interval);
    if cur.symbol_first {
        format!("{}{}", cur.symbol, lbl)
    } else {
        format!("{} {}", lbl.trim_start_matches('/'), cur.symbol)
    }
}

fn build_row(index: usize, name: &str, expense: &Expense) -> [String; 5] {
    let today = chrono::Local::now().date_naive();

    let cur = expense
        .currency
        .as_deref()
        .and_then(|c| iso::Currency::find(&c.to_uppercase()));

    let amount = expense
        .amount
        .map_or_else(|| "-".into(), |a| format!("{a:.2}"));
    let currency_interval = match (cur, &expense.interval) {
        (Some(c), Some(i)) => format_rate(c, i),
        (Some(c), None) => c.symbol.to_string(),
        (None, Some(i)) => interval_label(i).trim_start_matches('/').to_string(),
        (None, None) => String::new(),
    };
    let days_str = expense
        .days_until_next(today)
        .map(format_days)
        .unwrap_or_default();

    [
        format!("@{}", index + 1),
        name.to_string(),
        amount,
        currency_interval,
        days_str,
    ]
}

fn print_totals(
    expenses: &[(usize, &str, &Expense)],
    rates: Option<&HashMap<String, f64>>,
    target: Option<&str>,
    target_cur: Option<&'static iso::Currency>,
) {
    let monthly_total: f64 = expenses
        .iter()
        .filter_map(|(_, _, e)| {
            let amt = e.amount?;
            let interval = e.interval.as_ref()?;
            let (converted, _) =
                rates::convert_amount(amt, e.currency.as_deref(), rates, target, target_cur);
            Some(interval.to_monthly(converted))
        })
        .sum();

    let yearly_total = monthly_total * 12.0;
    if let Some(cur) = target_cur {
        let monthly_str = format_amount(cur, monthly_total);
        let yearly_str = format_amount(cur, yearly_total);
        let line = format!("\nTotal  {monthly_str}/month  {yearly_str}/year");
        println!("{}", line.bold());
    }
}

/// Visible width of a plain string in terminal columns.
/// Uses char count — sufficient for Latin/currency symbols (all 1-column wide).
fn char_width(s: &str) -> usize {
    s.chars().count()
}

/// Pad `colored` (which may contain ANSI codes) to `width` columns,
/// using `plain_w` (visible char width) for the math.
fn pad_end(colored: &str, plain_w: usize, width: usize) -> String {
    let spaces = width.saturating_sub(plain_w);
    format!("{colored}{}", " ".repeat(spaces))
}

fn pad_start(colored: &str, plain_w: usize, width: usize) -> String {
    let spaces = width.saturating_sub(plain_w);
    format!("{}{colored}", " ".repeat(spaces))
}

fn print_table(rows: &[[String; 5]], statuses: &[DueStatus]) {
    let headers = ["#", "name", "amount", "rate", "due"];
    // Widths in visible columns (char count), not bytes.
    let widths: [usize; 5] = std::array::from_fn(|i| {
        rows.iter()
            .fold(char_width(headers[i]), |w, row| w.max(char_width(&row[i])))
    });
    let [w0, w1, w2, w3, w4] = widths;
    println!(
        "{:<w0$}  {:<w1$}  {:>w2$}  {:<w3$}  {:<w4$}",
        headers[0], headers[1], headers[2], headers[3], headers[4]
    );
    println!(
        "{:─<w0$}  {:─<w1$}  {:─<w2$}  {:─<w3$}  {:─<w4$}",
        "", "", "", "", ""
    );
    for (row, status) in rows.iter().zip(statuses.iter()) {
        let c = colorize_row(row, status);
        println!(
            "{}  {}  {}  {}  {}",
            pad_end(&c[0], char_width(&row[0]), w0),
            pad_end(&c[1], char_width(&row[1]), w1),
            pad_start(&c[2], char_width(&row[2]), w2), // amount: right-aligned
            pad_end(&c[3], char_width(&row[3]), w3),
            pad_end(&c[4], char_width(&row[4]), w4),
        );
    }
}

pub fn execute() -> std::io::Result<()> {
    let expenses = store::list()?;
    if expenses.is_empty() {
        println!("No recurring expenses found.");
        return Ok(());
    }

    let cfg = config::load()?;
    let target: Option<&str> = cfg.currency.as_deref();
    let exchange_rates: Option<HashMap<String, f64>> = target.map(rates::get_rates).transpose()?;
    let target_cur: Option<&'static iso::Currency> = target.and_then(iso::Currency::find);

    let today = chrono::Local::now().date_naive();
    let mut indexed: Vec<(usize, &str, &Expense)> = expenses
        .iter()
        .enumerate()
        .map(|(i, (name, expense))| (i, name.as_str(), expense))
        .collect();
    indexed.sort_by_key(|(_, _, expense)| expense.days_until_next(today).unwrap_or(i64::MAX));

    let rows: Vec<[String; 5]> = indexed
        .iter()
        .map(|(i, name, expense)| build_row(*i, name, expense))
        .collect();

    let statuses: Vec<DueStatus> = indexed
        .iter()
        .map(|(_, _, expense)| expense.due_status(today))
        .collect();

    print_table(&rows, &statuses);

    if target.is_some() {
        print_totals(&indexed, exchange_rates.as_ref(), target, target_cur);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_amount_symbol_first() {
        let cur = iso::Currency::find("USD").unwrap();
        assert_eq!(format_amount(cur, 42.5), "$42.50");
    }

    #[test]
    fn format_amount_symbol_last() {
        let cur = iso::Currency::find("PLN").unwrap();
        assert_eq!(format_amount(cur, 42.5), "42.50 zł");
    }
}
