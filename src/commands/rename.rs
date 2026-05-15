use clap::Args;

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
    store.rename(&args.target, &args.new_name)?;
    println!("Renamed '{}' to '{}'", args.target, args.new_name);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support;
    use crate::test_support::seed_basic as seed_expenses;

    fn names(store: &Store) -> Vec<String> {
        store
            .list()
            .expect("list should succeed")
            .into_iter()
            .map(|e| e.name)
            .collect()
    }

    #[test]
    fn rename_by_name() {
        let store = test_support::store();
        seed_expenses(&store);
        execute(
            &RenameArgs {
                target: "Netflix".into(),
                new_name: "Netflix Plus".into(),
            },
            &store,
        )
        .expect("rename should succeed");
        let n = names(&store);
        assert!(n.contains(&"Netflix Plus".to_string()));
        assert!(!n.contains(&"Netflix".to_string()));
    }

    #[test]
    fn rename_by_id() {
        let store = test_support::store();
        seed_expenses(&store);
        execute(
            &RenameArgs {
                target: "@1".into(),
                new_name: "Netflix Plus".into(),
            },
            &store,
        )
        .expect("rename should succeed");
        assert!(names(&store).contains(&"Netflix Plus".to_string()));
    }

    #[test]
    fn rename_to_existing_name_errors() {
        let store = test_support::store();
        seed_expenses(&store);
        assert!(
            execute(
                &RenameArgs {
                    target: "Netflix".into(),
                    new_name: "Spotify".into(),
                },
                &store,
            )
            .is_err()
        );
    }

    #[test]
    fn rename_nonexistent_errors() {
        let store = test_support::store();
        seed_expenses(&store);
        assert!(
            execute(
                &RenameArgs {
                    target: "Hulu".into(),
                    new_name: "Foo".into(),
                },
                &store,
            )
            .is_err()
        );
    }
}
