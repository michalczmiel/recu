use clap::Args;

use crate::expense::validate_name;
use crate::store::Store;

#[derive(Args, Debug)]
#[command(after_help = "Examples:
  recu rename @1 \"Netflix Plus\"
  recu rename Netflix \"Netflix Plus\"")]
pub struct RenameArgs {
    /// Expense to rename: @id or name (case-insensitive)
    pub target: String,
    /// New name
    pub new_name: String,
}

pub fn execute(args: &RenameArgs, store: &Store) -> std::io::Result<()> {
    validate_name(&args.new_name)?;
    store.rename(&args.target, &args.new_name)?;
    println!("Renamed '{}' to '{}'", args.target, args.new_name);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support;
    use crate::test_support::seed_basic as seed_expenses;

    #[test]
    fn rename_rejects_blank_name() {
        let store = test_support::store();
        seed_expenses(&store);
        let err = execute(
            &RenameArgs {
                target: "Netflix".into(),
                new_name: "  ".into(),
            },
            &store,
        )
        .expect_err("should reject blank name");
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    }
}
