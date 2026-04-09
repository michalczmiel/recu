use clap::{Parser, Subcommand};

mod add;
mod rm;
mod storage;

#[derive(Parser, Debug)]
#[command(name = "recu")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// List recurring expenses
    Ls,
    /// Add a recurring expense
    Add(add::AddArgs),
    /// Remove a recurring expense
    Rm(rm::RmArgs),
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Ls => match storage::list() {
            Ok(expenses) if expenses.is_empty() => {
                println!("No recurring expenses found.");
            }
            Ok(expenses) => {
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
                    println!("@{} {} {} {} {}", index + 1, name, amount, currency, tags);
                }
            }
            Err(e) => eprintln!("Error listing expenses: {}", e),
        },
        Commands::Add(args) => add::execute(args),
        Commands::Rm(args) => rm::execute(args),
    }
}
