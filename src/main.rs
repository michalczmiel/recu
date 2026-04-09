use clap::{Parser, Subcommand};

mod add;
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
    /// Remove a recurring expense
    Rm(rm::RmArgs),
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        None | Some(Commands::Ls) => match storage::list() {
            Ok(expenses) if expenses.is_empty() => {
                println!("No recurring expenses found.");
            }
            Ok(expenses) => {
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
                    let days_left = expense
                        .first_payment_date
                        .as_ref()
                        .and_then(|d| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok())
                        .zip(expense.interval.as_ref())
                        .map(|(first, interval)| {
                            let next = interval.next_payment(first, today);
                            let days = (next - today).num_days();
                            if days == 0 {
                                "today".to_string()
                            } else {
                                format!("in {} days", days)
                            }
                        });
                    let days_str = days_left.as_deref().unwrap_or("");
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
            Err(e) => eprintln!("Error listing expenses: {}", e),
        },
        Some(Commands::Add(args)) => add::execute(args),
        Some(Commands::Rm(args)) => rm::execute(args),
    }
}
