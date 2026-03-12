use anyhow::{Context, Result};

pub const DEFAULT_CLIENT_ID: &str = "s9UiFixvPuLjs9ClHeg4HItID3X63j76XFY7AG_WEzk";
pub const DEFAULT_REDIRECT_URI: &str = "http://127.0.0.1:8787/callback";
pub const DEFAULT_SCOPES: &str = "profile read";

#[derive(Clone, Debug)]
pub struct AppConfig {
    pub client_id: String,
    pub redirect_uri: String,
    pub scopes: String,
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        let _ = dotenvy::dotenv();

        let client_id =
            std::env::var("HACKATIME_CLIENT_ID").unwrap_or_else(|_| DEFAULT_CLIENT_ID.to_string());
        let redirect_uri = std::env::var("HACKATIME_REDIRECT_URI")
            .unwrap_or_else(|_| DEFAULT_REDIRECT_URI.to_string());
        let scopes =
            std::env::var("HACKATIME_SCOPES").unwrap_or_else(|_| DEFAULT_SCOPES.to_string());

        if client_id.trim().is_empty() {
            anyhow::bail!("HACKATIME_CLIENT_ID is empty");
        }

        url::Url::parse(&redirect_uri)
            .with_context(|| format!("invalid HACKATIME_REDIRECT_URI: {redirect_uri}"))?;

        Ok(Self {
            client_id,
            redirect_uri,
            scopes,
        })
    }
}
