mod api;
mod auth_store;
mod config;
mod models;
mod oauth;
mod output;

use anyhow::{Context, Result};

use crate::{
    api::{HackatimeClient, ReportMode},
    config::AppConfig,
};

enum Command {
    Dashboard(ReportMode),
    FolderProject(String),
    Logout,
}

#[tokio::main]
async fn main() -> Result<()> {
    let command = parse_command(std::env::args().skip(1))?;

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

    println!("Fetching your Hackatime stats...");
    let client = HackatimeClient::new(access_token.clone());
    let dashboard = match fetch_for_command(&client, &command).await {
        Ok(dashboard) => dashboard,
        Err(error) if is_unauthorized_error(&error) => {
            auth_store::clear_access_token()?;
            println!("Saved login expired or was revoked. Re-authenticating...");
            let fresh_token = authenticate_and_store(&config).await?;
            println!("Fetching your Hackatime stats...");
            let client = HackatimeClient::new(fresh_token);
            fetch_for_command(&client, &command)
                .await
                .context("failed to fetch dashboard data after re-authentication")?
        }
        Err(error) => return Err(error).context("failed to fetch dashboard data"),
    };

    output::print_dashboard(&dashboard);
    Ok(())
}

fn parse_command(args: impl Iterator<Item = String>) -> Result<Command> {
    let mut mode = ReportMode::Summary;

    for arg in args {
        mode = match arg.as_str() {
            "logout" => return Ok(Command::Logout),
            "." => return Ok(Command::FolderProject(current_folder_project_name()?)),
            "--fetch" | "-f" => ReportMode::Fetch,
            "--current" | "-c" => ReportMode::Current,
            "--day" | "--today" | "-d" => ReportMode::Day,
            "--week" | "-w" => ReportMode::Week,
            "--month" | "-m" => ReportMode::Month,
            "--year" | "-y" => ReportMode::Year,
            "--lifetime" | "-l" => ReportMode::Lifetime,
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            _ => anyhow::bail!(
                "unknown argument: {arg}\n\nUse `.`, `logout`, --fetch/-f, --current/-c, --today/-d, --week/-w, --month/-m, --year/-y, or --lifetime/-l."
            ),
        };
    }

    Ok(Command::Dashboard(mode))
}

fn print_help() {
    println!("hackatime");
    println!();
    println!("Usage:");
    println!("  hackatime");
    println!("  hackatime .");
    println!("  hackatime logout");
    println!("  hackatime --fetch (-f)");
    println!("  hackatime --current (-c)");
    println!("  hackatime --today (-d)");
    println!("  hackatime --week (-w)");
    println!("  hackatime --month (-m)");
    println!("  hackatime --year (-y)");
    println!("  hackatime --lifetime (-l)");
}

async fn fetch_for_command(
    client: &HackatimeClient,
    command: &Command,
) -> Result<crate::models::DashboardData> {
    match command {
        Command::Dashboard(mode) => client.fetch_dashboard(*mode).await,
        Command::FolderProject(project_name) => {
            client.fetch_named_project_report(project_name).await
        }
        Command::Logout => unreachable!(),
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
