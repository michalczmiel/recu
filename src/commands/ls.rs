use std::collections::HashMap;
use std::io::Write;

use chrono::NaiveDate;
use colored::Colorize;

use crate::config::{self, Config};
use crate::expense::{self, DueStatus, Expense, format_amount};
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

fn build_row(index: usize, name: &str, expense: &Expense, today: NaiveDate) -> [String; 4] {
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
    out: &mut impl Write,
    expenses: &[&Expense],
    rates: Option<&HashMap<String, f64>>,
    target: Option<&str>,
    target_cur: Option<&'static iso::Currency>,
) -> std::io::Result<()> {
    let Some(cur) = target_cur else { return Ok(()) };
    let monthly = expense::monthly_total(expenses, rates, target, target_cur);
    let line = format!(
        "\nTotal  {}/month  {}/year",
        format_amount(cur, monthly),
        format_amount(cur, monthly * 12.0)
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
    rows: &[[String; 4]],
    statuses: &[DueStatus],
) -> std::io::Result<()> {
    let headers = ["#", "name", "amount", "due"];
    // Widths in visible columns (char count), not bytes.
    let widths: [usize; 4] = std::array::from_fn(|i| {
        rows.iter()
            .fold(char_width(headers[i]), |w, row| w.max(char_width(&row[i])))
    });
    let [w0, w1, w2, w3] = widths;
    writeln!(
        out,
        "{:<w0$}  {:<w1$}  {:>w2$}  {:<w3$}",
        headers[0], headers[1], headers[2], headers[3]
    )?;
    writeln!(
        out,
        "{:─<w0$}  {:─<w1$}  {:─<w2$}  {:─<w3$}",
        "", "", "", ""
    )?;
    for (row, status) in rows.iter().zip(statuses.iter()) {
        let c = colorize_row(row, status);
        writeln!(
            out,
            "{}  {}  {}  {}",
            pad_end(&c[0], char_width(&row[0]), w0),
            pad_end(&c[1], char_width(&row[1]), w1),
            pad_start(&c[2], char_width(&row[2]), w2), // amount: right-aligned
            pad_end(&c[3], char_width(&row[3]), w3),
        )?;
    }
    Ok(())
}

pub(crate) fn execute_with(
    out: &mut impl Write,
    today: NaiveDate,
    cfg: &Config,
    expenses: &[(String, Expense)],
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

    let mut indexed: Vec<(usize, &str, &Expense)> = expenses
        .iter()
        .enumerate()
        .map(|(i, (name, expense))| (i, name.as_str(), expense))
        .collect();
    indexed.sort_by_key(|(_, _, expense)| expense.days_until_next(today).unwrap_or(i64::MAX));

    let rows: Vec<[String; 4]> = indexed
        .iter()
        .map(|(i, name, expense)| build_row(*i, name, expense, today))
        .collect();

    let statuses: Vec<DueStatus> = indexed
        .iter()
        .map(|(_, _, expense)| expense.due_status(today))
        .collect();

    print_table(out, &rows, &statuses)?;

    let expense_refs: Vec<&Expense> = indexed.iter().map(|(_, _, e)| *e).collect();
    print_totals(
        out,
        &expense_refs,
        exchange_rates.as_ref(),
        target,
        target_cur,
    )?;

    Ok(())
}

pub fn execute() -> std::io::Result<()> {
    let expenses = store::list()?;
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

    fn run(expenses: &[(String, Expense)]) -> String {
        let mut buf = Vec::new();
        execute_with(&mut buf, today(), &Config::default(), expenses).expect("execute_with");
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
    fn ls() {
        let mut s = insta::Settings::clone_current();
        s.add_filter(r"\x1b\[[0-9;]*m", "");
        let _guard = s.bind_to_scope();

        let mut out = String::new();

        out += "=== empty ===\n";
        out += &run(&[]);

        // next_due == today → days = 0 → Overdue
        out += "\n=== single due today ===\n";
        out += &run(&[("Netflix".to_string(), monthly_usd(15.99, today()))]);

        // 5 days away → DueSoon
        out += "\n=== single due soon ===\n";
        out += &run(&[("Netflix".to_string(), monthly_usd(15.99, d(2026, 4, 20)))]);

        // 77 days away → Distant
        out += "\n=== single distant ===\n";
        out += &run(&[("Netflix".to_string(), monthly_usd(15.99, d(2026, 7, 1)))]);

        // Added in reverse order; output sorted by due date, @ids reflect insertion order
        out += "\n=== sorted by due date ===\n";
        out += &run(&[
            ("Notion".to_string(), monthly_usd(16.00, d(2026, 7, 1))),
            ("Spotify".to_string(), monthly_usd(9.99, d(2026, 4, 20))),
            ("Netflix".to_string(), monthly_usd(15.99, today())),
        ]);

        // All USD → uniform_currency → totals shown
        out += "\n=== totals with uniform currency ===\n";
        out += &run(&[
            ("Netflix".to_string(), monthly_usd(15.99, d(2026, 5, 1))),
            ("Spotify".to_string(), monthly_usd(9.99, d(2026, 5, 15))),
        ]);

        // No currency → uniform_currency returns None → no totals line
        out += "\n=== no currency, no totals ===\n";
        out += &run(&[(
            "Rent".to_string(),
            Expense {
                amount: Some(1000.0),
                next_due: Some(d(2026, 5, 1)),
                interval: Some(Interval::Monthly),
                ..Default::default()
            },
        )]);

        // No amount or date → dashes in amount and due columns
        out += "\n=== incomplete expense ===\n";
        out += &run(&[("Unknown".to_string(), Expense::default())]);

        insta::assert_snapshot!(out);
    }
}
