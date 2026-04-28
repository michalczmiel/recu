//! Terminal UI primitives: text layout, day humanization, and semantic styling.
//!
//! This module wraps `colored` the same way `prompt` wraps `inquire`: commands
//! should reach for these helpers instead of naming colors directly.

use crate::expense::DueStatus;
use colored::{ColoredString, Colorize};

// --- text layout ---

/// Visible width of a plain string in terminal columns.
/// Uses char count — sufficient for Latin/currency symbols (all 1-column wide).
pub fn char_width(s: &str) -> usize {
    s.chars().count()
}

/// Pad `colored` (which may contain ANSI codes) to `width` columns,
/// using `plain_w` (visible char width) for the math.
pub fn pad_end(colored: &str, plain_w: usize, width: usize) -> String {
    let spaces = width.saturating_sub(plain_w);
    format!("{colored}{}", " ".repeat(spaces))
}

pub fn pad_start(colored: &str, plain_w: usize, width: usize) -> String {
    let spaces = width.saturating_sub(plain_w);
    format!("{}{colored}", " ".repeat(spaces))
}

/// Truncate to `max` chars, appending `…` when cut.
pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else if max > 1 {
        format!("{}…", s.chars().take(max - 1).collect::<String>())
    } else {
        s.chars().take(max).collect()
    }
}

// --- day humanization ---

fn humanize(abs_days: i64) -> (i64, &'static str) {
    match abs_days {
        0..=6 => (abs_days, "day"),
        7..=29 => (abs_days / 7, "week"),
        30..=364 => (abs_days / 30, "month"),
        _ => (abs_days / 365, "year"),
    }
}

fn plural(n: i64) -> &'static str {
    if n == 1 { "" } else { "s" }
}

/// Format a non-negative day offset as "today" / "in N unit(s)".
pub fn format_in_days(days: i64) -> String {
    if days == 0 {
        return "today".to_string();
    }
    let (n, unit) = humanize(days);
    format!("in {n} {unit}{}", plural(n))
}

/// Format a signed day offset: future → "in N …", past → "N … ago".
pub fn format_ago_or_in(days: i64) -> String {
    if days >= 0 {
        return format_in_days(days);
    }
    let (n, unit) = humanize(-days);
    format!("{n} {unit}{} ago", plural(n))
}

// --- semantic styling ---
//
// Callers should prefer these over raw `.red()` / `.bold()` / `.dimmed()`.
// The `colored` crate already no-ops when stdout isn't a TTY.

pub fn dim(s: &str) -> ColoredString {
    s.dimmed()
}

pub fn heading(s: &str) -> ColoredString {
    s.bold()
}

pub fn error_label(s: &str) -> ColoredString {
    s.red().bold()
}

/// Apply the color (not weight) associated with a due status.
/// Returns the input unchanged for statuses without a dedicated color.
pub fn due(status: &DueStatus, s: &str) -> ColoredString {
    match status {
        DueStatus::Overdue => s.red(),
        DueStatus::DueSoon => s.yellow(),
        DueStatus::Distant => s.dimmed(),
        DueStatus::Normal | DueStatus::Unknown => s.normal(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncate_fits() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn truncate_with_ellipsis() {
        assert_eq!(truncate("hello world", 6), "hello…");
    }

    #[test]
    fn truncate_max_one() {
        assert_eq!(truncate("hello", 1), "h");
    }

    #[test]
    fn format_in_days_examples() {
        assert_eq!(format_in_days(0), "today");
        assert_eq!(format_in_days(1), "in 1 day");
        assert_eq!(format_in_days(5), "in 5 days");
        assert_eq!(format_in_days(14), "in 2 weeks");
        assert_eq!(format_in_days(60), "in 2 months");
        assert_eq!(format_in_days(800), "in 2 years");
    }

    #[test]
    fn format_ago_or_in_past_uses_ago_suffix() {
        assert_eq!(format_ago_or_in(-1), "1 day ago");
        assert_eq!(format_ago_or_in(-5), "5 days ago");
        assert_eq!(format_ago_or_in(-14), "2 weeks ago");
        assert_eq!(format_ago_or_in(-60), "2 months ago");
        assert_eq!(format_ago_or_in(-800), "2 years ago");
    }

    #[test]
    fn format_ago_or_in_future_matches_in_days() {
        assert_eq!(format_ago_or_in(0), "today");
        assert_eq!(format_ago_or_in(5), "in 5 days");
    }
}
