use std::io;

use clap::{Args, Subcommand};

use crate::store::Store;

#[derive(Subcommand, Debug)]
pub enum CategoryCommand {
    /// List categories currently used by expenses
    List,
    /// Remove categories from all matching expenses
    Rm(CategoryRmArgs),
    /// Rename one or more categories into a destination (merges if dst already exists)
    Rename(CategoryRenameArgs),
}

#[derive(Args, Debug)]
#[command(after_help = "Examples:
  recu category rm streaming
  recu category rm @1
  recu category rm @2,housing  (comma-separated; run 'recu category list' first for @ids)")]
pub struct CategoryRmArgs {
    /// Categories to remove: @id or name (case-insensitive), comma-separated.
    #[arg(value_delimiter = ',')]
    pub targets: Vec<String>,
}

#[derive(Args, Debug)]
#[command(after_help = "Examples:
  recu category rename streaming Streaming
  recu category rename @1 Streaming
  recu category rename streaming,subs Streaming  (comma-separated merges into dst)")]
pub struct CategoryRenameArgs {
    /// Source categories: @id or name (case-insensitive), comma-separated.
    #[arg(value_delimiter = ',', num_args = 1)]
    pub sources: Vec<String>,
    /// Destination category name
    pub dst: String,
}

/// Resolves comma-separated `@id` or name inputs against the store's categories.
/// Empty input returns an empty list. Used by `ls`/`calendar`/`treemap` filters.
pub(crate) fn resolve_filter(inputs: &[String], store: &Store) -> io::Result<Vec<String>> {
    if inputs.is_empty() {
        return Ok(Vec::new());
    }
    let categories = store.categories()?;
    inputs
        .iter()
        .map(|t| resolve_target(t, &categories))
        .collect()
}

fn resolve_target(target: &str, categories: &[String]) -> io::Result<String> {
    if let Some(id_str) = target.strip_prefix('@') {
        let id: usize = id_str.parse().map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid id '{target}'"),
            )
        })?;
        if id == 0 || id > categories.len() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "no category at @{id}. Run 'recu category list' to see available categories"
                ),
            ));
        }
        return Ok(categories[id - 1].clone());
    }

    categories
        .iter()
        .find(|c| c.eq_ignore_ascii_case(target))
        .cloned()
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "category '{target}' not found. Run 'recu category list' to see available categories"
                ),
            )
        })
}

fn validate_dst(dst: &str) -> io::Result<&str> {
    let trimmed = dst.trim();
    if trimmed.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "destination category name cannot be empty",
        ));
    }
    Ok(trimmed)
}

fn resolve_sources(targets: &[String], categories: &[String]) -> io::Result<Vec<String>> {
    let mut resolved: Vec<String> = Vec::with_capacity(targets.len());
    for target in targets {
        let name = resolve_target(target, categories)?;
        if resolved.iter().any(|n| n.eq_ignore_ascii_case(&name)) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("duplicate source '{target}'"),
            ));
        }
        resolved.push(name);
    }
    Ok(resolved)
}

pub fn run(cmd: &CategoryCommand, store: &Store) -> io::Result<()> {
    match cmd {
        CategoryCommand::List => {
            let categories = store.categories()?;
            if categories.is_empty() {
                println!("No categories found.");
            } else {
                let width = (categories.len()).to_string().len() + 1;
                for (i, cat) in categories.iter().enumerate() {
                    let id = format!("@{}", i + 1);
                    println!("{id:<width$}  {cat}");
                }
            }
        }
        CategoryCommand::Rm(args) => {
            if args.targets.is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "no category specified",
                ));
            }
            let categories = store.categories()?;
            let resolved = resolve_sources(&args.targets, &categories)?;

            let refs: Vec<&str> = resolved.iter().map(String::as_str).collect();
            let counts = store.clear_categories(&refs)?;
            for (name, count) in resolved.iter().zip(counts.iter()) {
                println!("Removed category '{name}' from {count} expense(s).");
            }
        }
        CategoryCommand::Rename(args) => {
            if args.sources.is_empty() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "no source category specified",
                ));
            }
            let dst = validate_dst(&args.dst)?;
            let categories = store.categories()?;
            let resolved = resolve_sources(&args.sources, &categories)?;

            let refs: Vec<&str> = resolved.iter().map(String::as_str).collect();
            let counts = store.reassign_category(&refs, dst)?;

            if resolved.len() == 1 {
                println!(
                    "Renamed category '{}' to '{}' in {} expense(s).",
                    resolved[0], dst, counts[0]
                );
            } else {
                for (name, count) in resolved.iter().zip(counts.iter()) {
                    println!("  '{name}': {count} expense(s)");
                }
                let total: usize = counts.iter().sum();
                println!("Renamed into '{dst}' ({total} expense(s) total).");
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Vec<String> {
        vec!["food".into(), "housing".into(), "streaming".into()]
    }

    #[test]
    fn resolve_target_by_name_is_case_insensitive() {
        let cats = sample();
        assert_eq!(
            resolve_target("Housing", &cats).expect("resolve should succeed"),
            "housing"
        );
    }

    #[test]
    fn resolve_target_by_id() {
        let cats = sample();
        assert_eq!(
            resolve_target("@2", &cats).expect("resolve should succeed"),
            "housing"
        );
    }

    #[test]
    fn resolve_target_non_numeric_id_is_invalid_input() {
        let cats = sample();
        let err = resolve_target("@abc", &cats).expect_err("non-numeric id should fail");
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
        assert!(err.to_string().contains("@abc"));
    }

    #[test]
    fn resolve_target_empty_id_is_invalid_input() {
        let cats = sample();
        let err = resolve_target("@", &cats).expect_err("empty id should fail");
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn resolve_target_zero_id_is_not_found() {
        let cats = sample();
        let err = resolve_target("@0", &cats).expect_err("zero id should fail");
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn resolve_target_out_of_range_id_is_not_found() {
        let cats = sample();
        let err = resolve_target("@99", &cats).expect_err("out-of-range id should fail");
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn resolve_target_unknown_name_is_not_found() {
        let cats = sample();
        let err = resolve_target("nope", &cats).expect_err("unknown name should fail");
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn validate_dst_rejects_empty_and_whitespace() {
        assert_eq!(
            validate_dst("").expect_err("empty dst").kind(),
            io::ErrorKind::InvalidInput
        );
        assert_eq!(
            validate_dst("   ").expect_err("whitespace dst").kind(),
            io::ErrorKind::InvalidInput
        );
    }

    #[test]
    fn validate_dst_trims_whitespace() {
        assert_eq!(
            validate_dst("  Streaming  ").expect("valid dst"),
            "Streaming"
        );
    }

    #[test]
    fn resolve_sources_rejects_duplicates_case_insensitive() {
        let cats = sample();
        let err = resolve_sources(&["food".into(), "FOOD".into()], &cats)
            .expect_err("duplicate sources should fail");
        assert_eq!(err.kind(), io::ErrorKind::InvalidInput);
    }

    #[test]
    fn resolve_sources_by_mixed_id_and_name() {
        let cats = sample();
        let resolved = resolve_sources(&["@1".into(), "Housing".into()], &cats)
            .expect("resolve should succeed");
        assert_eq!(resolved, vec!["food", "housing"]);
    }
}
