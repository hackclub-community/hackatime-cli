use std::path::PathBuf;

use anyhow::{Context, Result};

const APP_DIR_NAME: &str = "hackatime-viewer-terminal";
pub const CONFIG_DIR_ENV: &str = "HACKATIME_CONFIG_DIR";

pub fn app_config_dir() -> Result<PathBuf> {
    if let Ok(path) = std::env::var(CONFIG_DIR_ENV) {
        let trimmed = path.trim();
        if trimmed.is_empty() {
            anyhow::bail!("{CONFIG_DIR_ENV} is set but empty");
        }
        return Ok(PathBuf::from(trimmed));
    }

    let config_dir =
        dirs::config_dir().context("could not determine config directory for Hackatime storage")?;
    Ok(config_dir.join(APP_DIR_NAME))
}
