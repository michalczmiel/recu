use chrono::NaiveDate;
use clap::{Args, ValueEnum};
use rusty_money::iso;

use crate::expense::{Expense, ExpenseFields, Interval};

#[derive(Args, Debug)]
pub struct AddArgs {
    #[command(flatten)]
    pub fields: ExpenseFields,

    /// Positional args parsed implicitly by format
    pub args: Vec<String>,
}

struct ParsedExpense {
    name: Option<String>,
    expense: Expense,
}

fn parse(add: AddArgs) -> ParsedExpense {
    let implicit = parse_implicit_args(&add.args);

    let tags: Option<Vec<String>> = match (add.fields.tags, implicit.expense.tags) {
        (Some(mut f), Some(i)) => {
            f.extend(i);
            Some(f)
        }
        (a, b) => a.or(b),
    };

    ParsedExpense {
        name: add.fields.name.or(implicit.name),
        expense: Expense {
            amount: add.fields.amount.or(implicit.expense.amount),
            currency: add
                .fields
                .currency
                .map(|c| c.to_lowercase())
                .or(implicit.expense.currency),
            tags,
            first_payment_date: add.fields.date.or(implicit.expense.first_payment_date),
            interval: add.fields.interval.or(implicit.expense.interval),
        },
    }
}

fn is_currency(s: &str) -> bool {
    iso::find(&s.to_uppercase()).is_some()
}

fn parse_implicit_args(args: &[String]) -> ParsedExpense {
    let mut expense = Expense::default();
    let mut name_parts: Vec<&str> = Vec::new();

    for arg in args {
        if expense.interval.is_none() {
            if let Ok(iv) = Interval::from_str(arg, true) {
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

    ParsedExpense {
        name: if name_parts.is_empty() {
            None
        } else {
            Some(name_parts.join(" "))
        },
        expense,
    }
}

pub fn execute(add: AddArgs) -> std::io::Result<()> {
    let parsed = parse(add);

    let name = parsed
        .name
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "name is required"))?;

    let path = crate::storage::save(&name, &parsed.expense)?;
    println!("Saved: {}", path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expense::Interval;

    fn implicit(strs: &[&str]) -> ParsedExpense {
        let args: Vec<String> = strs.iter().map(|s| s.to_string()).collect();
        parse_implicit_args(&args)
    }

    fn add_args(flags: AddArgs) -> ParsedExpense {
        parse(flags)
    }

    fn date(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    #[test]
    fn parses_all_fields_implicitly() {
        let p = implicit(&["Netflix", "9.99", "#entertainment", "usd", "2024-01-15"]);
        assert_eq!(p.name.as_deref(), Some("Netflix"));
        assert_eq!(p.expense.amount, Some(9.99));
        assert_eq!(p.expense.tags, Some(vec!["entertainment".to_string()]));
        assert_eq!(p.expense.currency.as_deref(), Some("usd"));
        assert_eq!(p.expense.first_payment_date, Some(date("2024-01-15")));
    }

    #[test]
    fn parses_name_and_amount_only() {
        let p = implicit(&["Gym", "50"]);
        assert_eq!(p.name.as_deref(), Some("Gym"));
        assert_eq!(p.expense.amount, Some(50.0));
        assert_eq!(p.expense.tags, None);
        assert_eq!(p.expense.currency, None);
        assert_eq!(p.expense.first_payment_date, None);
    }

    #[test]
    fn joins_multi_word_name() {
        let p = implicit(&["NY", "Times", "15.99"]);
        assert_eq!(p.name.as_deref(), Some("NY Times"));
        assert_eq!(p.expense.amount, Some(15.99));
    }

    #[test]
    fn args_order_does_not_matter() {
        let p = implicit(&["2024-06-01", "#music", "EUR", "9.99", "Spotify"]);
        assert_eq!(p.name.as_deref(), Some("Spotify"));
        assert_eq!(p.expense.amount, Some(9.99));
        assert_eq!(p.expense.tags, Some(vec!["music".to_string()]));
        assert_eq!(p.expense.currency.as_deref(), Some("eur"));
        assert_eq!(p.expense.first_payment_date, Some(date("2024-06-01")));
    }

    #[test]
    fn currency_is_case_insensitive() {
        let p = implicit(&["Test", "USD"]);
        assert_eq!(p.expense.currency.as_deref(), Some("usd"));

        let p = implicit(&["Test", "eur"]);
        assert_eq!(p.expense.currency.as_deref(), Some("eur"));
    }

    #[test]
    fn tags_strip_hash() {
        let p = implicit(&["Test", "#bills"]);
        assert_eq!(p.expense.tags, Some(vec!["bills".to_string()]));
    }

    #[test]
    fn multiple_tags() {
        let p = implicit(&["Netflix", "9.99", "#entertainment", "#streaming"]);
        assert_eq!(
            p.expense.tags,
            Some(vec!["entertainment".to_string(), "streaming".to_string()])
        );
    }

    #[test]
    fn decimal_amount() {
        let p = implicit(&["Test", "49.99"]);
        assert_eq!(p.expense.amount, Some(49.99));
    }

    #[test]
    fn comma_decimal_separator() {
        let p = implicit(&["Test", "49,99"]);
        assert_eq!(p.expense.amount, Some(49.99));
    }

    #[test]
    fn name_only() {
        let p = implicit(&["Netflix"]);
        assert_eq!(p.name.as_deref(), Some("Netflix"));
        assert_eq!(p.expense.amount, None);
    }

    fn fields(
        name: Option<&str>,
        amount: Option<f64>,
        tags: Option<Vec<&str>>,
        currency: Option<&str>,
        date: Option<NaiveDate>,
        interval: Option<Interval>,
    ) -> ExpenseFields {
        ExpenseFields {
            name: name.map(Into::into),
            amount,
            tags: tags.map(|t| t.into_iter().map(Into::into).collect()),
            currency: currency.map(Into::into),
            date,
            interval,
        }
    }

    #[test]
    fn flags_override_implicit_args() {
        let p = add_args(AddArgs {
            fields: fields(Some("Override"), None, None, Some("GBP"), None, None),
            args: vec!["Netflix".into(), "9.99".into(), "usd".into()],
        });
        assert_eq!(p.name.as_deref(), Some("Override"));
        assert_eq!(p.expense.amount, Some(9.99));
        assert_eq!(p.expense.currency.as_deref(), Some("gbp"));
    }

    #[test]
    fn flags_only() {
        let p = add_args(AddArgs {
            fields: fields(
                Some("Spotify"),
                Some(9.99),
                Some(vec!["music"]),
                Some("EUR"),
                Some(date("2024-01-15")),
                None,
            ),
            args: vec![],
        });
        assert_eq!(p.name.as_deref(), Some("Spotify"));
        assert_eq!(p.expense.amount, Some(9.99));
        assert_eq!(p.expense.tags, Some(vec!["music".to_string()]));
        assert_eq!(p.expense.currency.as_deref(), Some("eur"));
        assert_eq!(p.expense.first_payment_date, Some(date("2024-01-15")));
    }

    #[test]
    fn flags_fill_gaps_in_implicit() {
        let p = add_args(AddArgs {
            fields: fields(None, None, None, Some("PLN"), None, None),
            args: vec!["Netflix".into(), "9.99".into()],
        });
        assert_eq!(p.name.as_deref(), Some("Netflix"));
        assert_eq!(p.expense.amount, Some(9.99));
        assert_eq!(p.expense.currency.as_deref(), Some("pln"));
    }
}
