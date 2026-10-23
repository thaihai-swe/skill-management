use crate::config::schema::{
    AgentConfig, AppConfig, DeployMode, PackagingStrategy, SettingsConfig,
};
use std::collections::BTreeMap;

pub fn get_default_config() -> AppConfig {
    let mut agents = BTreeMap::new();

    agents.insert(
        "universal-agent".to_string(),
        AgentConfig {
            display_name: "Universal Agent (OpenAI Standard)".to_string(),
            enabled: true,
            global_dir: "~/.agents/skills".to_string(),
            project_dir: ".agents/skills".to_string(),
            packaging: PackagingStrategy::Directory,
            target_file: "SKILL.md".to_string(),
        },
    );

    agents.insert(
        "claude-code".to_string(),
        AgentConfig {
            display_name: "Claude Code".to_string(),
            enabled: true,
            global_dir: "~/.claude/skills".to_string(),
            project_dir: ".claude/skills".to_string(),
            packaging: PackagingStrategy::Directory,
            target_file: "SKILL.md".to_string(),
        },
    );

    agents.insert(
        "cursor".to_string(),
        AgentConfig {
            display_name: "Cursor".to_string(),
            enabled: true,
            global_dir: "~/.cursor/rules".to_string(),
            project_dir: ".cursor/rules".to_string(),
            packaging: PackagingStrategy::File,
            target_file: "{skill_name}.mdc".to_string(),
        },
    );

    agents.insert(
        "gemini-cli".to_string(),
        AgentConfig {
            display_name: "Gemini CLI / Antigravity".to_string(),
            enabled: true,
            global_dir: "~/.gemini/antigravity-cli/skills".to_string(),
            project_dir: ".agent/skills".to_string(),
            packaging: PackagingStrategy::Directory,
            target_file: "SKILL.md".to_string(),
        },
    );

    agents.insert(
        "custom-agent".to_string(),
        AgentConfig {
            display_name: "Custom Agent (Example)".to_string(),
            enabled: false,
            global_dir: "~/.config/custom-agent/skills".to_string(),
            project_dir: ".skills".to_string(),
            packaging: PackagingStrategy::Directory,
            target_file: "SKILL.md".to_string(),
        },
    );

    AppConfig {
        version: 1,
        settings: SettingsConfig {
            default_deployment_mode: DeployMode::Symlink,
            confirm_destructive_actions: true,
            sources_cache_dir: "~/.config/asm/sources".to_string(),
            theme: "default".to_string(),
        },
        agents,
    }
}
