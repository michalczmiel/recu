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

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Expense {
    pub amount: Option<f64>,
    pub currency: Option<String>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interval_aliases() {
        assert_eq!("week".parse::<Interval>().unwrap(), Interval::Weekly);
        assert_eq!("month".parse::<Interval>().unwrap(), Interval::Monthly);
        assert_eq!("quarter".parse::<Interval>().unwrap(), Interval::Quarterly);
        assert_eq!("year".parse::<Interval>().unwrap(), Interval::Yearly);
        assert_eq!("annual".parse::<Interval>().unwrap(), Interval::Yearly);
        assert_eq!("annually".parse::<Interval>().unwrap(), Interval::Yearly);
        assert_eq!("YEARLY".parse::<Interval>().unwrap(), Interval::Yearly);
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
