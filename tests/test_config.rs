use asm::config::defaults::get_default_config;
use asm::config::loader::expand_path;
use asm::config::schema::{AgentConfig, AppConfig, PackagingStrategy};

#[test]
fn test_default_config_schema() {
    let config = get_default_config();
    assert_eq!(config.version, 1);
    assert!(config.agents.contains_key("universal-agent"));
    assert!(config.agents.contains_key("claude-code"));
    assert!(config.agents.contains_key("cursor"));
    assert!(config.agents.contains_key("gemini-cli"));

    let universal = config.agents.get("universal-agent").unwrap();
    assert_eq!(universal.display_name, "Universal Agent (OpenAI Standard)");
    assert_eq!(universal.project_dir, ".agents/skills");
    assert_eq!(universal.global_dir, "~/.agents/skills");
    assert_eq!(universal.packaging, PackagingStrategy::Directory);
    assert!(universal.enabled);

    let cursor = config.agents.get("cursor").unwrap();
    assert_eq!(cursor.packaging, PackagingStrategy::File);
    assert_eq!(cursor.target_file, "{skill_name}.mdc");

    let claude = config.agents.get("claude-code").unwrap();
    assert_eq!(claude.packaging, PackagingStrategy::Directory);
}

#[test]
fn test_config_json_roundtrip() {
    let mut config = get_default_config();
    config.agents.insert(
        "my-agent".to_string(),
        AgentConfig {
            display_name: "My Custom Agent".to_string(),
            enabled: true,
            global_dir: "~/.my-agent/skills".to_string(),
            project_dir: ".my-agent/skills".to_string(),
            packaging: PackagingStrategy::Directory,
            target_file: "SKILL.md".to_string(),
        },
    );

    let json = serde_json::to_string_pretty(&config).unwrap();
    let loaded: AppConfig = serde_json::from_str(&json).unwrap();
    assert!(loaded.agents.contains_key("my-agent"));
    assert_eq!(
        loaded.agents.get("my-agent").unwrap().display_name,
        "My Custom Agent"
    );
}

#[test]
fn test_expand_path() {
    let expanded = expand_path("~/some/path");
    assert!(!expanded.to_string_lossy().starts_with("~"));
}
