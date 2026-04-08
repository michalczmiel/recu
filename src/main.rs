use clap::{Parser, Subcommand};

mod add;
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
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Ls => {
            match storage::list() {
                Ok(expenses) if expenses.is_empty() => {
                    println!("No recurring expenses found.");
                }
                Ok(expenses) => {
                    for (name, expense) in &expenses {
                        let amount = expense
                            .amount
                            .map(|a| a.to_string())
                            .unwrap_or("-".into());
                        let currency = expense.currency.as_deref().unwrap_or("");
                        let category = expense
                            .category
                            .as_ref()
                            .map(|c| format!("@{}", c))
                            .unwrap_or_default();
                        println!("  {} {} {} {}", name, amount, currency, category);
                    }
                }
                Err(e) => eprintln!("Error listing expenses: {}", e),
            }
        }
        Commands::Add(args) => add::execute(args),
    }
}
