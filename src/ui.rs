//! Terminal UI primitives: text layout, day humanization, and semantic styling.
//!
//! This module wraps `colored` the same way `prompt` wraps `inquire`: commands
//! should reach for these helpers instead of naming colors directly.

use crate::expense::DueStatus;
use chrono::{Datelike, NaiveDate};
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

// --- date humanization ---

/// Format a target date relative to `today` as a calendar-aware label:
/// `yesterday` / `today` / `tomorrow`, the weekday name (`Mon`..`Sun`) within
/// the next 2–6 days, otherwise an absolute date (`May 28`, with `, YYYY`
/// appended when the year differs from `today`).
pub fn format_relative_date(target: NaiveDate, today: NaiveDate) -> String {
    let days = (target - today).num_days();
    match days {
        -1 => "yesterday".to_string(),
        0 => "today".to_string(),
        1 => "tomorrow".to_string(),
        2..=6 => target.format("%a").to_string(),
        _ if target.year() == today.year() => target.format("%b %-d").to_string(),
        _ => target.format("%b %-d, %Y").to_string(),
    }
}

// --- semantic styling ---
//
// Callers should prefer these over raw `.red()` / `.bold()` / `.dimmed()`.
// The `colored` crate already no-ops when stdout isn't a TTY.

pub fn dim(s: &str) -> ColoredString {
    s.dimmed()
}

pub fn bold(s: &str) -> ColoredString {
    s.bold()
}

pub fn heading(s: &str) -> ColoredString {
    s.bold()
}

pub fn error_label(s: &str) -> ColoredString {
    s.red().bold()
}

/// Calendar cell styling: today, past day, current/future charge, past charge.
pub fn today_cell(s: &str) -> ColoredString {
    s.yellow().bold()
}

pub fn charge(s: &str) -> ColoredString {
    s.cyan().bold()
}

pub fn past_charge(s: &str) -> ColoredString {
    s.dimmed().bold()
}

/// Render a single character with truecolor foreground over truecolor background.
pub fn truecolor_pixel(ch: char, fg: (u8, u8, u8), bg: (u8, u8, u8)) -> String {
    ch.to_string()
        .truecolor(fg.0, fg.1, fg.2)
        .on_truecolor(bg.0, bg.1, bg.2)
        .to_string()
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

    use chrono::Days;

    // Friday, picked so that +2..=+6 spans Sun..Thu.
    fn today() -> NaiveDate {
        NaiveDate::from_ymd_opt(2026, 5, 8).expect("valid date")
    }

    fn plus(days: u64) -> NaiveDate {
        today().checked_add_days(Days::new(days)).expect("in range")
    }

    fn minus(days: u64) -> NaiveDate {
        today().checked_sub_days(Days::new(days)).expect("in range")
    }

    #[test]
    fn relative_date_named_anchors() {
        assert_eq!(format_relative_date(today(), today()), "today");
        assert_eq!(format_relative_date(plus(1), today()), "tomorrow");
        assert_eq!(format_relative_date(minus(1), today()), "yesterday");
    }

    #[test]
    fn relative_date_weekday_window_future_only() {
        assert_eq!(format_relative_date(plus(2), today()), "Sun");
        assert_eq!(format_relative_date(plus(6), today()), "Thu");
        // Past beyond yesterday falls through to absolute date.
        assert_eq!(format_relative_date(minus(2), today()), "May 6");
    }

    #[test]
    fn relative_date_absolute_same_year_omits_year() {
        assert_eq!(format_relative_date(plus(7), today()), "May 15");
        assert_eq!(
            format_relative_date(
                NaiveDate::from_ymd_opt(2026, 12, 1).expect("valid date"),
                today()
            ),
            "Dec 1"
        );
    }

    #[test]
    fn relative_date_absolute_different_year_includes_year() {
        assert_eq!(
            format_relative_date(
                NaiveDate::from_ymd_opt(2027, 1, 15).expect("valid date"),
                today()
            ),
            "Jan 15, 2027"
        );
        assert_eq!(
            format_relative_date(
                NaiveDate::from_ymd_opt(2025, 11, 30).expect("valid date"),
                today()
            ),
            "Nov 30, 2025"
        );
    }
}
