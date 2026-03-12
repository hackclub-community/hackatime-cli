use std::io::{self, Write};

use crate::models::DashboardData;

pub fn print_dashboard(data: &DashboardData) {
    print!("\x1b[2J\x1b[H");
    let _ = io::stdout().flush();
    println!("{}", data.title);
    println!("===============");
    for stat in &data.stats {
        println!("{:<28} {}", stat.label, stat.value);
    }

    if !data.languages.is_empty() {
        println!();
        if let Some(title) = &data.languages_title {
            println!("{title}");
            println!("{}", "-".repeat(title.chars().count()));
        }
        for language in &data.languages {
            println!(
                "{:<14} {:<24} {:>5.1}%  {}",
                truncate_label(&language.name, 14),
                bar(language.percent),
                language.percent,
                language.hours_text
            );
        }
    }
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
