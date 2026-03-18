mod api;
mod auth_store;
mod config;
mod models;
mod oauth;
mod output;
mod settings;
mod storage;

use anyhow::{Context, Result};

use crate::{
    api::{HackatimeClient, ReportMode},
    config::AppConfig,
};

enum Command {
    Dashboard {
        mode: ReportMode,
        lookup_username: Option<String>,
    },
    FolderProject {
        project_name: String,
        lookup_username: Option<String>,
    },
    Logout,
    Settings(SettingsCommand),
}

enum SettingsCommand {
    Show,
    ColorPicker,
    SetClearTerminal(bool),
    ToggleClearTerminal,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum ViewSelection {
    Summary,
    Fetch,
    Graph,
    Projects,
    Current,
    Range,
}

#[derive(Clone, Copy)]
enum RangeSelection {
    Day,
    Week,
    Month,
    Year,
    Lifetime,
}

#[tokio::main]
async fn main() -> Result<()> {
    let mut user_settings = settings::load_settings()?;
    let command = parse_command(std::env::args().skip(1))?;

    if let Command::Settings(settings_command) = command {
        run_settings_command(settings_command, &mut user_settings)?;
        return Ok(());
    }

    if matches!(command, Command::Logout) {
        auth_store::clear_access_token()?;
        println!("Logged out of Hackatime.");
        return Ok(());
    }

    let config = AppConfig::load()?;
    let access_token = match auth_store::load_access_token()? {
        Some(saved_token) => saved_token,
        None => authenticate_and_store(&config).await?,
    };

    println!("{}", fetch_message(&command));
    let client = HackatimeClient::new(access_token.clone());
    let dashboard = match fetch_for_command(&client, &command).await {
        Ok(dashboard) => dashboard,
        Err(error) if is_unauthorized_error(&error) => {
            auth_store::clear_access_token()?;
            println!("Saved login expired or was revoked. Re-authenticating...");
            let fresh_token = authenticate_and_store(&config).await?;
            println!("{}", fetch_message(&command));
            let client = HackatimeClient::new(fresh_token);
            fetch_for_command(&client, &command)
                .await
                .context("failed to fetch dashboard data after re-authentication")?
        }
        Err(error) => return Err(error).context("failed to fetch dashboard data"),
    };

    output::print_dashboard(
        &dashboard,
        user_settings.fetch_theme,
        user_settings.clear_terminal,
    );
    Ok(())
}

fn parse_command(args: impl Iterator<Item = String>) -> Result<Command> {
    let args = args.collect::<Vec<_>>();
    let mut view = ViewSelection::Summary;
    let mut range = None;
    let mut lookup_username = None;
    let mut folder_project = None;
    let mut simple_command = None;

    let mut index = 0;
    while index < args.len() {
        let arg = &args[index];
        match arg.as_str() {
            "logout" => simple_command = Some(Command::Logout),
            "settings" => {
                let remaining_args = &args[index + 1..];
                return Ok(Command::Settings(parse_settings_command(remaining_args)?));
            }
            "." => folder_project = Some(current_folder_project_name()?),
            "--lookup" => {
                index += 1;
                let username = args.get(index).context("missing username after --lookup")?;
                lookup_username = Some(username.to_string());
            }
            "--fetch" | "-f" => view = ViewSelection::Fetch,
            "--graph" | "-g" => view = ViewSelection::Graph,
            "--projects" | "-p" => view = ViewSelection::Projects,
            "--current" | "-c" => view = ViewSelection::Current,
            "--day" | "--today" | "-d" => {
                range = Some(RangeSelection::Day);
                if !matches!(view, ViewSelection::Projects) {
                    view = ViewSelection::Range;
                }
            }
            "--week" | "-w" => {
                range = Some(RangeSelection::Week);
                if !matches!(view, ViewSelection::Projects) {
                    view = ViewSelection::Range;
                }
            }
            "--month" | "-m" => {
                range = Some(RangeSelection::Month);
                if !matches!(view, ViewSelection::Projects) {
                    view = ViewSelection::Range;
                }
            }
            "--year" | "-y" => {
                range = Some(RangeSelection::Year);
                if !matches!(view, ViewSelection::Projects) {
                    view = ViewSelection::Range;
                }
            }
            "--lifetime" | "-l" => {
                range = Some(RangeSelection::Lifetime);
                if !matches!(view, ViewSelection::Projects) {
                    view = ViewSelection::Range;
                }
            }
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            _ => anyhow::bail!(
                "unknown argument: {arg}\n\nUse `.`, `settings`, `settings color`, `settings clear <on|off|toggle>`, `logout`, --lookup <username>, --fetch/-f, --graph/-g, --projects/-p, --current/-c, --today/-d, --week/-w, --month/-m, --year/-y, or --lifetime/-l."
            ),
        }
        index += 1;
    }

    if let Some(command) = simple_command {
        if lookup_username.is_some()
            || folder_project.is_some()
            || !matches!(view, ViewSelection::Summary)
            || range.is_some()
        {
            anyhow::bail!("`logout` cannot be combined with other arguments.");
        }
        return Ok(command);
    }

    let mode = match view {
        ViewSelection::Summary => ReportMode::Summary,
        ViewSelection::Fetch => ReportMode::Fetch,
        ViewSelection::Graph => ReportMode::Graph,
        ViewSelection::Current => ReportMode::Current,
        ViewSelection::Range => match range.unwrap_or(RangeSelection::Lifetime) {
            RangeSelection::Day => ReportMode::Day,
            RangeSelection::Week => ReportMode::Week,
            RangeSelection::Month => ReportMode::Month,
            RangeSelection::Year => ReportMode::Year,
            RangeSelection::Lifetime => ReportMode::Lifetime,
        },
        ViewSelection::Projects => match range.unwrap_or(RangeSelection::Lifetime) {
            RangeSelection::Week => ReportMode::ProjectsWeek,
            RangeSelection::Month => ReportMode::ProjectsMonth,
            RangeSelection::Year => ReportMode::ProjectsYear,
            RangeSelection::Lifetime => ReportMode::Projects,
            RangeSelection::Day => {
                anyhow::bail!(
                    "`--projects` currently supports --week, --month, --year, or --lifetime."
                );
            }
        },
    };

    if let Some(project_name) = folder_project {
        return Ok(Command::FolderProject {
            project_name,
            lookup_username,
        });
    }

    Ok(Command::Dashboard {
        mode,
        lookup_username,
    })
}

fn parse_settings_command(args: &[String]) -> Result<SettingsCommand> {
    match args {
        [] => Ok(SettingsCommand::Show),
        [subcommand] if matches!(subcommand.as_str(), "color") => Ok(SettingsCommand::ColorPicker),
        [subcommand, value]
            if matches!(
                subcommand.as_str(),
                "clear" | "clear-terminal" | "clear_terminal"
            ) =>
        {
            match value.as_str() {
                "on" | "true" | "yes" => Ok(SettingsCommand::SetClearTerminal(true)),
                "off" | "false" | "no" => Ok(SettingsCommand::SetClearTerminal(false)),
                "toggle" => Ok(SettingsCommand::ToggleClearTerminal),
                _ => anyhow::bail!(
                    "invalid clear-terminal value: {value}\n\nUse `on`, `off`, or `toggle`."
                ),
            }
        }
        [subcommand]
            if matches!(
                subcommand.as_str(),
                "clear" | "clear-terminal" | "clear_terminal"
            ) =>
        {
            Ok(SettingsCommand::ToggleClearTerminal)
        }
        _ => anyhow::bail!(
            "unknown settings command.\n\nUse `hackatime settings`, `hackatime settings color`, or `hackatime settings clear <on|off|toggle>`."
        ),
    }
}

fn run_settings_command(
    command: SettingsCommand,
    user_settings: &mut settings::UserSettings,
) -> Result<()> {
    match command {
        SettingsCommand::Show => {
            println!("Hackatime Settings");
            println!("==================");
            println!("{}", settings::format_settings_summary(user_settings));
            println!();
            println!("Commands:");
            println!("  hackatime settings color");
            println!("  hackatime settings clear <on|off|toggle>");
        }
        SettingsCommand::ColorPicker => {
            if settings::open_color_picker(user_settings)? {
                println!("Saved settings.");
                println!("{}", settings::format_settings_summary(user_settings));
            } else {
                println!("Color selection canceled.");
            }
        }
        SettingsCommand::SetClearTerminal(clear_terminal) => {
            user_settings.clear_terminal = clear_terminal;
            settings::save_settings(user_settings)?;
            println!("Saved settings.");
            println!("{}", settings::format_settings_summary(user_settings));
        }
        SettingsCommand::ToggleClearTerminal => {
            user_settings.clear_terminal = !user_settings.clear_terminal;
            settings::save_settings(user_settings)?;
            println!("Saved settings.");
            println!("{}", settings::format_settings_summary(user_settings));
        }
    }

    Ok(())
}

fn print_help() {
    println!("Usage:");
    println!("  hackatime [command] [flags]");
    println!();
    println!("Main Commands:");
    println!("  hackatime");
    println!("      Show the default dashboard with multi-range totals and lifetime languages.");
    println!("  hackatime .");
    println!(
        "      Show stats for the current folder if it matches one of your Hackatime projects."
    );
    println!("  hackatime logout");
    println!("      Clear your saved Hackatime login.");
    println!();
    println!("Settings:");
    println!("  hackatime settings");
    println!("      Show your saved settings and the settings file path.");
    println!("  hackatime settings color");
    println!("      Open the interactive color picker for the fetch theme.");
    println!("  hackatime settings clear on");
    println!("  hackatime settings clear off");
    println!("  hackatime settings clear toggle");
    println!("      Control whether the terminal is cleared before output is shown.");
    println!();
    println!("Personal Views:");
    println!("  hackatime --fetch, -f");
    println!("      Show the neofetch-style overview.");
    println!("  hackatime --graph, -g");
    println!("      Show the GitHub-style coding heatmap for the last 365 days.");
    println!("  hackatime --projects, -p");
    println!("      Show your top projects with per-project language breakdowns.");
    println!("  hackatime --current, -c");
    println!("      Show the current project report.");
    println!("  hackatime --today, -d");
    println!("      Show today's total plus today's languages.");
    println!("  hackatime --week, -w");
    println!("      Show this week's total plus this week's languages.");
    println!("  hackatime --month, -m");
    println!("      Show this month's total plus this month's languages.");
    println!("  hackatime --year, -y");
    println!("      Show this year's total plus this year's languages.");
    println!("  hackatime --lifetime, -l");
    println!("      Show lifetime total plus lifetime languages.");
    println!();
    println!("Project Range Flags:");
    println!("  hackatime --projects --week");
    println!("  hackatime --projects --month");
    println!("  hackatime --projects --year");
    println!("  hackatime --projects --lifetime");
    println!("      Scope the projects view to a time range.");
    println!();
    println!("Lookup Another User:");
    println!("  hackatime --lookup <username>");
    println!("      Show another person's public Hackatime stats.");
    println!("  hackatime --lookup <username> -g");
    println!("      Show another person's graph.");
    println!("  hackatime --lookup <username> --projects --year");
    println!("      Show another person's projects for a selected range.");
    println!("  hackatime --lookup <username> .");
    println!("      Show a matching project in that user's public stats.");
    println!();
    println!("Help:");
    println!("  hackatime --help, -h");
    println!("      Show this help message.");
}

async fn fetch_for_command(
    client: &HackatimeClient,
    command: &Command,
) -> Result<crate::models::DashboardData> {
    match command {
        Command::Dashboard {
            mode,
            lookup_username,
            ..
        } => {
            if let Some(username) = lookup_username {
                client.fetch_lookup_dashboard(*mode, username).await
            } else {
                client.fetch_dashboard(*mode).await
            }
        }
        Command::FolderProject {
            project_name,
            lookup_username,
        } => {
            if let Some(username) = lookup_username {
                client
                    .fetch_lookup_named_project_report(username, project_name)
                    .await
            } else {
                client.fetch_named_project_report(project_name).await
            }
        }
        Command::Logout | Command::Settings(_) => unreachable!(),
    }
}

fn current_folder_project_name() -> Result<String> {
    let current_dir = std::env::current_dir().context("failed to determine current directory")?;
    let folder_name = current_dir
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .context("could not determine current folder name")?;
    Ok(folder_name.to_string())
}

async fn authenticate_and_store(config: &AppConfig) -> Result<String> {
    println!("Starting OAuth login for Hackatime...");
    let access_token = oauth::authorize(config)
        .await
        .context("failed to complete OAuth login")?;
    auth_store::save_access_token(&access_token)?;
    Ok(access_token)
}

fn is_unauthorized_error(error: &anyhow::Error) -> bool {
    error.chain().any(|cause| {
        cause.to_string().contains("401 Unauthorized")
            || cause
                .to_string()
                .contains("status client error (401 Unauthorized)")
    })
}

fn fetch_message(command: &Command) -> String {
    match command {
        Command::Dashboard {
            lookup_username: Some(username),
            ..
        }
        | Command::FolderProject {
            lookup_username: Some(username),
            ..
        } => format!("Fetching Hackatime stats for {username}..."),
        Command::Dashboard { .. } | Command::FolderProject { .. } => {
            "Fetching your Hackatime stats...".to_string()
        }
        Command::Logout | Command::Settings(_) => String::new(),
    }
}
