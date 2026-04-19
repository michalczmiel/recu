use std::collections::HashMap;
use std::io::Write;

use chrono::NaiveDate;
use colored::Colorize;

use crate::config::{self, Config};
use rusty_money::iso;

use crate::expense::{self, DueStatus, Expense, RecurringTotals, find_currency, format_amount};
use crate::rates;
use crate::store::Store;

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

fn build_row(index: usize, expense: &Expense, today: NaiveDate) -> [String; 5] {
    let cur = expense.currency.as_deref().and_then(find_currency);

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
        expense.name.clone(),
        amount,
        days_str,
        expense.category.clone().unwrap_or_default(),
    ]
}

fn print_totals(
    out: &mut impl Write,
    totals: &RecurringTotals,
    target_cur: Option<&'static iso::Currency>,
) -> std::io::Result<()> {
    let Some(cur) = target_cur else { return Ok(()) };
    let line = format!(
        "\nTotal  {}/month  {}/year",
        format_amount(cur, totals.monthly),
        format_amount(cur, totals.yearly)
    );
    writeln!(out, "{}", line.bold())
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

fn print_table(
    out: &mut impl Write,
    rows: &[[String; 5]],
    statuses: &[DueStatus],
) -> std::io::Result<()> {
    let headers = ["@", "name", "amount", "due", "category"];
    // Widths in visible columns (char count), not bytes.
    let widths: [usize; 5] = std::array::from_fn(|i| {
        rows.iter()
            .fold(char_width(headers[i]), |w, row| w.max(char_width(&row[i])))
    });
    let [w0, w1, w2, w3, w4] = widths;
    writeln!(
        out,
        "{:<w0$}  {:<w1$}  {:>w2$}  {:<w3$}  {:<w4$}",
        headers[0], headers[1], headers[2], headers[3], headers[4]
    )?;
    writeln!(
        out,
        "{:─<w0$}  {:─<w1$}  {:─<w2$}  {:─<w3$}  {:─<w4$}",
        "", "", "", "", ""
    )?;
    for (row, status) in rows.iter().zip(statuses.iter()) {
        let c = colorize_row(row, status);
        writeln!(
            out,
            "{}  {}  {}  {}  {}",
            pad_end(&c[0], char_width(&row[0]), w0),
            pad_end(&c[1], char_width(&row[1]), w1),
            pad_start(&c[2], char_width(&row[2]), w2), // amount: right-aligned
            pad_end(&c[3], char_width(&row[3]), w3),
            pad_end(&c[4], char_width(&row[4]), w4),
        )?;
    }
    Ok(())
}

pub(crate) fn execute_with(
    out: &mut impl Write,
    today: NaiveDate,
    cfg: &Config,
    expenses: &[Expense],
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

    let mut indexed: Vec<(usize, &Expense)> = expenses.iter().enumerate().collect();
    indexed.sort_by_key(|(_, expense)| expense.days_until_next(today).unwrap_or(i64::MAX));

    let rows: Vec<[String; 5]> = indexed
        .iter()
        .map(|(i, expense)| build_row(*i, expense, today))
        .collect();

    let statuses: Vec<DueStatus> = indexed
        .iter()
        .map(|(_, expense)| expense.due_status(today))
        .collect();

    print_table(out, &rows, &statuses)?;

    let totals = RecurringTotals::compute(
        indexed.iter().map(|(_, e)| *e),
        exchange_rates.as_ref(),
        target,
    );
    print_totals(out, &totals, target_cur)?;

    Ok(())
}

pub fn execute(store: &Store) -> std::io::Result<()> {
    let expenses = store.list()?;
    let cfg = config::load()?;
    let today = chrono::Local::now().date_naive();
    execute_with(&mut std::io::stdout(), today, &cfg, &expenses)
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

    fn run(expenses: &[Expense]) -> String {
        let mut buf = Vec::new();
        execute_with(&mut buf, today(), &Config::default(), expenses).expect("execute_with");
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
    fn ls() {
        let mut s = insta::Settings::clone_current();
        s.add_filter(r"\x1b\[[0-9;]*m", "");
        let _guard = s.bind_to_scope();

        let mut out = String::new();

        out += "=== empty ===\n";
        out += &run(&[]);

        // start_date == today → days = 0 → Overdue
        out += "\n=== single due today ===\n";
        out += &run(&[monthly_usd("Netflix", 15.99, today())]);

        // 5 days away → DueSoon
        out += "\n=== single due soon ===\n";
        out += &run(&[monthly_usd("Netflix", 15.99, d(2026, 4, 20))]);

        // 77 days away → Distant
        out += "\n=== single distant ===\n";
        out += &run(&[monthly_usd("Netflix", 15.99, d(2026, 7, 1))]);

        // Added in reverse order; output sorted by due date, @ids reflect insertion order
        out += "\n=== sorted by due date ===\n";
        out += &run(&[
            monthly_usd("Notion", 16.00, d(2026, 7, 1)),
            monthly_usd("Spotify", 9.99, d(2026, 4, 20)),
            monthly_usd("Netflix", 15.99, today()),
        ]);

        // All USD → uniform_currency → totals shown
        out += "\n=== totals with uniform currency ===\n";
        out += &run(&[
            monthly_usd("Netflix", 15.99, d(2026, 5, 1)),
            monthly_usd("Spotify", 9.99, d(2026, 5, 15)),
        ]);

        // No currency → uniform_currency returns None → no totals line
        out += "\n=== no currency, no totals ===\n";
        out += &run(&[Expense {
            name: "Rent".to_string(),
            amount: Some(1000.0),
            start_date: Some(d(2026, 5, 1)),
            interval: Some(Interval::Monthly),
            ..Default::default()
        }]);

        // No amount or date → dashes in amount and due columns
        out += "\n=== incomplete expense ===\n";
        out += &run(&[Expense {
            name: "Unknown".to_string(),
            ..Default::default()
        }]);

        insta::assert_snapshot!(out);
    }
}
