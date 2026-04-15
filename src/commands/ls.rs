use std::collections::HashMap;

use colored::Colorize;

use crate::config;
use crate::expense::{self, DueStatus, Expense};
use crate::rates;
use crate::store;
use rusty_money::{Findable, iso};

fn colorize_row(row: &[String; 4], status: &DueStatus) -> [String; 4] {
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

fn format_amount(cur: &iso::Currency, amount: f64) -> String {
    if cur.symbol_first {
        format!("{}{:.2}", cur.symbol, amount)
    } else {
        format!("{:.2} {}", amount, cur.symbol)
    }
}

fn build_row(index: usize, name: &str, expense: &Expense) -> [String; 4] {
    let today = chrono::Local::now().date_naive();

    let cur = expense
        .currency
        .as_deref()
        .and_then(|c| iso::Currency::find(&c.to_uppercase()));

    let amount = match (cur, expense.amount) {
        (Some(c), Some(a)) => format_amount(c, a),
        (None, Some(a)) => format!("{a:.2}"),
        _ => "-".into(),
    };
    let days_str = expense
        .days_until_next(today)
        .map(format_days)
        .unwrap_or_default();

    [
        format!("@{}", index + 1),
        name.to_string(),
        amount,
        days_str,
    ]
}

fn print_totals(
    expenses: &[&Expense],
    rates: Option<&HashMap<String, f64>>,
    target: Option<&str>,
    target_cur: Option<&'static iso::Currency>,
) {
    let Some(cur) = target_cur else { return };
    let monthly = expense::monthly_total(expenses, rates, target, target_cur);
    let line = format!(
        "\nTotal  {}/month  {}/year",
        format_amount(cur, monthly),
        format_amount(cur, monthly * 12.0)
    );
    println!("{}", line.bold());
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

fn print_table(rows: &[[String; 4]], statuses: &[DueStatus]) {
    let headers = ["#", "name", "amount", "due"];
    // Widths in visible columns (char count), not bytes.
    let widths: [usize; 4] = std::array::from_fn(|i| {
        rows.iter()
            .fold(char_width(headers[i]), |w, row| w.max(char_width(&row[i])))
    });
    let [w0, w1, w2, w3] = widths;
    println!(
        "{:<w0$}  {:<w1$}  {:>w2$}  {:<w3$}",
        headers[0], headers[1], headers[2], headers[3]
    );
    println!("{:─<w0$}  {:─<w1$}  {:─<w2$}  {:─<w3$}", "", "", "", "");
    for (row, status) in rows.iter().zip(statuses.iter()) {
        let c = colorize_row(row, status);
        println!(
            "{}  {}  {}  {}",
            pad_end(&c[0], char_width(&row[0]), w0),
            pad_end(&c[1], char_width(&row[1]), w1),
            pad_start(&c[2], char_width(&row[2]), w2), // amount: right-aligned
            pad_end(&c[3], char_width(&row[3]), w3),
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
    let target_cur: Option<&'static iso::Currency> = target
        .and_then(iso::Currency::find)
        .or_else(|| expense::uniform_currency(&expenses));

    let today = chrono::Local::now().date_naive();
    let mut indexed: Vec<(usize, &str, &Expense)> = expenses
        .iter()
        .enumerate()
        .map(|(i, (name, expense))| (i, name.as_str(), expense))
        .collect();
    indexed.sort_by_key(|(_, _, expense)| expense.days_until_next(today).unwrap_or(i64::MAX));

    let rows: Vec<[String; 4]> = indexed
        .iter()
        .map(|(i, name, expense)| build_row(*i, name, expense))
        .collect();

    let statuses: Vec<DueStatus> = indexed
        .iter()
        .map(|(_, _, expense)| expense.due_status(today))
        .collect();

    print_table(&rows, &statuses);

    let expense_refs: Vec<&Expense> = indexed.iter().map(|(_, _, e)| *e).collect();
    print_totals(&expense_refs, exchange_rates.as_ref(), target, target_cur);

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
