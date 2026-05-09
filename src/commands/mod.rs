use chrono::NaiveDate;
use clap::ValueEnum;
use serde::Serialize;

use crate::expense::{Expense, Interval};

pub mod add;
pub mod calendar;
pub mod category;
pub mod config;
pub mod edit;
pub mod list;
pub mod rename;
pub mod rm;
pub mod treemap;
pub mod undo;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
}

#[derive(Serialize)]
pub(crate) struct JsonExpense<'a> {
    pub id: u64,
    pub name: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub amount: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_date: Option<NaiveDate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub interval: Option<&'a Interval>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub category: Option<&'a str>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_date: Option<NaiveDate>,
}

impl<'a> From<&'a Expense> for JsonExpense<'a> {
    fn from(e: &'a Expense) -> Self {
        Self {
            id: e.id,
            name: &e.name,
            amount: e.amount,
            currency: e.currency.as_deref(),
            start_date: e.start_date,
            interval: e.interval.as_ref(),
            category: e.category.as_deref(),
            end_date: e.end_date,
        }
    }
}
