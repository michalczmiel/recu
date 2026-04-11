use clap::{Parser, Subcommand};

mod add;
mod edit;
mod expense;
mod rm;
mod storage;

#[derive(Parser, Debug)]
#[command(name = "recu", version, about = "Track recurring expenses")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// List recurring expenses
    Ls,
    /// Add a recurring expense
    Add(add::AddArgs),
    /// Edit a recurring expense
    Edit(edit::EditArgs),
    /// Remove a recurring expense
    Rm(rm::RmArgs),
}

fn main() -> std::io::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        None | Some(Commands::Ls) => {
            let expenses = storage::list()?;
            if expenses.is_empty() {
                println!("No recurring expenses found.");
                return Ok(());
            }
            let today = chrono::Local::now().date_naive();
            for (index, (name, expense)) in expenses.iter().enumerate() {
                let amount = expense.amount.map(|a| a.to_string()).unwrap_or("-".into());
                let currency = expense.currency.as_deref().unwrap_or("");
                let tags = expense
                    .tags
                    .as_ref()
                    .map(|t| {
                        t.iter()
                            .map(|t| format!("#{}", t))
                            .collect::<Vec<_>>()
                            .join(" ")
                    })
                    .unwrap_or_default();
                let days_str = expense
                    .days_until_next(today)
                    .map(|days| {
                        if days == 0 {
                            "today".to_string()
                        } else {
                            format!("in {} days", days)
                        }
                    })
                    .unwrap_or_default();
                println!(
                    "@{} {} {} {} {} {}",
                    index + 1,
                    name,
                    amount,
                    currency,
                    tags,
                    days_str
                );
            }
        }
        Some(Commands::Add(args)) => add::execute(args)?,
        Some(Commands::Edit(args)) => edit::execute(args)?,
        Some(Commands::Rm(args)) => rm::execute(args)?,
    }
    Ok(())
}
