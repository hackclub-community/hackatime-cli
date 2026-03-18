use std::{
    collections::HashMap,
    io::{self, IsTerminal, Write},
};

use chrono::Datelike;

use crate::models::{
    ActivityGraph, DashboardData, DashboardLayout, FetchTheme, LanguageLine, ProjectGraphLine,
};

pub fn print_dashboard(data: &DashboardData, fetch_theme: FetchTheme, clear_terminal: bool) {
    let stdout = io::stdout();
    let use_color = stdout.is_terminal();

    if use_color && clear_terminal {
        print!("\x1b[2J\x1b[H");
        let _ = io::stdout().flush();
    }

    match data.layout {
        DashboardLayout::Fetch => {
            print_fetch(data, use_color, fetch_theme);
            return;
        }
        DashboardLayout::Projects => {
            print_standard_header(data, use_color);
            print_stats(data, use_color);
            print_project_graphs(data, use_color);
            return;
        }
        DashboardLayout::Graph => {
            if let Some(graph) = &data.activity_graph {
                print_activity_graph(graph, use_color, fetch_theme);
            }
            return;
        }
        DashboardLayout::Standard => {}
    }

    print_standard_header(data, use_color);
    print_stats(data, use_color);

    if !data.languages.is_empty() {
        print_language_graph(data, use_color);
    }
}

fn print_standard_header(data: &DashboardData, use_color: bool) {
    println!("{}", paint(&data.title, Style::title(use_color)));
    println!(
        "{}",
        paint(
            &"=".repeat(data.title.chars().count()),
            Style::accent(use_color)
        )
    );
}

fn print_stats(data: &DashboardData, use_color: bool) {
    for stat in &data.stats {
        println!(
            "{} {}",
            paint(&format!("{:<28}", stat.label), Style::label(use_color)),
            paint(&stat.value, Style::value(use_color))
        );
    }
}

fn section_item_width(items: &[crate::models::LanguageLine]) -> usize {
    items
        .iter()
        .map(|item| item.name.chars().count())
        .max()
        .unwrap_or(14)
        .clamp(14, 20)
}

fn print_language_graph(data: &DashboardData, use_color: bool) {
    if data.languages.is_empty() {
        return;
    }

    let legend = build_language_legend(&data.languages);
    if legend.is_empty() {
        return;
    }

    let item_width = section_item_width(&data.languages).max(11);

    println!();
    if let Some(title) = &data.languages_title {
        println!("{}", paint(title, Style::section(use_color)));
        println!(
            "{}",
            paint(&"-".repeat(title.chars().count()), Style::muted(use_color))
        );
    }

    println!(
        "{} {}",
        paint(
            &format!("{:<item_width$}", "Distribution"),
            Style::language(use_color)
        ),
        language_distribution_bar(&data.languages, &legend, use_color)
    );

    println!();
    println!("{}", paint("Key", Style::section(use_color)));
    println!("{}", paint("---", Style::muted(use_color)));
    for entry in &legend {
        let details = language_legend_details(entry, &data.languages);
        match details {
            Some(details) => println!(
                "{} {}  {}",
                legend_swatch(entry, use_color),
                paint(
                    &format!("{:<item_width$}", truncate_label(&entry.name, item_width)),
                    Style::value(use_color)
                ),
                paint(&details, Style::hours(use_color))
            ),
            None => println!(
                "{} {}",
                legend_swatch(entry, use_color),
                paint(&entry.name, Style::value(use_color))
            ),
        }
    }
}

fn build_language_legend(languages: &[LanguageLine]) -> Vec<GraphLegendEntry> {
    let mut legend = Vec::new();
    let mut color_index = 0;
    let mut needs_other = languages
        .iter()
        .map(|language| language.percent)
        .sum::<f64>()
        < 99.5;

    for language in languages {
        if language.name == "Other" {
            needs_other = true;
            continue;
        }

        if color_index < GRAPH_COLORS.len() {
            legend.push(GraphLegendEntry {
                name: language.name.clone(),
                color: GRAPH_COLORS[color_index],
                marker: GRAPH_MARKERS[color_index],
            });
            color_index += 1;
        } else {
            needs_other = true;
        }
    }

    if needs_other {
        legend.push(GraphLegendEntry {
            name: "Other".to_string(),
            color: 244,
            marker: '+',
        });
    }

    legend
}

fn language_distribution_bar(
    languages: &[LanguageLine],
    legend: &[GraphLegendEntry],
    use_color: bool,
) -> String {
    if legend.is_empty() {
        return paint(&"-".repeat(BAR_WIDTH), Style::muted(use_color));
    }

    let indexed = legend
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.name != "Other")
        .map(|(index, entry)| (entry.name.as_str(), index))
        .collect::<HashMap<_, _>>();

    let other_index = legend.iter().position(|entry| entry.name == "Other");
    let mut buckets = vec![0.0; legend.len()];
    let mut known_percent = 0.0;

    for language in languages {
        known_percent += language.percent;
        if let Some(&index) = indexed.get(language.name.as_str()) {
            buckets[index] += language.percent;
        } else if let Some(other_index) = other_index {
            buckets[other_index] += language.percent;
        }
    }

    if let Some(other_index) = other_index {
        let remainder = (100.0 - known_percent).max(0.0);
        buckets[other_index] += remainder;
    }

    render_stacked_bar(
        buckets
            .into_iter()
            .enumerate()
            .filter(|(_, percent)| *percent > 0.0)
            .collect(),
        legend,
        use_color,
    )
}

fn language_legend_details(entry: &GraphLegendEntry, languages: &[LanguageLine]) -> Option<String> {
    if let Some(language) = languages
        .iter()
        .find(|language| language.name == entry.name)
    {
        return Some(format!(
            "{:>5.1}%  {}",
            language.percent, language.hours_text
        ));
    }

    if entry.name == "Other" {
        let remaining_percent = (100.0
            - languages
                .iter()
                .map(|language| language.percent)
                .sum::<f64>())
        .max(0.0);
        if remaining_percent > 0.0 {
            return Some(format!("{remaining_percent:>5.1}%"));
        }
    }

    None
}

fn print_project_graphs(data: &DashboardData, use_color: bool) {
    if data.project_graphs.is_empty() {
        return;
    }

    let label_width = project_item_width(&data.project_graphs);
    let legend = build_project_legend(&data.project_graphs);

    println!();
    if let Some(title) = &data.project_graphs_title {
        println!("{}", paint(title, Style::section(use_color)));
        println!(
            "{}",
            paint(&"-".repeat(title.chars().count()), Style::muted(use_color))
        );
    }

    for project in &data.project_graphs {
        println!(
            "{} {}  {}",
            paint(
                &format!(
                    "{:<label_width$}",
                    truncate_label(&project.name, label_width)
                ),
                Style::language(use_color)
            ),
            project_bar(project, &legend, use_color),
            paint(&project.hours_text, Style::hours(use_color))
        );
    }

    if legend.is_empty() {
        return;
    }

    println!();
    println!("{}", paint("Key", Style::section(use_color)));
    println!("{}", paint("---", Style::muted(use_color)));
    for entry in &legend {
        println!(
            "{} {}",
            legend_swatch(entry, use_color),
            paint(&entry.name, Style::value(use_color))
        );
    }
}

fn print_activity_graph(graph: &ActivityGraph, use_color: bool, fetch_theme: FetchTheme) {
    if graph.weeks.is_empty() {
        return;
    }

    println!();
    println!();

    let month_labels = graph_month_labels(graph);
    println!("      {}", month_labels.join(" "));

    let day_labels = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
    for (day_index, day_label) in day_labels.iter().enumerate() {
        let cells = graph
            .weeks
            .iter()
            .map(|week| graph_cell(&week.days[day_index], graph, use_color, fetch_theme))
            .collect::<Vec<_>>();
        println!(
            "{}  {}",
            paint(day_label, Style::label(use_color)),
            cells.join(" ")
        );
    }

    println!();
    println!("{}", paint("Legend", Style::section(use_color)));
    println!("{}", paint("------", Style::muted(use_color)));
    for (level, label) in graph_legend_labels().iter().enumerate() {
        println!(
            "{} {}",
            graph_legend_cell(level, use_color, fetch_theme),
            paint(label, Style::value(use_color))
        );
    }

    println!();
    println!();
}

fn graph_month_labels(graph: &ActivityGraph) -> Vec<String> {
    let mut labels = Vec::with_capacity(graph.weeks.len());
    let mut seen_month = None;

    for week in &graph.weeks {
        let label = week
            .days
            .iter()
            .filter(|day| day.date <= graph.display_end)
            .find(|day| day.date.day() <= 7)
            .map(|day| day.date.format("%b").to_string());

        if let Some(label) = label {
            if seen_month.as_deref() != Some(label.as_str()) {
                labels.push(format!("{label:>2}"));
                seen_month = Some(label);
                continue;
            }
        }

        labels.push("  ".to_string());
    }

    labels
}

fn graph_cell(
    day: &crate::models::ActivityDay,
    graph: &ActivityGraph,
    use_color: bool,
    fetch_theme: FetchTheme,
) -> String {
    if day.date > graph.today {
        return future_graph_cell(use_color, fetch_theme);
    }

    let level = activity_level(day, graph);
    graph_legend_cell(level, use_color, fetch_theme)
}

fn future_graph_cell(use_color: bool, fetch_theme: FetchTheme) -> String {
    if use_color {
        format!(
            "{}  {}",
            ansi_bg(graph_palette(fetch_theme)[0]),
            ansi_reset()
        )
    } else {
        "░░".to_string()
    }
}

fn graph_legend_cell(level: usize, use_color: bool, fetch_theme: FetchTheme) -> String {
    if use_color {
        let color = graph_palette(fetch_theme)[level.min(4)];
        format!("{}  {}", ansi_bg(color), ansi_reset())
    } else {
        match level {
            0 => "· ".to_string(),
            1 => "░░".to_string(),
            2 => "▒▒".to_string(),
            3 => "▓▓".to_string(),
            _ => "██".to_string(),
        }
    }
}

fn activity_level(day: &crate::models::ActivityDay, graph: &ActivityGraph) -> usize {
    if day.date > graph.today
        && day.date.year() == graph.today.year()
        && day.date.month() == graph.today.month()
    {
        return 0;
    }

    if day.seconds <= 0.0 {
        return 0;
    }

    let hours = day.seconds / 3600.0;
    if hours < 1.0 {
        1
    } else if hours < 2.0 {
        2
    } else if hours < 4.0 {
        3
    } else {
        4
    }
}

fn graph_legend_labels() -> [&'static str; 5] {
    ["0h", "< 1h", "1-2h", "2-4h", "4h+"]
}

fn project_item_width(projects: &[ProjectGraphLine]) -> usize {
    projects
        .iter()
        .map(|project| project.name.chars().count())
        .max()
        .unwrap_or(14)
        .clamp(14, 20)
}

fn build_project_legend(projects: &[ProjectGraphLine]) -> Vec<ProjectLegendEntry> {
    let mut totals = HashMap::<String, f64>::new();
    let mut needs_other = false;

    for project in projects {
        let mut known_percent = 0.0;
        for language in &project.languages {
            known_percent += language.percent;
            if language.name == "Other" {
                needs_other = true;
                continue;
            }
            *totals.entry(language.name.clone()).or_insert(0.0) +=
                project.total_seconds * (language.percent / 100.0);
        }
        if known_percent < 99.5 {
            needs_other = true;
        }
    }

    let mut totals = totals.into_iter().collect::<Vec<_>>();
    totals.sort_by(|left, right| right.1.total_cmp(&left.1));

    let overflow = totals.len() > GRAPH_COLORS.len();
    let mut legend = totals
        .into_iter()
        .take(GRAPH_COLORS.len())
        .enumerate()
        .map(|(index, (name, _))| ProjectLegendEntry {
            name,
            color: GRAPH_COLORS[index],
            marker: GRAPH_MARKERS[index],
        })
        .collect::<Vec<_>>();

    if needs_other || overflow {
        legend.push(ProjectLegendEntry {
            name: "Other".to_string(),
            color: 244,
            marker: '+',
        });
    }

    legend
}

fn project_bar(
    project: &ProjectGraphLine,
    legend: &[ProjectLegendEntry],
    use_color: bool,
) -> String {
    const WIDTH: usize = 24;

    if legend.is_empty() {
        return paint(&"-".repeat(WIDTH), Style::muted(use_color));
    }

    let indexed = legend
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.name != "Other")
        .map(|(index, entry)| (entry.name.as_str(), index))
        .collect::<HashMap<_, _>>();

    let other_index = legend.iter().position(|entry| entry.name == "Other");
    let mut buckets = vec![0.0; legend.len()];
    let mut known_percent = 0.0;

    for language in &project.languages {
        known_percent += language.percent;
        if let Some(&index) = indexed.get(language.name.as_str()) {
            buckets[index] += language.percent;
        } else if let Some(other_index) = other_index {
            buckets[other_index] += language.percent;
        }
    }

    if let Some(other_index) = other_index {
        let remainder = (100.0 - known_percent).max(0.0);
        buckets[other_index] += remainder;
    }

    render_stacked_bar(
        buckets
            .into_iter()
            .enumerate()
            .filter(|(_, percent)| *percent > 0.0)
            .collect(),
        legend,
        use_color,
    )
}

fn render_stacked_bar(
    segments: Vec<(usize, f64)>,
    legend: &[GraphLegendEntry],
    use_color: bool,
) -> String {
    if segments.is_empty() {
        return paint(&"-".repeat(BAR_WIDTH), Style::muted(use_color));
    }

    let widths = proportional_widths(
        &segments
            .iter()
            .map(|(_, percent)| *percent)
            .collect::<Vec<_>>(),
        BAR_WIDTH,
    );

    let mut bar = String::new();
    for ((index, _), width) in segments.into_iter().zip(widths) {
        if width == 0 {
            continue;
        }

        if use_color {
            bar.push_str(&ansi_bg(legend[index].color));
            bar.push_str(&" ".repeat(width));
            bar.push_str(ansi_reset());
        } else {
            bar.push_str(&legend[index].marker.to_string().repeat(width));
        }
    }

    let current_width = visible_width(&bar);
    if current_width < BAR_WIDTH {
        let filler = BAR_WIDTH - current_width;
        if use_color {
            bar.push_str(&paint(&"-".repeat(filler), Style::muted(true)));
        } else {
            bar.push_str(&"-".repeat(filler));
        }
    }

    bar
}

fn proportional_widths(values: &[f64], width: usize) -> Vec<usize> {
    if values.is_empty() || width == 0 {
        return Vec::new();
    }

    let total = values.iter().sum::<f64>();
    if total <= 0.0 {
        return vec![0; values.len()];
    }

    let raw_widths = values
        .iter()
        .map(|value| (value / total) * width as f64)
        .collect::<Vec<_>>();
    let mut widths = raw_widths
        .iter()
        .map(|value| value.floor() as usize)
        .collect::<Vec<_>>();

    let used = widths.iter().sum::<usize>();
    let mut remainder = width.saturating_sub(used);
    let mut order = raw_widths
        .iter()
        .enumerate()
        .map(|(index, raw)| (index, raw.fract()))
        .collect::<Vec<_>>();
    order.sort_by(|left, right| right.1.total_cmp(&left.1));

    for (index, _) in order {
        if remainder == 0 {
            break;
        }
        widths[index] += 1;
        remainder -= 1;
    }

    widths
}

fn legend_swatch(entry: &GraphLegendEntry, use_color: bool) -> String {
    if use_color {
        format!("{}  {}", ansi_bg(entry.color), ansi_reset())
    } else {
        format!("[{}]", entry.marker)
    }
}

#[derive(Clone)]
struct GraphLegendEntry {
    name: String,
    color: u8,
    marker: char,
}

type ProjectLegendEntry = GraphLegendEntry;

const BAR_WIDTH: usize = 24;
const GRAPH_COLORS: [u8; 8] = [203, 39, 48, 220, 176, 51, 81, 141];
const GRAPH_MARKERS: [char; 8] = ['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H'];

fn print_fetch(data: &DashboardData, use_color: bool, fetch_theme: FetchTheme) {
    let mut right = Vec::new();
    right.push(paint(
        &data.title,
        Style::fetch_title(use_color, fetch_theme),
    ));
    right.push(paint(
        &"-".repeat(data.title.chars().count()),
        Style::muted(use_color),
    ));

    for stat in &data.stats {
        right.push(fetch_row(&stat.label, &stat.value, use_color, fetch_theme));
    }

    if !data.languages.is_empty() {
        if let Some(language) = data.languages.first() {
            right.push(fetch_row(
                "Top Language",
                &format!("{} ({:.1}%)", language.name, language.percent),
                use_color,
                fetch_theme,
            ));
        }
        right.push(String::new());
        right.extend(fetch_swatches(use_color));
    }

    let logo = fetch_logo(right.len(), use_color, fetch_theme);
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

fn fetch_row(label: &str, value: &str, use_color: bool, fetch_theme: FetchTheme) -> String {
    format!(
        "{} {}",
        paint(
            &format!("{:<16}", format!("{label}:")),
            Style::fetch_label(use_color, fetch_theme)
        ),
        paint(value, Style::value(use_color))
    )
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

fn fetch_logo(height: usize, use_color: bool, fetch_theme: FetchTheme) -> Vec<String> {
    let height = height.max(8);
    let crossbar = (height / 3).max(2);
    let palette = fetch_palette(fetch_theme);
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
            let color = fetch_logo_color(row, height, palette);
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

pub(crate) fn fetch_logo_preview(height: usize, fetch_theme: FetchTheme) -> Vec<String> {
    fetch_logo(height, true, fetch_theme)
}

fn fetch_logo_color(row: usize, height: usize, palette: FetchPalette) -> u8 {
    let progress = row as f32 / height.max(1) as f32;
    if progress < 0.25 {
        palette.logo[0]
    } else if progress < 0.5 {
        palette.logo[1]
    } else if progress < 0.75 {
        palette.logo[2]
    } else {
        palette.logo[3]
    }
}

#[derive(Clone, Copy)]
struct FetchPalette {
    logo: [u8; 4],
    title: u8,
    label: u8,
}

fn fetch_palette(theme: FetchTheme) -> FetchPalette {
    match theme {
        FetchTheme::Red => FetchPalette {
            logo: [196, 203, 210, 217],
            title: 203,
            label: 210,
        },
        FetchTheme::Blue => FetchPalette {
            logo: [27, 33, 39, 45],
            title: 39,
            label: 45,
        },
        FetchTheme::Green => FetchPalette {
            logo: [28, 34, 40, 48],
            title: 40,
            label: 48,
        },
        FetchTheme::Yellow => FetchPalette {
            logo: [136, 178, 220, 228],
            title: 220,
            label: 228,
        },
        FetchTheme::Pink => FetchPalette {
            logo: [162, 169, 176, 213],
            title: 176,
            label: 213,
        },
        FetchTheme::Cyan => FetchPalette {
            logo: [30, 37, 44, 51],
            title: 44,
            label: 51,
        },
        FetchTheme::Noir => FetchPalette {
            logo: [240, 244, 248, 252],
            title: 252,
            label: 248,
        },
    }
}

fn graph_palette(theme: FetchTheme) -> [u8; 5] {
    match theme {
        FetchTheme::Red => [52, 88, 124, 167, 203],
        FetchTheme::Blue => [17, 24, 31, 38, 45],
        FetchTheme::Green => [22, 28, 34, 40, 48],
        FetchTheme::Yellow => [58, 100, 142, 184, 228],
        FetchTheme::Pink => [53, 89, 125, 176, 213],
        FetchTheme::Cyan => [23, 30, 37, 44, 51],
        FetchTheme::Noir => [236, 239, 242, 245, 250],
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

fn ansi_bg(code: u8) -> String {
    format!("\x1b[48;5;{code}m")
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
    fn fetch_title(enabled: bool, theme: FetchTheme) -> Self {
        Self {
            color: fetch_palette(theme).title,
            bold: true,
            enabled,
        }
    }

    fn fetch_label(enabled: bool, theme: FetchTheme) -> Self {
        Self {
            color: fetch_palette(theme).label,
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
