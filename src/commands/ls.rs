use std::collections::HashMap;

use crate::config;
use crate::expense::{Expense, Interval};
use crate::rates;
use crate::store;
use rusty_money::{Findable, iso};

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

fn format_rate(cur: &iso::Currency, interval: &Interval) -> String {
    let lbl = interval_label(interval);
    if cur.symbol_first {
        format!("{}{}", cur.symbol, lbl)
    } else {
        format!("{} {}", lbl.trim_start_matches('/'), cur.symbol)
    }
}

fn build_row(
    index: usize,
    name: &str,
    expense: &Expense,
    rates: Option<&HashMap<String, f64>>,
    target: Option<&str>,
    target_cur: Option<&'static iso::Currency>,
) -> [String; 5] {
    let today = chrono::Local::now().date_naive();

    let (display_amount, display_cur) = if let Some(amt) = expense.amount {
        let (converted, cur) =
            rates::convert_amount(amt, expense.currency.as_deref(), rates, target, target_cur);
        (Some(converted), cur)
    } else {
        let cur = expense
            .currency
            .as_deref()
            .and_then(|c| iso::Currency::find(&c.to_uppercase()));
        (None, cur)
    };

    let amount = display_amount.map_or_else(|| "-".into(), |a| format!("{a:.2}"));
    let currency_interval = match (display_cur, &expense.interval) {
        (Some(cur), Some(i)) => format_rate(cur, i),
        (Some(cur), None) => cur.symbol.to_string(),
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

fn print_table(rows: &[[String; 5]]) {
    let headers = ["#", "name", "amount", "rate", "due"];
    let widths: [usize; 5] = std::array::from_fn(|i| {
        rows.iter()
            .fold(headers[i].len(), |w, row| w.max(row[i].len()))
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
    for row in rows {
        println!(
            "{:<w0$}  {:<w1$}  {:>w2$}  {:<w3$}  {:<w4$}",
            row[0], row[1], row[2], row[3], row[4]
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

    let rows: Vec<[String; 5]> = expenses
        .iter()
        .enumerate()
        .map(|(i, (name, expense))| {
            build_row(
                i,
                name,
                expense,
                exchange_rates.as_ref(),
                target,
                target_cur,
            )
        })
        .collect();

    print_table(&rows);
    Ok(())
}
