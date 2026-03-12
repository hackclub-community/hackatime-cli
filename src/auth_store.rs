use std::{fs, path::PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredAuth {
    access_token: String,
}

pub fn load_access_token() -> Result<Option<String>> {
    let path = auth_file_path()?;
    if !path.exists() {
        return Ok(None);
    }

    let contents = fs::read_to_string(&path)
        .with_context(|| format!("failed to read auth file at {}", path.display()))?;
    let auth: StoredAuth = serde_json::from_str(&contents)
        .with_context(|| format!("failed to parse auth file at {}", path.display()))?;
    Ok(Some(auth.access_token))
}

pub fn save_access_token(access_token: &str) -> Result<()> {
    let path = auth_file_path()?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create auth dir at {}", parent.display()))?;
    }

    let auth = StoredAuth {
        access_token: access_token.to_string(),
    };
    let json = serde_json::to_string_pretty(&auth).context("failed to serialize auth token")?;
    fs::write(&path, json)
        .with_context(|| format!("failed to write auth file at {}", path.display()))
}

pub fn clear_access_token() -> Result<()> {
    let path = auth_file_path()?;
    if path.exists() {
        fs::remove_file(&path)
            .with_context(|| format!("failed to remove auth file at {}", path.display()))?;
    }
    Ok(())
}

fn auth_file_path() -> Result<PathBuf> {
    let config_dir =
        dirs::config_dir().context("could not determine config directory for auth storage")?;
    Ok(config_dir
        .join("hackatime-viewer-terminal")
        .join("auth.json"))
}
