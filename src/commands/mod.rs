use std::collections::BTreeMap;
use std::io::{self, Write};

use chrono::NaiveDate;
use clap::Args;
use serde::Serialize;

use crate::expense::{AmountRange, Expense, Interval};

pub mod add;
pub mod calendar;
pub mod category;
pub mod completion;
pub mod config;
pub mod edit;
pub mod list;
pub mod remove;
pub mod rename;
pub mod treemap;
pub mod undo;

/// Shared filtering flags for the list, calendar, and treemap commands.
#[derive(Args, Debug, Default)]
pub struct Filters {
    /// Include ended expenses
    #[arg(short, long)]
    pub all: bool,
    /// Filter by category (case-insensitive); comma-separated for multiple
    #[arg(short, long, value_delimiter = ',')]
    pub category: Vec<String>,
    #[command(flatten)]
    pub amount: AmountRange,
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
    #[serde(flatten, serialize_with = "serialize_non_empty_extra")]
    pub extra: &'a BTreeMap<String, String>,
}

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "signature required by serde serialize_with"
)]
fn serialize_non_empty_extra<S: serde::Serializer>(
    map: &&BTreeMap<String, String>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    use serde::ser::SerializeMap;
    let mut m = serializer.serialize_map(None)?;
    for (k, v) in map.iter().filter(|(_, v)| !v.is_empty()) {
        m.serialize_entry(k, v)?;
    }
    m.end()
}

pub(crate) fn emit_json<T: Serialize>(out: &mut impl Write, value: &T) -> io::Result<()> {
    serde_json::to_writer_pretty(&mut *out, value)?;
    writeln!(out)
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
            extra: &e.extra,
        }
    }
}
