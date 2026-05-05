use std::collections::HashMap;
use std::io::{self, Write as _};

use clap::Args;

use crate::config;
use crate::rates;
use crate::store::Store;
use crate::ui;
use colored::Colorize;
use rusty_money::{Findable, iso};

#[derive(Args, Debug, Default)]
pub struct TreemapArgs {
    /// Include ended expenses
    #[arg(short, long)]
    pub all: bool,
}

// Terminal characters are roughly twice as tall as wide.
// We scale the logical layout height so cells appear visually square.
const CHAR_ASPECT: f64 = 2.0;

const PALETTE: [(u8, u8, u8); 10] = [
    (52, 100, 150),
    (45, 130, 90),
    (160, 80, 50),
    (110, 70, 160),
    (150, 60, 90),
    (40, 120, 140),
    (140, 110, 40),
    (80, 130, 90),
    (120, 80, 130),
    (90, 120, 160),
];

// --- Squarified treemap layout ---

#[derive(Clone)]
struct Rect {
    left: f64,
    top: f64,
    width: f64,
    height: f64,
}

fn worst_ratio(row: &[f64], side: f64) -> f64 {
    if row.is_empty() || side == 0.0 {
        return f64::INFINITY;
    }
    let sum: f64 = row.iter().sum();
    if sum == 0.0 {
        return f64::INFINITY;
    }
    let max = row
        .iter()
        .copied()
        .reduce(f64::max)
        .unwrap_or(f64::NEG_INFINITY);
    let min = row
        .iter()
        .copied()
        .reduce(f64::min)
        .unwrap_or(f64::INFINITY);
    f64::max(
        side * side * max / (sum * sum),
        sum * sum / (side * side * min),
    )
}

fn lay_out_row(row: &[f64], left: f64, top: f64, width: f64, height: f64) -> Vec<Rect> {
    let row_sum: f64 = row.iter().sum();
    if width >= height {
        let strip_w = if height > 0.0 { row_sum / height } else { 0.0 };
        row.iter()
            .scan(top, |curr_top, &size| {
                let cell_h = if strip_w > 0.0 { size / strip_w } else { 0.0 };
                let rect = Rect {
                    left,
                    top: *curr_top,
                    width: strip_w,
                    height: cell_h,
                };
                *curr_top += cell_h;
                Some(rect)
            })
            .collect()
    } else {
        let strip_h = if width > 0.0 { row_sum / width } else { 0.0 };
        row.iter()
            .scan(left, |curr_left, &size| {
                let cell_w = if strip_h > 0.0 { size / strip_h } else { 0.0 };
                let rect = Rect {
                    left: *curr_left,
                    top,
                    width: cell_w,
                    height: strip_h,
                };
                *curr_left += cell_w;
                Some(rect)
            })
            .collect()
    }
}

// Iterative squarified treemap: avoids per-call Vec allocation of the recursive version.
fn squarify(sizes: &[f64], width: f64, height: f64) -> Vec<Rect> {
    let total: f64 = sizes.iter().sum();
    if total == 0.0 {
        return vec![];
    }
    let area = width * height;
    let norm: Vec<f64> = sizes.iter().map(|&s| s / total * area).collect();

    let mut rects = Vec::with_capacity(norm.len());
    let mut row: Vec<f64> = Vec::new();
    let mut left = 0.0_f64;
    let mut top = 0.0_f64;
    let mut w = width;
    let mut h = height;
    let mut side = f64::min(w, h);

    for &size in &norm {
        let old_ratio = worst_ratio(&row, side);
        row.push(size);
        let new_ratio = worst_ratio(&row, side);

        // Always keep the first item in a new row; otherwise keep if ratio improves.
        if row.len() > 1 && new_ratio > old_ratio {
            row.pop();
            rects.extend(lay_out_row(&row, left, top, w, h));
            let row_sum: f64 = row.iter().sum();
            if w >= h {
                let sw = if h > 0.0 { row_sum / h } else { 0.0 };
                left += sw;
                w -= sw;
            } else {
                let sh = if w > 0.0 { row_sum / w } else { 0.0 };
                top += sh;
                h -= sh;
            }
            side = f64::min(w, h);
            row.clear();
            row.push(size);
        }
    }

    if !row.is_empty() {
        rects.extend(lay_out_row(&row, left, top, w, h));
    }

    rects
}

// --- Rendering ---

#[derive(Clone)]
struct Pixel {
    ch: char,
    bg: (u8, u8, u8),
    fg: (u8, u8, u8),
}

impl Default for Pixel {
    fn default() -> Self {
        Self {
            ch: ' ',
            bg: (15, 15, 15),
            fg: (200, 200, 200),
        }
    }
}

fn lighten(color: (u8, u8, u8)) -> (u8, u8, u8) {
    // midpoint(u8_val, 255) is always <= 255, so the cast never truncates
    #[allow(clippy::cast_possible_truncation)]
    let blend = |ch: u8| u16::midpoint(u16::from(ch), 255) as u8;
    (blend(color.0), blend(color.1), blend(color.2))
}

fn fill_rect(
    grid: &mut [Vec<Pixel>],
    col0: usize,
    row0: usize,
    rw: usize,
    rh: usize,
    color: (u8, u8, u8),
) {
    let total_rows = grid.len();
    let total_cols = grid.first().map_or(0, Vec::len);
    let col1 = (col0 + rw).min(total_cols);
    let row1 = (row0 + rh).min(total_rows);
    let actual_rows = row1.saturating_sub(row0);
    let actual_cols = col1.saturating_sub(col0);

    for (ri, row_pixels) in grid[row0..row1].iter_mut().enumerate() {
        let is_top = ri == 0;
        let is_bot = ri == actual_rows.saturating_sub(1);
        for (ci, px) in row_pixels[col0..col1].iter_mut().enumerate() {
            let is_left = ci == 0;
            let is_right = ci == actual_cols.saturating_sub(1);
            let ch = if is_top && is_left {
                '┌'
            } else if is_top && is_right {
                '┐'
            } else if is_bot && is_left {
                '└'
            } else if is_bot && is_right {
                '┘'
            } else if is_top || is_bot {
                '─'
            } else if is_left || is_right {
                '│'
            } else {
                ' '
            };
            *px = Pixel {
                ch,
                bg: color,
                fg: (255, 255, 255),
            };
        }
    }
}

fn write_str(
    grid: &mut [Vec<Pixel>],
    row: usize,
    col: usize,
    s: &str,
    bg: (u8, u8, u8),
    fg: (u8, u8, u8),
) {
    if row >= grid.len() {
        return;
    }
    let total_cols = grid[row].len();
    for (i, ch) in s.chars().enumerate() {
        let c = col + i;
        if c < total_cols {
            grid[row][c] = Pixel { ch, bg, fg };
        }
    }
}

struct Tile {
    name: String,
    monthly: f64,
    yearly: f64,
    symbol: String,
    symbol_first: bool,
    rect: Rect,
    color: (u8, u8, u8),
}

#[allow(clippy::similar_names)]
fn render(tiles: &[Tile], cols: usize, rows: usize) {
    let mut grid: Vec<Vec<Pixel>> = vec![vec![Pixel::default(); cols]; rows];

    for tile in tiles {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let col0 = tile.rect.left.max(0.0).round() as usize;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let row0 = tile.rect.top.max(0.0).round() as usize;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let rw = tile.rect.width.max(0.0).round() as usize;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let rh = tile.rect.height.max(0.0).round() as usize;

        if rw < 3 || rh < 2 {
            continue;
        }

        fill_rect(&mut grid, col0, row0, rw, rh, tile.color);

        let inner_w = rw.saturating_sub(2);
        if rh >= 3 && inner_w > 0 {
            let name = ui::truncate(&tile.name, inner_w);
            write_str(
                &mut grid,
                row0 + 1,
                col0 + 1,
                &name,
                tile.color,
                (255, 255, 255),
            );
        }
        if rh >= 4 && inner_w >= 5 {
            let mo_label = if tile.symbol_first {
                format!("{}{:.0}/mo", tile.symbol, tile.monthly)
            } else {
                format!("{:.0} {}/mo", tile.monthly, tile.symbol)
            };
            write_str(
                &mut grid,
                row0 + 2,
                col0 + 1,
                &ui::truncate(&mo_label, inner_w),
                tile.color,
                lighten(tile.color),
            );
        }
        if rh >= 5 && inner_w >= 5 {
            let yr_label = if tile.symbol_first {
                format!("{}{:.0}/yr", tile.symbol, tile.yearly)
            } else {
                format!("{:.0} {}/yr", tile.yearly, tile.symbol)
            };
            write_str(
                &mut grid,
                row0 + 3,
                col0 + 1,
                &ui::truncate(&yr_label, inner_w),
                tile.color,
                lighten(tile.color),
            );
        }
    }

    let stdout = io::stdout();
    let mut out = io::BufWriter::new(stdout.lock());
    for row in &grid {
        for px in row {
            let (br, bg, bb) = px.bg;
            let (fr, fg, fb) = px.fg;
            let _ = write!(
                out,
                "{}",
                px.ch
                    .to_string()
                    .truecolor(fr, fg, fb)
                    .on_truecolor(br, bg, bb)
            );
        }
        let _ = writeln!(out);
    }
}

fn query_terminal_size() -> (usize, usize) {
    use terminal_size::{Height, Width, terminal_size as ts};
    if let Some((Width(w), Height(h))) = ts() {
        #[allow(clippy::cast_possible_truncation)]
        return (w as usize, h as usize);
    }
    let cols = std::env::var("COLUMNS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(80);
    let rows = std::env::var("LINES")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(20);
    (cols, rows)
}

pub fn execute(args: &TreemapArgs, store: &Store) -> std::io::Result<()> {
    let expenses = store.list()?;
    if expenses.is_empty() {
        println!("No recurring expenses found.");
        return Ok(());
    }

    let today = chrono::Local::now().date_naive();
    let cfg = config::load()?;
    let target: Option<&str> = cfg.currency.as_deref();
    let exchange_rates: Option<HashMap<String, f64>> = target.map(rates::get_rates).transpose()?;
    let target_cur: Option<&'static iso::Currency> = target.and_then(iso::Currency::find);

    let mut items: Vec<(String, f64, String, bool, Option<String>)> = expenses
        .into_iter()
        .filter(|expense| args.all || !expense.is_ended(today))
        .filter_map(|expense| {
            let amount = expense.amount?;
            let interval = expense.interval.as_ref()?;
            let converted = crate::expense::convert(
                amount,
                expense.currency.as_deref(),
                exchange_rates.as_ref(),
                target,
            );
            let cur = crate::expense::display_currency(
                expense.currency.as_deref(),
                exchange_rates.as_ref(),
                target,
                target_cur,
            );
            let symbol = cur.map_or("", |c| c.symbol).to_string();
            let symbol_first = cur.is_none_or(|c| c.symbol_first);
            Some((
                expense.name,
                interval.to_monthly(converted),
                symbol,
                symbol_first,
                expense.category,
            ))
        })
        .collect();

    if items.is_empty() {
        println!("No expenses with amount and interval set.");
        return Ok(());
    }

    items.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    let (cols, rows) = query_terminal_size();

    let sizes: Vec<f64> = items.iter().map(|(_, v, _, _, _)| *v).collect();

    #[allow(clippy::cast_precision_loss)]
    let logical_w = cols as f64;
    #[allow(clippy::cast_precision_loss)]
    let logical_h = rows as f64 * CHAR_ASPECT;

    let rects = squarify(&sizes, logical_w, logical_h);

    // Assign consistent colors per category so the same category always
    // gets the same color regardless of sort order or item count.
    let mut category_colors: HashMap<String, (u8, u8, u8)> = HashMap::new();
    let mut next_color_idx = 0usize;
    for (_, _, _, _, cat) in &items {
        let key = cat.clone().unwrap_or_default();
        category_colors.entry(key).or_insert_with(|| {
            let color = PALETTE[next_color_idx % PALETTE.len()];
            next_color_idx += 1;
            color
        });
    }

    let tiles: Vec<Tile> = items
        .into_iter()
        .zip(rects)
        .map(|((name, monthly, symbol, symbol_first, category), r)| {
            let key = category.unwrap_or_default();
            let color = category_colors[&key];
            Tile {
                name,
                monthly,
                yearly: monthly * 12.0,
                symbol,
                symbol_first,
                rect: Rect {
                    left: r.left,
                    top: r.top / CHAR_ASPECT,
                    width: r.width,
                    height: r.height / CHAR_ASPECT,
                },
                color,
            }
        })
        .collect();

    render(&tiles, cols, rows);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::expense::Interval;

    #[test]
    fn to_monthly_all_intervals() {
        assert!((Interval::Monthly.to_monthly(12.0) - 12.0).abs() < 1e-10);
        // 12 * 52 / 12 = 52
        assert!((Interval::Weekly.to_monthly(12.0) - 52.0).abs() < 1e-10);
        assert!((Interval::Quarterly.to_monthly(30.0) - 10.0).abs() < 1e-10);
        assert!((Interval::Yearly.to_monthly(120.0) - 10.0).abs() < 1e-10);
    }

    #[test]
    fn worst_ratio_edge_cases() {
        assert!(worst_ratio(&[], 10.0).is_infinite());
        assert!(worst_ratio(&[1.0], 0.0).is_infinite());
        assert!(worst_ratio(&[0.0], 10.0).is_infinite());
    }

    #[test]
    fn worst_ratio_perfect_square() {
        // Single element with area == side²: ratio == 1.0 (perfectly square).
        let ratio = worst_ratio(&[4.0], 2.0);
        assert!((ratio - 1.0).abs() < 1e-10);
    }

    #[test]
    fn squarify_empty_input() {
        assert!(squarify(&[], 100.0, 100.0).is_empty());
        assert!(squarify(&[0.0], 100.0, 100.0).is_empty());
    }

    #[test]
    fn squarify_rect_count() {
        let sizes = vec![6.0, 3.0, 2.0, 2.0, 1.0];
        let rects = squarify(&sizes, 100.0, 60.0);
        assert_eq!(rects.len(), sizes.len());
    }

    #[test]
    fn squarify_total_area() {
        let sizes = vec![6.0, 3.0, 2.0, 2.0, 1.0];
        let rects = squarify(&sizes, 100.0, 60.0);
        let area: f64 = rects.iter().map(|r| r.width * r.height).sum();
        assert!((area - 100.0 * 60.0).abs() < 1e-6);
    }
}
