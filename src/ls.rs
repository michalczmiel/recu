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
    for (index, (name, expense)) in expenses.iter().enumerate() {
        let amount = expense
            .amount
            .map(|a| format!("{:.2}", a))
            .unwrap_or_else(|| "-".into());

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

        let tags = expense
            .tags
            .as_ref()
            .map(|t| {
                t.iter()
                    .map(|tag| format!("#{}", tag))
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .unwrap_or_default();

        let days_str = expense
            .days_until_next(today)
            .map(format_days)
            .unwrap_or_default();

        let parts: Vec<String> = [
            format!("@{}", index + 1),
            name.clone(),
            amount,
            currency_interval,
            days_str,
            tags,
        ]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect();

        println!("{}", parts.join(" "));
    }
    Ok(())
}
