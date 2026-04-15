use std::collections::HashMap;

use chrono::{Datelike, NaiveDate};
use clap::{Args, ValueEnum};
use rusty_money::{Findable, iso};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Interval {
    Weekly,
    Monthly,
    Quarterly,
    Yearly,
}

impl std::str::FromStr for Interval {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "weekly" | "week" => Ok(Interval::Weekly),
            "monthly" | "month" => Ok(Interval::Monthly),
            "quarterly" | "quarter" => Ok(Interval::Quarterly),
            "yearly" | "year" | "annual" | "annually" => Ok(Interval::Yearly),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for Interval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Interval::Weekly => write!(f, "weekly"),
            Interval::Monthly => write!(f, "monthly"),
            Interval::Quarterly => write!(f, "quarterly"),
            Interval::Yearly => write!(f, "yearly"),
        }
    }
}

impl Interval {
    pub fn to_monthly(&self, amount: f64) -> f64 {
        match self {
            Interval::Weekly => amount * 52.0 / 12.0,
            Interval::Monthly => amount,
            Interval::Quarterly => amount / 3.0,
            Interval::Yearly => amount / 12.0,
        }
    }

    pub fn next_payment(&self, first: NaiveDate, today: NaiveDate) -> NaiveDate {
        match self {
            Interval::Weekly => {
                let days_since = (today - first).num_days().rem_euclid(7);
                if days_since == 0 {
                    today
                } else {
                    today + chrono::Days::new((7 - days_since).cast_unsigned())
                }
            }
            Interval::Monthly => advance_months(first, today, 1),
            Interval::Quarterly => advance_months(first, today, 3),
            Interval::Yearly => advance_months(first, today, 12),
        }
    }
}

fn advance_months(first: NaiveDate, today: NaiveDate, step: u32) -> NaiveDate {
    let diff = (today.year() - first.year()) * 12
        + (today.month().cast_signed() - first.month().cast_signed());
    let mut k = (diff.max(0).cast_unsigned() / step) * step;
    loop {
        let candidate = first
            .checked_add_months(chrono::Months::new(k))
            .unwrap_or(first);
        if candidate >= today {
            return candidate;
        }
        k += step;
    }
}

pub enum DueStatus {
    Overdue,
    DueSoon,
    Normal,
    Distant,
    Unknown,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Expense {
    pub amount: Option<f64>,
    pub currency: Option<String>,
    pub next_due: Option<NaiveDate>,
    pub interval: Option<Interval>,
    pub category: Option<String>,
}

impl Expense {
    pub fn next_payment(&self, today: NaiveDate) -> Option<NaiveDate> {
        let first = self.next_due?;
        let interval = self.interval.as_ref()?;
        Some(interval.next_payment(first, today))
    }

    pub fn days_until_next(&self, today: NaiveDate) -> Option<i64> {
        Some((self.next_payment(today)? - today).num_days())
    }

    pub fn due_status(&self, today: NaiveDate) -> DueStatus {
        match self.days_until_next(today) {
            Some(d) if d <= 0 => DueStatus::Overdue,
            Some(d) if d <= 7 => DueStatus::DueSoon,
            Some(d) if d > 60 => DueStatus::Distant,
            Some(_) => DueStatus::Normal,
            None => DueStatus::Unknown,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interval_aliases() {
        assert_eq!(
            "week".parse::<Interval>().expect("valid alias"),
            Interval::Weekly
        );
        assert_eq!(
            "month".parse::<Interval>().expect("valid alias"),
            Interval::Monthly
        );
        assert_eq!(
            "quarter".parse::<Interval>().expect("valid alias"),
            Interval::Quarterly
        );
        assert_eq!(
            "year".parse::<Interval>().expect("valid alias"),
            Interval::Yearly
        );
        assert_eq!(
            "annual".parse::<Interval>().expect("valid alias"),
            Interval::Yearly
        );
        assert_eq!(
            "annually".parse::<Interval>().expect("valid alias"),
            Interval::Yearly
        );
        assert_eq!(
            "YEARLY".parse::<Interval>().expect("valid alias"),
            Interval::Yearly
        );
    }

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

pub fn format_amount(cur: &iso::Currency, amount: f64) -> String {
    if cur.symbol_first {
        format!("{}{:.2}", cur.symbol, amount)
    } else {
        format!("{:.2} {}", amount, cur.symbol)
    }
}

/// Convert `amount` from `expense_currency` to `target`, using `rates`.
/// Returns `(converted_amount, currency_to_display)`.
/// Falls back to the original currency if conversion is not possible.
pub fn convert_amount(
    amount: f64,
    expense_currency: Option<&str>,
    rates: Option<&HashMap<String, f64>>,
    target: Option<&str>,
    target_cur: Option<&'static iso::Currency>,
) -> (f64, Option<&'static iso::Currency>) {
    let original_cur = expense_currency.and_then(|c| iso::Currency::find(&c.to_uppercase()));
    if let (Some(rates_map), Some(target_code), Some(exp_cur)) = (rates, target, expense_currency) {
        let exp_upper = exp_cur.to_uppercase();
        if exp_upper == target_code {
            return (amount, target_cur);
        }
        if let Some(&rate) = rates_map.get(exp_upper.as_str()) {
            return (amount / rate, target_cur);
        }
    }
    (amount, original_cur)
}

/// Sum of all expenses converted to `target_cur` and normalised to monthly amounts.
pub fn monthly_total(
    expenses: &[&Expense],
    rates: Option<&HashMap<String, f64>>,
    target: Option<&str>,
    target_cur: Option<&'static iso::Currency>,
) -> f64 {
    expenses
        .iter()
        .filter_map(|e| {
            let amt = e.amount?;
            let interval = e.interval.as_ref()?;
            let (converted, _) =
                convert_amount(amt, e.currency.as_deref(), rates, target, target_cur);
            Some(interval.to_monthly(converted))
        })
        .sum()
}

/// Returns the single shared currency if every expense has the same one, otherwise `None`.
pub fn uniform_currency(expenses: &[(String, Expense)]) -> Option<&'static iso::Currency> {
    let mut cur: Option<&str> = None;
    for (_, e) in expenses {
        let c = e.currency.as_deref()?;
        match cur {
            None => cur = Some(c),
            Some(prev) if prev.eq_ignore_ascii_case(c) => {}
            _ => return None,
        }
    }
    cur.and_then(|c| iso::Currency::find(&c.to_uppercase()))
}

#[derive(Args, Debug, Default)]
pub struct ExpenseInput {
    /// Expense name
    #[arg(short, long)]
    pub name: Option<String>,
    /// Amount (e.g. 9.99)
    #[arg(short, long)]
    pub amount: Option<f64>,
    /// ISO 4217 currency code (e.g. usd, eur)
    #[arg(short, long)]
    pub currency: Option<String>,
    /// Next due date (YYYY-MM-DD)
    #[arg(short, long)]
    pub date: Option<NaiveDate>,
    /// Billing interval
    #[arg(short, long)]
    pub interval: Option<Interval>,
    /// Category label (e.g. streaming, utilities)
    #[arg(short = 'C', long)]
    pub category: Option<String>,
}
