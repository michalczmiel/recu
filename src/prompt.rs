use crate::expense::{Interval, find_currency, parse_amount};
use chrono::NaiveDate;
use inquire::{
    Autocomplete, CustomType, DateSelect, Select, Text,
    ui::{Color, RenderConfig, Styled},
    validator::Validation,
};

fn is_currency(s: &str) -> bool {
    find_currency(s).is_some()
}

pub fn inquire_err(e: &inquire::InquireError) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Interrupted, e.to_string())
}

pub fn render_config() -> RenderConfig<'static> {
    RenderConfig::default_colored()
        .with_prompt_prefix(Styled::new("›").with_fg(Color::LightCyan))
        .with_answered_prompt_prefix(Styled::new("✓").with_fg(Color::LightGreen))
}

const CURRENCIES: &[&str] = &[
    "ars", "aud", "bdt", "brl", "cad", "chf", "clp", "cny", "cop", "czk", "dkk", "egp", "eur",
    "gbp", "hkd", "huf", "idr", "ils", "inr", "jpy", "kes", "krw", "mxn", "myr", "ngn", "nok",
    "nzd", "pen", "php", "pkr", "pln", "ron", "rub", "sek", "sgd", "thb", "try", "uah", "usd",
    "vnd", "zar",
];

#[derive(Clone)]
struct CurrencyCompleter;

impl Autocomplete for CurrencyCompleter {
    fn get_suggestions(&mut self, input: &str) -> Result<Vec<String>, inquire::CustomUserError> {
        let input_lower = input.to_lowercase();
        Ok(CURRENCIES
            .iter()
            .filter(|c| c.starts_with(input_lower.as_str()))
            .map(|c| (*c).to_string())
            .collect())
    }

    fn get_completion(
        &mut self,
        _input: &str,
        highlighted_suggestion: Option<String>,
    ) -> Result<inquire::autocompletion::Replacement, inquire::CustomUserError> {
        Ok(highlighted_suggestion)
    }
}

const NEW_CATEGORY_SENTINEL: &str = "+ New category...";

/// Prompt for a required name (used by `add`).
pub fn prompt_name(initial: &str) -> std::io::Result<String> {
    Text::new("Name:")
        .with_initial_value(initial)
        .with_validator(|s: &str| {
            if s.trim().is_empty() {
                Ok(Validation::Invalid("Name cannot be empty".into()))
            } else {
                Ok(Validation::Valid)
            }
        })
        .prompt()
        .map_err(|e| inquire_err(&e))
}

/// Prompt for a name that may be skipped. Returns `Some(new)` only when changed (used by `edit`).
pub fn prompt_name_skippable(current: &str) -> std::io::Result<Option<String>> {
    let answer = Text::new("Name:")
        .with_initial_value(current)
        .with_validator(|s: &str| {
            if s.trim().is_empty() {
                Ok(Validation::Invalid("Name cannot be empty".into()))
            } else {
                Ok(Validation::Valid)
            }
        })
        .prompt_skippable()
        .map_err(|e| inquire_err(&e))?;
    Ok(answer.filter(|n| !n.is_empty() && n != current))
}

pub fn prompt_amount(default: Option<f64>) -> std::io::Result<Option<f64>> {
    let amount_parser = |input: &str| -> Result<f64, ()> { parse_amount(input).map_err(|_| ()) };
    let mut prompt = CustomType::<f64>::new("Amount:")
        .with_placeholder("e.g. 9.99 or 9,99")
        .with_parser(&amount_parser)
        .with_error_message("Please enter a positive number");
    if let Some(v) = default {
        prompt = prompt.with_default(v);
    }
    prompt.prompt_skippable().map_err(|e| inquire_err(&e))
}

pub fn prompt_currency(initial: &str) -> std::io::Result<Option<String>> {
    Text::new("Currency (ISO 4217):")
        .with_initial_value(initial)
        .with_placeholder("e.g. usd, eur, gbp")
        .with_autocomplete(CurrencyCompleter)
        .with_validator(|s: &str| {
            if s.is_empty() || is_currency(s) {
                Ok(Validation::Valid)
            } else {
                Ok(Validation::Invalid(
                    "Not a valid ISO 4217 currency code".into(),
                ))
            }
        })
        .prompt_skippable()
        .map_err(|e| inquire_err(&e))
        .map(|opt| opt.filter(|s| !s.is_empty()).map(|s| s.to_lowercase()))
}

pub fn prompt_date(
    label: &str,
    default: Option<NaiveDate>,
) -> std::io::Result<Option<NaiveDate>> {
    let mut prompt = DateSelect::new(label);
    if let Some(d) = default {
        prompt = prompt.with_default(d);
    }
    prompt.prompt_skippable().map_err(|e| inquire_err(&e))
}

pub fn prompt_interval(default: Option<&Interval>) -> std::io::Result<Option<Interval>> {
    let intervals = vec![
        Interval::Weekly,
        Interval::Monthly,
        Interval::Quarterly,
        Interval::Yearly,
    ];
    let starting_cursor = default
        .and_then(|iv| intervals.iter().position(|x| x == iv))
        .unwrap_or(0);
    Select::new("Interval:", intervals)
        .with_starting_cursor(starting_cursor)
        .prompt_skippable()
        .map_err(|e| inquire_err(&e))
}

pub fn prompt_category(
    existing: &[String],
    preselected: Option<&str>,
) -> std::io::Result<Option<String>> {
    if existing.is_empty() {
        return Text::new("Category:")
            .with_initial_value(preselected.unwrap_or(""))
            .prompt_skippable()
            .map_err(|e| inquire_err(&e))
            .map(|opt| opt.filter(|s| !s.is_empty()));
    }

    let mut options: Vec<String> = existing.to_vec();
    options.push(NEW_CATEGORY_SENTINEL.to_string());

    let cursor = preselected
        .and_then(|p| options.iter().position(|o| o == p))
        .unwrap_or(0);

    let chosen = Select::new("Category:", options)
        .with_starting_cursor(cursor)
        .prompt_skippable()
        .map_err(|e| inquire_err(&e))?;

    match chosen.as_deref() {
        None => Ok(None),
        Some(s) if s == NEW_CATEGORY_SENTINEL => Text::new("New category name:")
            .prompt_skippable()
            .map_err(|e| inquire_err(&e))
            .map(|opt| opt.filter(|s| !s.is_empty())),
        Some(_) => Ok(chosen),
    }
}
