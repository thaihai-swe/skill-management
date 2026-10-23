use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum DeployMode {
    #[default]
    Symlink,
    Copy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum PackagingStrategy {
    #[default]
    Directory,
    File,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub display_name: String,
    pub enabled: bool,
    pub global_dir: String,
    pub project_dir: String,
    #[serde(default)]
    pub packaging: PackagingStrategy,
    #[serde(default = "default_target_file")]
    pub target_file: String,
}

fn default_target_file() -> String {
    "SKILL.md".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsConfig {
    #[serde(default)]
    pub default_deployment_mode: DeployMode,
    #[serde(default = "default_confirm_actions")]
    pub confirm_destructive_actions: bool,
    #[serde(default = "default_sources_cache_dir")]
    pub sources_cache_dir: String,
    #[serde(default = "default_theme")]
    pub theme: String,
}

fn default_confirm_actions() -> bool {
    true
}

fn default_sources_cache_dir() -> String {
    "~/.config/asm/sources".to_string()
}

fn default_theme() -> String {
    "default".to_string()
}

impl Default for SettingsConfig {
    fn default() -> Self {
        Self {
            default_deployment_mode: DeployMode::Symlink,
            confirm_destructive_actions: true,
            sources_cache_dir: default_sources_cache_dir(),
            theme: default_theme(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default)]
    pub settings: SettingsConfig,
    #[serde(default)]
    pub agents: BTreeMap<String, AgentConfig>,
}

fn default_version() -> u32 {
    1
}

impl Default for AppConfig {
    fn default() -> Self {
        crate::config::defaults::get_default_config()
    }
}
