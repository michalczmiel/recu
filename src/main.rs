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
        Commands::Ls => match storage::list() {
            Ok(expenses) if expenses.is_empty() => {
                println!("No recurring expenses found.");
            }
            Ok(expenses) => {
                for (name, expense) in &expenses {
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
                    println!("  {} {} {} {}", name, amount, currency, tags);
                }
            }
            Err(e) => eprintln!("Error listing expenses: {}", e),
        },
        Commands::Add(args) => add::execute(args),
    }
}
