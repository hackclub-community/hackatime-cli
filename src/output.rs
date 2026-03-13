use std::io::{self, IsTerminal, Write};

use crate::models::{DashboardData, DashboardLayout};

pub fn print_dashboard(data: &DashboardData) {
    let stdout = io::stdout();
    let use_color = stdout.is_terminal();

    if use_color {
        print!("\x1b[2J\x1b[H");
        let _ = io::stdout().flush();
    }

    if matches!(data.layout, DashboardLayout::Fetch) {
        print_fetch(data, use_color);
        return;
    }

    println!("{}", paint(&data.title, Style::title(use_color)));
    println!(
        "{}",
        paint(
            &"=".repeat(data.title.chars().count()),
            Style::accent(use_color)
        )
    );

    for stat in &data.stats {
        println!(
            "{} {}",
            paint(&format!("{:<28}", stat.label), Style::label(use_color)),
            paint(&stat.value, Style::value(use_color))
        );
    }

    if !data.languages.is_empty() {
        println!();
        if let Some(title) = &data.languages_title {
            println!("{}", paint(title, Style::section(use_color)));
            println!(
                "{}",
                paint(&"-".repeat(title.chars().count()), Style::muted(use_color))
            );
        }
        for language in &data.languages {
            println!(
                "{} {} {}  {}",
                paint(
                    &format!("{:<14}", truncate_label(&language.name, 14)),
                    Style::language(use_color)
                ),
                color_bar(language.percent, use_color),
                paint(
                    &format!("{:>5.1}%", language.percent),
                    Style::percent(use_color)
                ),
                paint(&language.hours_text, Style::hours(use_color))
            );
        }
    }
}

fn print_fetch(data: &DashboardData, use_color: bool) {
    let mut right = Vec::new();
    right.push(paint(&data.title, Style::fetch_title(use_color)));
    right.push(paint(
        &"-".repeat(data.title.chars().count()),
        Style::muted(use_color),
    ));

    for stat in &data.stats {
        right.push(fetch_row(&stat.label, &stat.value, use_color));
    }

    if !data.languages.is_empty() {
        if let Some(language) = data.languages.first() {
            right.push(fetch_row(
                "Top Language",
                &format!("{} ({:.1}%)", language.name, language.percent),
                use_color,
            ));
        }
        right.push(String::new());
        right.extend(fetch_swatches(use_color));
    }

    let logo = fetch_logo(right.len(), use_color);
    let logo_width = logo
        .iter()
        .map(|line| visible_width(line))
        .max()
        .unwrap_or(0)
        .max(18);

    println!();
    println!();

    let row_count = right.len().max(logo.len());
    for index in 0..row_count {
        let logo_line = logo.get(index).cloned().unwrap_or_default();
        let right_line = right.get(index).cloned().unwrap_or_default();
        let padding = logo_width.saturating_sub(visible_width(&logo_line));
        println!("{logo_line}{}    {right_line}", " ".repeat(padding));
    }

    println!();
    println!();
}

fn fetch_row(label: &str, value: &str, use_color: bool) -> String {
    format!(
        "{} {}",
        paint(
            &format!("{:<16}", format!("{label}:")),
            Style::fetch_label(use_color)
        ),
        paint(value, Style::value(use_color))
    )
}

fn color_bar(percent: f64, use_color: bool) -> String {
    let filled = bar(percent);
    if !use_color {
        return filled;
    }

    let fill_color = if percent >= 50.0 {
        46
    } else if percent >= 20.0 {
        220
    } else {
        208
    };

    let filled_len = filled.chars().take_while(|ch| *ch == '#').count();
    let empty_len = filled.len().saturating_sub(filled_len);
    format!(
        "{}{}{}{}{}",
        ansi(fill_color, false),
        "#".repeat(filled_len),
        ansi_reset(),
        paint(&"-".repeat(empty_len), Style::muted(true)),
        ansi_reset()
    )
}

fn bar(percent: f64) -> String {
    let width = 24_usize;
    let filled = ((percent.clamp(0.0, 100.0) / 100.0) * width as f64).round() as usize;
    let filled = filled.min(width);
    let mut bar = String::with_capacity(width);
    for _ in 0..filled {
        bar.push('#');
    }
    for _ in filled..width {
        bar.push('-');
    }
    bar
}

fn truncate_label(label: &str, width: usize) -> String {
    let char_count = label.chars().count();
    if char_count <= width {
        return label.to_string();
    }

    let mut shortened = label
        .chars()
        .take(width.saturating_sub(1))
        .collect::<String>();
    shortened.push('~');
    shortened
}

fn visible_width(text: &str) -> usize {
    let mut width = 0_usize;
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch == '\x1b' {
            while let Some(next) = chars.next() {
                if next == 'm' {
                    break;
                }
            }
        } else {
            width += 1;
        }
    }

    width
}

fn fetch_logo(height: usize, use_color: bool) -> Vec<String> {
    let height = height.max(8);
    let crossbar = (height / 3).max(2);
    let mut lines = Vec::with_capacity(height);

    for row in 0..height {
        let raw = if row < crossbar {
            "hhhhhh              "
        } else if row == crossbar {
            "hhhhhhhhhhhhhh      "
        } else {
            "hhhhhh      hhhhhh  "
        };

        if use_color {
            let color = fetch_logo_color(row, height);
            lines.push(paint(
                raw,
                Style {
                    color,
                    bold: true,
                    enabled: true,
                },
            ));
        } else {
            lines.push(raw.to_string());
        }
    }

    lines
}

fn fetch_logo_color(row: usize, height: usize) -> u8 {
    let progress = row as f32 / height.max(1) as f32;
    if progress < 0.25 {
        196
    } else if progress < 0.5 {
        203
    } else if progress < 0.75 {
        210
    } else {
        217
    }
}

fn fetch_swatches(use_color: bool) -> Vec<String> {
    if !use_color {
        return vec![
            "[blk][red][grn][ylw][blu][mag][cyn][wht]".to_string(),
            "[gry][lrd][lgn][lyw][lbl][lmg][lcy][brt]".to_string(),
        ];
    }

    vec![
        swatch_row(&[16, 160, 34, 184, 19, 163, 37, 252]),
        swatch_row(&[236, 203, 120, 229, 111, 219, 159, 15]),
    ]
}

fn swatch_row(colors: &[u8]) -> String {
    colors
        .iter()
        .map(|color| format!("\x1b[48;5;{color}m   \x1b[0m"))
        .collect::<Vec<_>>()
        .join("")
}

fn paint(text: &str, style: Style) -> String {
    if !style.enabled {
        return text.to_string();
    }

    format!("{}{}{}", style.prefix(), text, ansi_reset())
}

fn ansi(code: u8, bold: bool) -> String {
    if bold {
        format!("\x1b[1;38;5;{code}m")
    } else {
        format!("\x1b[38;5;{code}m")
    }
}

fn ansi_reset() -> &'static str {
    "\x1b[0m"
}

#[derive(Clone, Copy)]
struct Style {
    color: u8,
    bold: bool,
    enabled: bool,
}

impl Style {
    fn fetch_title(enabled: bool) -> Self {
        Self {
            color: 203,
            bold: true,
            enabled,
        }
    }

    fn fetch_label(enabled: bool) -> Self {
        Self {
            color: 210,
            bold: true,
            enabled,
        }
    }

    fn title(enabled: bool) -> Self {
        Self {
            color: 45,
            bold: true,
            enabled,
        }
    }

    fn accent(enabled: bool) -> Self {
        Self {
            color: 81,
            bold: false,
            enabled,
        }
    }

    fn section(enabled: bool) -> Self {
        Self {
            color: 117,
            bold: true,
            enabled,
        }
    }

    fn label(enabled: bool) -> Self {
        Self {
            color: 250,
            bold: true,
            enabled,
        }
    }

    fn value(enabled: bool) -> Self {
        Self {
            color: 231,
            bold: false,
            enabled,
        }
    }

    fn language(enabled: bool) -> Self {
        Self {
            color: 159,
            bold: true,
            enabled,
        }
    }

    fn percent(enabled: bool) -> Self {
        Self {
            color: 151,
            bold: false,
            enabled,
        }
    }

    fn hours(enabled: bool) -> Self {
        Self {
            color: 223,
            bold: false,
            enabled,
        }
    }

    fn muted(enabled: bool) -> Self {
        Self {
            color: 244,
            bold: false,
            enabled,
        }
    }

    fn prefix(self) -> String {
        ansi(self.color, self.bold)
    }
}
