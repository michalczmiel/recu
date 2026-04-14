use crate::expense::Interval;
use crate::storage;
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

pub fn execute() -> std::io::Result<()> {
    let expenses = storage::list()?;
    if expenses.is_empty() {
        println!("No recurring expenses found.");
        return Ok(());
    }
    let today = chrono::Local::now().date_naive();
    let mut rows: Vec<[String; 5]> = Vec::new();
    for (index, (name, expense)) in expenses.iter().enumerate() {
        let amount = expense
            .amount
            .map_or_else(|| "-".into(), |a| format!("{a:.2}"));

        let currency_symbol = expense
            .currency
            .as_deref()
            .and_then(|c| iso::Currency::find(&c.to_uppercase()).map(|cur| cur.symbol));

        let currency_interval = match (currency_symbol, &expense.interval) {
            (Some(s), Some(i)) => format!("{}{}", s, interval_label(i)),
            (Some(s), None) => s.to_string(),
            (None, Some(i)) => interval_label(i).trim_start_matches('/').to_string(),
            (None, None) => String::new(),
        };

        let days_str = expense
            .days_until_next(today)
            .map(format_days)
            .unwrap_or_default();

        rows.push([
            format!("@{}", index + 1),
            name.clone(),
            amount,
            currency_interval,
            days_str,
        ]);
    }

    let headers = ["#", "name", "amount", "rate", "due"];
    let col_count = rows[0].len();
    let mut widths = vec![0usize; col_count];
    for (i, h) in headers.iter().enumerate() {
        widths[i] = h.len();
    }
    for row in &rows {
        for (i, cell) in row.iter().enumerate() {
            widths[i] = widths[i].max(cell.len());
        }
    }

    println!(
        "{:<w0$}  {:<w1$}  {:>w2$}  {:<w3$}  {:<w4$}",
        headers[0],
        headers[1],
        headers[2],
        headers[3],
        headers[4],
        w0 = widths[0],
        w1 = widths[1],
        w2 = widths[2],
        w3 = widths[3],
        w4 = widths[4],
    );
    println!(
        "{:─<w0$}  {:─<w1$}  {:─<w2$}  {:─<w3$}  {:─<w4$}",
        "",
        "",
        "",
        "",
        "",
        w0 = widths[0],
        w1 = widths[1],
        w2 = widths[2],
        w3 = widths[3],
        w4 = widths[4],
    );

    for row in &rows {
        println!(
            "{:<w0$}  {:<w1$}  {:>w2$}  {:<w3$}  {:<w4$}",
            row[0],
            row[1],
            row[2],
            row[3],
            row[4],
            w0 = widths[0],
            w1 = widths[1],
            w2 = widths[2],
            w3 = widths[3],
            w4 = widths[4],
        );
    }
    Ok(())
}
