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
    use crate::expense::Expense;
    use std::fs;

    fn make_store(test_name: &str) -> Store {
        let file = std::env::temp_dir()
            .join("recu-test-rename")
            .join(format!("{test_name}.csv"));
        let _ = fs::remove_file(&file);
        Store::at(file)
    }

    fn seed(store: &Store) {
        for name in ["Netflix", "Spotify"] {
            store
                .save(&Expense {
                    name: name.to_string(),
                    ..Default::default()
                })
                .expect("seed save should succeed");
        }
    }

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
        let store = make_store("rename-by-name");
        seed(&store);
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
        let store = make_store("rename-by-id");
        seed(&store);
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
        let store = make_store("rename-conflict");
        seed(&store);
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
        let store = make_store("rename-nonexistent");
        seed(&store);
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
