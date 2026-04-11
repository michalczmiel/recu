use chrono::NaiveDate;
use clap::Args;
use rusty_money::iso;

#[derive(Args, Debug)]
pub struct AddArgs {
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
    pub interval: Option<crate::storage::Interval>,

    /// Positional args parsed implicitly by format
    pub args: Vec<String>,
}

pub struct ParsedExpense {
    pub name: Option<String>,
    pub amount: Option<f64>,
    pub tags: Option<Vec<String>>,
    pub currency: Option<String>,
    pub first_payment_date: Option<NaiveDate>,
    pub interval: Option<crate::storage::Interval>,
}

impl From<AddArgs> for ParsedExpense {
    fn from(add: AddArgs) -> Self {
        let implicit = parse_implicit_args(&add.args);

        ParsedExpense {
            name: add.name.or(implicit.name),
            amount: add.amount.or(implicit.amount),
            tags: match (add.tags, implicit.tags) {
                (Some(mut f), Some(i)) => {
                    f.extend(i);
                    Some(f)
                }
                (a, b) => a.or(b),
            },
            currency: add.currency.map(|c| c.to_lowercase()).or(implicit.currency),
            first_payment_date: add.date.or(implicit.first_payment_date),
            interval: add.interval.or(implicit.interval),
        }
    }
}

fn parse_interval(s: &str) -> Option<crate::storage::Interval> {
    match s.to_lowercase().as_str() {
        "weekly" => Some(crate::storage::Interval::Weekly),
        "monthly" => Some(crate::storage::Interval::Monthly),
        "quarterly" => Some(crate::storage::Interval::Quarterly),
        "yearly" => Some(crate::storage::Interval::Yearly),
        _ => None,
    }
}

fn is_currency(s: &str) -> bool {
    iso::find(&s.to_uppercase()).is_some()
}

fn parse_implicit_args(args: &[String]) -> ParsedExpense {
    let mut expense = ParsedExpense {
        name: None,
        amount: None,
        tags: None,
        currency: None,
        first_payment_date: None,
        interval: None,
    };
    let mut name_parts: Vec<&str> = Vec::new();

    for arg in args {
        if expense.interval.is_none() {
            if let Some(iv) = parse_interval(arg) {
                expense.interval = Some(iv);
                continue;
            }
        }
        if expense.first_payment_date.is_none() {
            if let Ok(d) = NaiveDate::parse_from_str(arg, "%Y-%m-%d") {
                expense.first_payment_date = Some(d);
                continue;
            }
        }
        if let Some(tag) = arg.strip_prefix('#') {
            expense
                .tags
                .get_or_insert_with(Vec::new)
                .push(tag.to_string());
            continue;
        }
        if expense.currency.is_none() && is_currency(arg) {
            expense.currency = Some(arg.to_lowercase());
            continue;
        }
        if expense.amount.is_none() {
            if let Ok(val) = arg.replace(',', ".").parse::<f64>() {
                expense.amount = Some(val);
                continue;
            }
        }
        name_parts.push(arg);
    }

    if !name_parts.is_empty() {
        expense.name = Some(name_parts.join(" "));
    }

    expense
}

pub fn execute(add: AddArgs) -> std::io::Result<()> {
    let parsed = ParsedExpense::from(add);

    let name = parsed.name.ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "name is required")
    })?;

    let expense = crate::storage::Expense {
        amount: parsed.amount,
        currency: parsed.currency,
        tags: parsed.tags,
        first_payment_date: parsed.first_payment_date,
        interval: parsed.interval,
    };

    let path = crate::storage::save(&name, &expense)?;
    println!("Saved: {}", path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn implicit(strs: &[&str]) -> ParsedExpense {
        let args: Vec<String> = strs.iter().map(|s| s.to_string()).collect();
        parse_implicit_args(&args)
    }

    fn add_args(flags: AddArgs) -> ParsedExpense {
        ParsedExpense::from(flags)
    }

    fn default_add() -> AddArgs {
        AddArgs {
            name: None,
            amount: None,
            tags: None,
            currency: None,
            date: None,
            interval: None,
            args: vec![],
        }
    }

    fn date(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn parses_all_fields_implicitly() {
        let expense = implicit(&["Netflix", "9.99", "#entertainment", "usd", "2024-01-15"]);
        assert_eq!(expense.name.as_deref(), Some("Netflix"));
        assert_eq!(expense.amount, Some(9.99));
        assert_eq!(expense.tags, Some(vec!["entertainment".to_string()]));
        assert_eq!(expense.currency.as_deref(), Some("usd"));
        assert_eq!(expense.first_payment_date, Some(date("2024-01-15")));
    }

    #[test]
    fn parses_name_and_amount_only() {
        let expense = implicit(&["Gym", "50"]);
        assert_eq!(expense.name.as_deref(), Some("Gym"));
        assert_eq!(expense.amount, Some(50.0));
        assert_eq!(expense.tags, None);
        assert_eq!(expense.currency, None);
        assert_eq!(expense.first_payment_date, None);
    }

    #[test]
    fn joins_multi_word_name() {
        let expense = implicit(&["NY", "Times", "15.99"]);
        assert_eq!(expense.name.as_deref(), Some("NY Times"));
        assert_eq!(expense.amount, Some(15.99));
    }

    #[test]
    fn args_order_does_not_matter() {
        let expense = implicit(&["2024-06-01", "#music", "EUR", "9.99", "Spotify"]);
        assert_eq!(expense.name.as_deref(), Some("Spotify"));
        assert_eq!(expense.amount, Some(9.99));
        assert_eq!(expense.tags, Some(vec!["music".to_string()]));
        assert_eq!(expense.currency.as_deref(), Some("eur"));
        assert_eq!(expense.first_payment_date, Some(date("2024-06-01")));
    }

    #[test]
    fn currency_is_case_insensitive() {
        let expense = implicit(&["Test", "USD"]);
        assert_eq!(expense.currency.as_deref(), Some("usd"));

        let expense = implicit(&["Test", "eur"]);
        assert_eq!(expense.currency.as_deref(), Some("eur"));
    }

    #[test]
    fn tags_strip_hash() {
        let expense = implicit(&["Test", "#bills"]);
        assert_eq!(expense.tags, Some(vec!["bills".to_string()]));
    }

    #[test]
    fn multiple_tags() {
        let expense = implicit(&["Netflix", "9.99", "#entertainment", "#streaming"]);
        assert_eq!(
            expense.tags,
            Some(vec!["entertainment".to_string(), "streaming".to_string()])
        );
    }

    #[test]
    fn decimal_amount() {
        let expense = implicit(&["Test", "49.99"]);
        assert_eq!(expense.amount, Some(49.99));
    }

    #[test]
    fn comma_decimal_separator() {
        let expense = implicit(&["Test", "49,99"]);
        assert_eq!(expense.amount, Some(49.99));
    }

    #[test]
    fn name_only() {
        let expense = implicit(&["Netflix"]);
        assert_eq!(expense.name.as_deref(), Some("Netflix"));
        assert_eq!(expense.amount, None);
    }

    #[test]
    fn flags_override_implicit_args() {
        let expense = add_args(AddArgs {
            name: Some("Override".into()),
            currency: Some("GBP".into()),
            interval: None,
            args: vec!["Netflix".into(), "9.99".into(), "usd".into()],
            ..default_add()
        });
        assert_eq!(expense.name.as_deref(), Some("Override"));
        assert_eq!(expense.amount, Some(9.99));
        assert_eq!(expense.currency.as_deref(), Some("gbp"));
    }

    #[test]
    fn flags_only() {
        let expense = add_args(AddArgs {
            name: Some("Spotify".into()),
            amount: Some(9.99),
            tags: Some(vec!["music".into()]),
            currency: Some("EUR".into()),
            date: Some(date("2024-01-15")),
            interval: None,
            args: vec![],
        });
        assert_eq!(expense.name.as_deref(), Some("Spotify"));
        assert_eq!(expense.amount, Some(9.99));
        assert_eq!(expense.tags, Some(vec!["music".to_string()]));
        assert_eq!(expense.currency.as_deref(), Some("eur"));
        assert_eq!(expense.first_payment_date, Some(date("2024-01-15")));
    }

    #[test]
    fn flags_fill_gaps_in_implicit() {
        let expense = add_args(AddArgs {
            currency: Some("PLN".into()),
            args: vec!["Netflix".into(), "9.99".into()],
            ..default_add()
        });
        assert_eq!(expense.name.as_deref(), Some("Netflix"));
        assert_eq!(expense.amount, Some(9.99));
        assert_eq!(expense.currency.as_deref(), Some("pln"));
    }
}
