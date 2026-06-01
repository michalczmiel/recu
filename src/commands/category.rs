use std::io::{self, Write};

use clap::{Args, Subcommand};
use serde::Serialize;

use crate::commands::emit_json;
use crate::store::Store;

#[derive(Subcommand, Debug)]
pub enum CategoryCommand {
    /// List categories currently used by expenses
    #[command(alias = "ls")]
    List(CategoryListArgs),
    /// Remove categories from all matching expenses
    #[command(aliases = ["rm", "delete", "del"])]
    Remove(CategoryRemoveArgs),
    /// Rename one or more categories into a destination (merges if dst already exists)
    #[command(alias = "mv")]
    Rename(CategoryRenameArgs),
}

#[derive(Args, Debug)]
pub struct CategoryListArgs {
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
#[command(after_help = "Examples:
  recu category remove streaming
  recu category remove @1
  recu category remove @2,housing  (comma-separated; run 'recu category list' first for @ids)")]
pub struct CategoryRemoveArgs {
    /// Categories to remove: @id or name (case-insensitive), comma-separated.
    #[arg(value_delimiter = ',')]
    pub targets: Vec<String>,
    /// Output as JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
#[command(after_help = "Examples:
  recu category rename streaming Streaming
  recu category rename @1 Streaming
  recu category rename streaming,subs Streaming  (comma-separated merges into dst)")]
pub struct CategoryRenameArgs {
    /// Source categories: @id or name (case-insensitive), comma-separated.
    #[arg(value_delimiter = ',', num_args = 1, required = true)]
    pub sources: Vec<String>,
    /// Destination category name
    pub dst: String,
}

#[derive(Serialize)]
struct CategoryRemoval<'a> {
    category: &'a str,
    expenses_updated: usize,
}

/// Resolves comma-separated `@id` or name inputs against the store's categories.
/// Empty input returns an empty list. Used by `list`/`calendar`/`treemap` filters.
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

fn render_list(out: &mut impl Write, categories: &[String], json: bool) -> io::Result<()> {
    if json {
        emit_json(out, &categories)?;
    } else if categories.is_empty() {
        writeln!(out, "No categories found.")?;
    } else {
        let width = categories.len().to_string().len() + 1;
        for (i, cat) in categories.iter().enumerate() {
            let id = format!("@{}", i + 1);
            writeln!(out, "{id:<width$}  {cat}")?;
        }
    }
    Ok(())
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
                unknown_category_message(target, categories),
            )
        })
}

fn unknown_category_message(target: &str, categories: &[String]) -> String {
    let known = if categories.is_empty() {
        "(none)".to_string()
    } else {
        categories.join(", ")
    };
    let example = categories.first().map_or_else(
        || "recu category list".to_string(),
        |c| format!("recu list --category {c}"),
    );
    format!("unknown category '{target}'; known: {known}\nexample: {example}")
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
        CategoryCommand::List(args) => {
            let categories = store.categories()?;
            render_list(&mut std::io::stdout(), &categories, args.json)?;
        }
        CategoryCommand::Remove(args) => {
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
            if args.json {
                let items: Vec<_> = resolved
                    .iter()
                    .zip(counts.iter())
                    .map(|(name, &count)| CategoryRemoval {
                        category: name,
                        expenses_updated: count,
                    })
                    .collect();
                emit_json(&mut std::io::stdout(), &items)?;
            } else {
                for (name, count) in resolved.iter().zip(counts.iter()) {
                    println!("Removed category '{name}' from {count} expense(s)");
                }
            }
        }
        CategoryCommand::Rename(args) => {
            let dst = validate_dst(&args.dst)?;
            let categories = store.categories()?;
            let resolved = resolve_sources(&args.sources, &categories)?;

            let refs: Vec<&str> = resolved.iter().map(String::as_str).collect();
            let counts = store.reassign_category(&refs, dst)?;

            if resolved.len() == 1 {
                println!(
                    "Renamed category '{}' to '{}' in {} expense(s)",
                    resolved[0], dst, counts[0]
                );
            } else {
                for (name, count) in resolved.iter().zip(counts.iter()) {
                    println!("  '{name}': {count} expense(s)");
                }
                let total: usize = counts.iter().sum();
                println!("Renamed into '{dst}' ({total} expense(s) total)");
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

    fn run_list(categories: &[String], json: bool) -> String {
        let mut buf = Vec::new();
        render_list(&mut buf, categories, json).expect("render_list");
        String::from_utf8(buf).expect("utf8")
    }

    #[test]
    fn list() {
        let mut out = String::new();

        out += "=== empty ===\n";
        out += &run_list(&[], false);

        out += "\n=== empty json ===\n";
        out += &run_list(&[], true);

        // Single-digit count → width 2; @ids reflect store order
        out += "\n=== a few categories ===\n";
        out += &run_list(&sample(), false);

        out += "\n=== a few categories json ===\n";
        out += &run_list(&sample(), true);

        // 10+ categories → two-digit @ids widen the id column
        let many: Vec<String> = (1..=12).map(|i| format!("cat{i}")).collect();
        out += "\n=== many categories widen id column ===\n";
        out += &run_list(&many, false);

        insta::assert_snapshot!(out);
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
    fn resolve_target_invalid_ids() {
        let cats = sample();
        let cases = [
            ("@abc", io::ErrorKind::InvalidInput),
            ("@", io::ErrorKind::InvalidInput),
            ("@0", io::ErrorKind::NotFound),
            ("@99", io::ErrorKind::NotFound),
        ];
        for (input, expected) in cases {
            let err = resolve_target(input, &cats).expect_err("invalid id should fail");
            assert_eq!(err.kind(), expected, "input: {input}");
        }
    }

    #[test]
    fn resolve_target_unknown_name_is_not_found() {
        let cats = sample();
        let err = resolve_target("nope", &cats).expect_err("unknown name should fail");
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn unknown_category_message_lists_known_categories() {
        let msg = unknown_category_message("nope", &sample());
        assert_eq!(
            msg,
            "unknown category 'nope'; known: food, housing, streaming\n\
             example: recu list --category food"
        );
    }

    #[test]
    fn unknown_category_message_with_no_known_categories() {
        let msg = unknown_category_message("nope", &[]);
        assert_eq!(
            msg,
            "unknown category 'nope'; known: (none)\n\
             example: recu category list"
        );
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
