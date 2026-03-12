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

#[tokio::main]
async fn main() -> Result<()> {
    let config = AppConfig::load()?;
    let mode = parse_mode(std::env::args().skip(1))?;
    let access_token = match auth_store::load_access_token()? {
        Some(saved_token) => saved_token,
        None => authenticate_and_store(&config).await?,
    };

    println!("Fetching your Hackatime stats...");
    let client = HackatimeClient::new(access_token.clone());
    let dashboard = match client.fetch_dashboard(mode).await {
        Ok(dashboard) => dashboard,
        Err(error) if is_unauthorized_error(&error) => {
            auth_store::clear_access_token()?;
            println!("Saved login expired or was revoked. Re-authenticating...");
            let fresh_token = authenticate_and_store(&config).await?;
            println!("Fetching your Hackatime stats...");
            let client = HackatimeClient::new(fresh_token);
            client
                .fetch_dashboard(mode)
                .await
                .context("failed to fetch dashboard data after re-authentication")?
        }
        Err(error) => return Err(error).context("failed to fetch dashboard data"),
    };

    output::print_dashboard(&dashboard);
    Ok(())
}

fn parse_mode(args: impl Iterator<Item = String>) -> Result<ReportMode> {
    let mut mode = ReportMode::Summary;

    for arg in args {
        mode = match arg.as_str() {
            "--current" => ReportMode::Current,
            "--day" | "--today" => ReportMode::Day,
            "--week" => ReportMode::Week,
            "--month" => ReportMode::Month,
            "--year" => ReportMode::Year,
            "--lifetime" => ReportMode::Lifetime,
            "--help" | "-h" => {
                print_help();
                std::process::exit(0);
            }
            _ => anyhow::bail!(
                "unknown argument: {arg}\n\nUse --current, --today, --week, --month, --year, or --lifetime."
            ),
        };
    }

    Ok(mode)
}

fn print_help() {
    println!("hackatime");
    println!();
    println!("Usage:");
    println!("  hackatime");
    println!("  hackatime --current");
    println!("  hackatime --today");
    println!("  hackatime --week");
    println!("  hackatime --month");
    println!("  hackatime --year");
    println!("  hackatime --lifetime");
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
