use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::schema::AppConfig;

pub fn get_asm_dir() -> PathBuf {
    #[cfg(windows)]
    {
        if let Some(config_dir) = dirs::config_dir() {
            return config_dir.join("asm");
        }
    }
    if let Some(home) = dirs::home_dir() {
        home.join(".config").join("asm")
    } else if let Some(config_dir) = dirs::config_dir() {
        config_dir.join("asm")
    } else {
        PathBuf::from(".asm")
    }
}

pub fn get_config_path() -> PathBuf {
    get_asm_dir().join("config.json")
}

pub fn expand_path<P: AsRef<Path>>(path: P) -> PathBuf {
    let p = path.as_ref();
    let path_str = p.to_string_lossy();
    if path_str.starts_with("~/") || path_str == "~" {
        if let Some(home) = dirs::home_dir() {
            if path_str == "~" {
                return home;
            } else {
                return home.join(&path_str[2..]);
            }
        }
    }
    p.to_path_buf()
}

pub fn load_or_init_config() -> Result<AppConfig> {
    let config_path = get_config_path();
    if !config_path.exists() {
        let asm_dir = get_asm_dir();
        fs::create_dir_all(&asm_dir)
            .with_context(|| format!("Failed to create asm config directory at {:?}", asm_dir))?;

        let default_config = crate::config::defaults::get_default_config();
        let serialized = serde_json::to_string_pretty(&default_config)
            .context("Failed to serialize default config")?;
        fs::write(&config_path, serialized)
            .with_context(|| format!("Failed to write default config to {:?}", config_path))?;
        return Ok(default_config);
    }

    let contents = fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config from {:?}", config_path))?;
    let mut config: AppConfig = serde_json::from_str(&contents)
        .with_context(|| format!("Failed to parse config JSON at {:?}", config_path))?;

    // Auto-merge missing default agents so users don't miss new standard agents
    let defaults = crate::config::defaults::get_default_config();
    let mut updated = false;
    for (key, default_agent) in defaults.agents {
        if let std::collections::btree_map::Entry::Vacant(e) = config.agents.entry(key) {
            e.insert(default_agent);
            updated = true;
        }
    }
    if updated {
        let _ = save_config(&config);
    }

    Ok(config)
}

#[allow(dead_code)]
pub fn save_config(config: &AppConfig) -> Result<()> {
    let config_path = get_config_path();
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let serialized = serde_json::to_string_pretty(config)?;
    fs::write(&config_path, serialized)?;
    Ok(())
}
