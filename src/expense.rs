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

impl std::fmt::Display for Interval {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

impl Interval {
    pub fn name(&self) -> &'static str {
        match self {
            Interval::Weekly => "weekly",
            Interval::Monthly => "monthly",
            Interval::Quarterly => "quarterly",
            Interval::Yearly => "yearly",
        }
    }

    pub fn short(&self) -> &'static str {
        match self {
            Interval::Weekly => "wk",
            Interval::Monthly => "mo",
            Interval::Quarterly => "qtr",
            Interval::Yearly => "yr",
        }
    }

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
            .unwrap_or(today);
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

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
pub struct Expense {
    pub id: u64,
    pub name: String,
    pub amount: Option<f64>,
    pub currency: Option<String>,
    pub start_date: Option<NaiveDate>,
    pub interval: Option<Interval>,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub end_date: Option<NaiveDate>,
}

impl Expense {
    pub fn next_payment(&self, today: NaiveDate) -> Option<NaiveDate> {
        let first = self.start_date?;
        let interval = self.interval.as_ref()?;
        Some(interval.next_payment(first, today))
    }

    pub fn days_until_next(&self, today: NaiveDate) -> Option<i64> {
        Some((self.next_payment(today)? - today).num_days())
    }

    pub fn days_until_end(&self, today: NaiveDate) -> Option<i64> {
        Some((self.end_date? - today).num_days())
    }

    pub fn is_ended(&self, today: NaiveDate) -> bool {
        self.days_until_end(today).is_some_and(|d| d < 0)
    }

    pub fn summary(&self) -> String {
        let parts: Vec<String> = [
            self.amount
                .map(|a| format_expense_amount(self.currency.as_deref(), a)),
            self.interval.as_ref().map(ToString::to_string),
        ]
        .into_iter()
        .flatten()
        .collect();
        if parts.is_empty() {
            self.name.clone()
        } else {
            format!("{}: {}", self.name, parts.join(", "))
        }
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
    fn format_amount_symbol_first() {
        let cur = iso::Currency::find("USD").expect("USD is a valid currency code");
        assert_eq!(format_amount(cur, 42.5), "$42.50");
    }

    #[test]
    fn format_amount_symbol_last() {
        let cur = iso::Currency::find("PLN").expect("PLN is a valid currency code");
        assert_eq!(format_amount(cur, 42.5), "42.50 zł");
    }

    #[test]
    fn format_amount_normalizes_negative_zero() {
        // Rust's empty f64 sum yields -0.0, which `{:.2}` renders as "-0.00".
        let usd = iso::Currency::find("USD").expect("USD is a valid currency code");
        let pln = iso::Currency::find("PLN").expect("PLN is a valid currency code");
        assert_eq!(format_amount(usd, -0.0), "$0.00");
        assert_eq!(format_amount(pln, -0.0), "0.00 zł");
        assert_eq!(format_expense_amount(None, -0.0), "0.00");
        assert_eq!(format_expense_amount(Some("usd"), -0.0), "$0.00");
    }

    fn assert_parses(input: &str, expected: f64) {
        let got = parse_amount(input).expect("parse_amount should succeed");
        assert!(
            (got - expected).abs() < 1e-9,
            "{input} -> {got}, expected {expected}"
        );
    }

    #[test]
    fn parse_amount_plain_decimal() {
        assert_parses("9.99", 9.99);
        assert_parses("9,99", 9.99);
    }

    #[test]
    fn parse_amount_us_format() {
        assert_parses("1,234.56", 1234.56);
        assert_parses("1,234,567.89", 1_234_567.89);
    }

    #[test]
    fn parse_amount_european_format() {
        assert_parses("1.234,56", 1234.56);
        assert_parses("1.234.567,89", 1_234_567.89);
    }

    #[test]
    fn parse_amount_thousands_only() {
        assert_parses("1,234", 1234.0);
        assert_parses("1,234,567", 1_234_567.0);
    }

    #[test]
    fn parse_amount_trims_whitespace() {
        assert_parses("  42.50  ", 42.50);
    }

    #[test]
    fn parse_amount_rejects_invalid() {
        assert!(parse_amount("").is_err());
        assert!(parse_amount("abc").is_err());
        assert!(parse_amount("0").is_err());
        assert!(parse_amount("-1").is_err());
        assert!(parse_amount("inf").is_err());
        assert!(parse_amount("NaN").is_err());
    }

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).expect("valid date")
    }

    #[test]
    fn days_until_end_signed_or_none() {
        let today = d(2026, 4, 15);
        assert_eq!(Expense::default().days_until_end(today), None);
        let future = Expense {
            end_date: Some(d(2026, 5, 15)),
            ..Default::default()
        };
        let past = Expense {
            end_date: Some(d(2026, 4, 10)),
            ..Default::default()
        };
        assert_eq!(future.days_until_end(today), Some(30));
        assert_eq!(past.days_until_end(today), Some(-5));
    }

    #[test]
    fn is_ended_only_when_end_in_past() {
        let today = d(2026, 4, 15);
        let same_day = Expense {
            end_date: Some(today),
            ..Default::default()
        };
        let past = Expense {
            end_date: Some(d(2026, 4, 1)),
            ..Default::default()
        };
        assert!(!Expense::default().is_ended(today));
        assert!(!same_day.is_ended(today));
        assert!(past.is_ended(today));
    }

    #[test]
    fn recurring_totals_compute() {
        let monthly = Expense {
            amount: Some(10.0),
            currency: Some("USD".to_string()),
            interval: Some(Interval::Monthly),
            ..Default::default()
        };
        let yearly = Expense {
            amount: Some(120.0),
            currency: Some("USD".to_string()),
            interval: Some(Interval::Yearly),
            ..Default::default()
        };
        let totals = RecurringTotals::compute([&monthly, &yearly], None, Some("USD"));

        assert!((totals.monthly - 20.0).abs() < 1e-9);
        assert!((totals.yearly - 240.0).abs() < 1e-9);
    }
}

/// Round to 2 decimals — strips f64 accumulation drift from JSON output.
pub fn round_money(n: f64) -> f64 {
    (n * 100.0).round() / 100.0
}

pub fn format_amount(cur: &iso::Currency, amount: f64) -> String {
    // `{:.2}` renders -0.0 as "-0.00"; normalize so empty sums and tiny
    // negative rounding artifacts don't print a stray minus.
    let amount = amount + 0.0;
    if cur.symbol_first {
        format!("{}{:.2}", cur.symbol, amount)
    } else {
        format!("{:.2} {}", amount, cur.symbol)
    }
}

/// Format an amount with optional currency code, falling back to plain `{:.2}`.
pub fn format_expense_amount(currency: Option<&str>, amount: f64) -> String {
    match currency.and_then(find_currency) {
        Some(c) => format_amount(c, amount),
        None => format!("{:.2}", amount + 0.0),
    }
}

/// Look up an ISO 4217 currency by code (case-insensitive).
pub fn find_currency(code: &str) -> Option<&'static iso::Currency> {
    iso::Currency::find(&code.to_ascii_uppercase())
}

pub fn convert(
    amount: f64,
    expense_currency: Option<&str>,
    rates: Option<&HashMap<String, f64>>,
    target: Option<&str>,
) -> f64 {
    if let (Some(rates_map), Some(target_code), Some(exp_cur)) = (rates, target, expense_currency) {
        let exp_upper = exp_cur.to_ascii_uppercase();
        if exp_upper == target_code {
            return amount;
        }
        if let Some(&rate) = rates_map.get(exp_upper.as_str()) {
            return amount / rate;
        }
    }
    amount
}

pub fn display_currency(
    expense_currency: Option<&str>,
    rates: Option<&HashMap<String, f64>>,
    target: Option<&str>,
    target_cur: Option<&'static iso::Currency>,
) -> Option<&'static iso::Currency> {
    if let (Some(rates_map), Some(target_code), Some(exp_cur)) = (rates, target, expense_currency) {
        let exp_upper = exp_cur.to_ascii_uppercase();
        if exp_upper == target_code || rates_map.contains_key(exp_upper.as_str()) {
            return target_cur;
        }
    }
    expense_currency.and_then(find_currency)
}

pub struct RecurringTotals {
    pub monthly: f64,
    pub yearly: f64,
}

impl RecurringTotals {
    pub fn compute<'a>(
        expenses: impl IntoIterator<Item = &'a Expense>,
        rates: Option<&HashMap<String, f64>>,
        target: Option<&str>,
    ) -> Self {
        let monthly: f64 = expenses
            .into_iter()
            .filter_map(|e| {
                let amt = e.amount?;
                let interval = e.interval.as_ref()?;
                let converted = convert(amt, e.currency.as_deref(), rates, target);
                Some(interval.to_monthly(converted))
            })
            .sum();

        Self {
            monthly,
            yearly: monthly * 12.0,
        }
    }
}

/// `true` if `expense`'s category matches any of `filters` (case-insensitive).
/// Empty `filters` matches everything.
pub fn matches_categories(expense: &Expense, filters: &[String]) -> bool {
    if filters.is_empty() {
        return true;
    }
    expense
        .category
        .as_deref()
        .is_some_and(|c| filters.iter().any(|f| f.eq_ignore_ascii_case(c)))
}

/// Returns the single shared currency if every expense has the same one, otherwise `None`.
pub fn uniform_currency(expenses: &[Expense]) -> Option<&'static iso::Currency> {
    let mut cur: Option<&str> = None;
    for e in expenses {
        let c = e.currency.as_deref()?;
        match cur {
            None => cur = Some(c),
            Some(prev) if prev.eq_ignore_ascii_case(c) => {}
            _ => return None,
        }
    }
    cur.and_then(find_currency)
}

/// Curated list of currencies supported by exchange-rate fetching and prompts.
/// Exposed here so flag-parse errors can enumerate valid choices.
pub const VALID_CURRENCIES: &[&str] = &[
    "ars", "aud", "bdt", "brl", "cad", "chf", "clp", "cny", "cop", "czk", "dkk", "egp", "eur",
    "gbp", "hkd", "huf", "idr", "ils", "inr", "jpy", "kes", "krw", "mxn", "myr", "ngn", "nok",
    "nzd", "pen", "php", "pkr", "pln", "ron", "rub", "sek", "sgd", "thb", "try", "uah", "usd",
    "vnd", "zar",
];

pub fn normalize_currency(s: &str) -> Result<String, String> {
    let lower = s.trim().to_lowercase();
    if find_currency(&lower).is_none() {
        return Err(format!(
            "invalid currency \"{s}\"; valid: {}\nexample: recu add --name Netflix --amount 9.99 --currency usd --interval monthly",
            VALID_CURRENCIES.join(", ")
        ));
    }
    Ok(lower)
}

pub fn parse_amount(s: &str) -> Result<f64, String> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return Err("amount cannot be empty".into());
    }
    let normalized = normalize_amount(trimmed);
    let v = normalized
        .parse::<f64>()
        .map_err(|_| format!("'{s}' is not a valid amount"))?;
    if !v.is_finite() {
        return Err("amount must be finite".into());
    }
    if v <= 0.0 {
        return Err("amount must be greater than 0".into());
    }
    Ok(v)
}

/// Normalize thousands/decimal separators so both `1,234.56` and `1.234,56` parse.
///
/// Heuristic: the last `,` or `.` is the decimal point iff it's followed by 1-2 digits;
/// otherwise every separator is treated as a thousands grouping and stripped.
fn normalize_amount(s: &str) -> String {
    let Some(pos) = s.rfind([',', '.']) else {
        return s.to_string();
    };
    let after = &s[pos + 1..];
    let is_decimal =
        !after.is_empty() && after.len() <= 2 && after.chars().all(|c| c.is_ascii_digit());
    if is_decimal {
        let head: String = s[..pos]
            .chars()
            .filter(|c| *c != ',' && *c != '.')
            .collect();
        format!("{head}.{after}")
    } else {
        s.chars().filter(|c| *c != ',' && *c != '.').collect()
    }
}

#[derive(Args, Debug, Default, PartialEq)]
pub struct ExpenseFields {
    /// Amount (e.g. 9.99 or 9,99)
    #[arg(short, long, value_parser = parse_amount)]
    pub amount: Option<f64>,
    /// ISO 4217 currency code (e.g. usd, eur)
    #[arg(short, long, value_parser = normalize_currency)]
    pub currency: Option<String>,
    /// Start date (YYYY-MM-DD)
    #[arg(short, long)]
    pub date: Option<NaiveDate>,
    /// Billing interval
    #[arg(short, long)]
    pub interval: Option<Interval>,
    /// Category label (e.g. streaming, utilities)
    #[arg(long = "category")]
    pub category: Option<String>,
    /// End date — when the subscription stops (YYYY-MM-DD)
    #[arg(long = "end")]
    pub end_date: Option<NaiveDate>,
}

impl From<&ExpenseFields> for Expense {
    fn from(f: &ExpenseFields) -> Self {
        Expense {
            amount: f.amount,
            currency: f.currency.clone(),
            start_date: f.date,
            interval: f.interval.clone(),
            category: f.category.clone(),
            end_date: f.end_date,
            ..Default::default()
        }
    }
}

#[derive(Args, Debug, Default)]
pub struct ExpenseInput {
    /// Expense name
    #[arg(value_name = "NAME")]
    pub name: Option<String>,
    #[command(flatten)]
    pub fields: ExpenseFields,
}
