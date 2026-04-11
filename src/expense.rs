use chrono::{Datelike, NaiveDate};
use clap::{Args, ValueEnum};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Interval {
    Weekly,
    Monthly,
    Quarterly,
    Yearly,
}

impl Interval {
    pub fn next_payment(&self, first: NaiveDate, today: NaiveDate) -> NaiveDate {
        match self {
            Interval::Weekly => {
                let days_since = (today - first).num_days().rem_euclid(7);
                if days_since == 0 {
                    today
                } else {
                    today + chrono::Days::new((7 - days_since) as u64)
                }
            }
            Interval::Monthly => advance_months(first, today, 1),
            Interval::Quarterly => advance_months(first, today, 3),
            Interval::Yearly => advance_months(first, today, 12),
        }
    }
}

fn advance_months(first: NaiveDate, today: NaiveDate, step: u32) -> NaiveDate {
    let diff = (today.year() - first.year()) * 12 + (today.month() as i32 - first.month() as i32);
    let mut k = (diff.max(0) as u32 / step) * step;
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

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Expense {
    pub amount: Option<f64>,
    pub currency: Option<String>,
    pub tags: Option<Vec<String>>,
    pub first_payment_date: Option<NaiveDate>,
    pub interval: Option<Interval>,
}

impl Expense {
    pub fn next_payment(&self, today: NaiveDate) -> Option<NaiveDate> {
        let first = self.first_payment_date?;
        let interval = self.interval.as_ref()?;
        Some(interval.next_payment(first, today))
    }

    pub fn days_until_next(&self, today: NaiveDate) -> Option<i64> {
        Some((self.next_payment(today)? - today).num_days())
    }
}

#[derive(Args, Debug, Default)]
pub struct ExpenseFields {
    /// Expense name
    #[arg(long)]
    pub name: Option<String>,
    /// Amount (e.g. 9.99)
    #[arg(long)]
    pub amount: Option<f64>,
    /// Tags (e.g. --tags entertainment,streaming)
    #[arg(long, value_delimiter = ',')]
    pub tags: Option<Vec<String>>,
    /// ISO 4217 currency code (e.g. usd, eur)
    #[arg(long)]
    pub currency: Option<String>,
    /// First payment date (YYYY-MM-DD)
    #[arg(long)]
    pub date: Option<NaiveDate>,
    /// Billing interval
    #[arg(long)]
    pub interval: Option<Interval>,
}
