use clap::Args;

use crate::commands::{JsonExpense, OutputFormat};
use crate::expense::{Expense, ExpenseInput};
use crate::prompt::{
    prompt_amount, prompt_category, prompt_currency, prompt_date, prompt_interval, prompt_name,
    render_config,
};
use crate::store::Store;

/// Add a recurring expense.
///
/// Pass --name to skip prompts; any missing fields stay unset (fill in later via
/// 'recu edit'). Without --name, runs interactively.
#[derive(Args, Debug)]
#[command(after_help = "Examples:
  recu add -n Netflix -a 9.99 -c usd -d 2026-05-01 -i monthly
  recu add -n Netflix          # stored with name only, fill in later via 'recu edit'
  recu add          # interactive mode")]
pub struct AddArgs {
    #[command(flatten)]
    pub fields: ExpenseInput,
    /// Output format
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    pub format: OutputFormat,
}

fn prompt_fields(fields: &ExpenseInput, store: &Store) -> std::io::Result<Expense> {
    let name = prompt_name(fields.name.as_deref().unwrap_or(""))?;
    let amount = prompt_amount(fields.amount)?;
    let currency = prompt_currency(fields.currency.as_deref().unwrap_or(""))?;
    let start_date = prompt_date("Start date:", fields.date)?;
    let interval = prompt_interval(fields.interval.as_ref())?;
    let categories = store.categories()?;
    let category = prompt_category(&categories, fields.category.as_deref())?;
    Ok(Expense {
        id: 0,
        name,
        amount,
        currency,
        start_date,
        interval,
        category,
        end_date: None,
    })
}

pub fn execute(add: &AddArgs, store: &Store) -> std::io::Result<()> {
    let f = &add.fields;
    let expense = if let Some(name) = &f.name {
        Expense {
            id: 0,
            name: name.clone(),
            amount: f.amount,
            currency: f.currency.clone(),
            start_date: f.date,
            interval: f.interval.clone(),
            category: f.category.clone(),
            end_date: f.end_date,
        }
    } else {
        inquire::set_global_render_config(render_config());
        prompt_fields(f, store)?
    };

    store.save(&expense)?;

    match add.format {
        OutputFormat::Json => {
            let saved = store.get(&expense.name)?;
            serde_json::to_writer_pretty(std::io::stdout(), &JsonExpense::from(&saved))?;
            println!();
        }
        OutputFormat::Text => println!("Added {}", expense.summary()),
    }
    Ok(())
}
