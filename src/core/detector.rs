use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::config::schema::AgentConfig;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DetectedAgent {
    pub id: String,
    pub display_name: String,
    pub marker_path: PathBuf,
}

/// Checks if a specific agent has active configuration or workspace directories in the given project.
pub fn is_agent_detected(
    project_dir: &Path,
    agent_id: &str,
    agent_cfg: &AgentConfig,
) -> Option<PathBuf> {
    // 1. Direct configured project directory
    let direct_path = project_dir.join(&agent_cfg.project_dir);
    if direct_path.exists() {
        return Some(direct_path);
    }

    // 2. Common agent root directories or marker files
    let markers: &[&str] = match agent_id {
        "cursor" => &[".cursor", ".cursorrules", ".cursor/rules"],
        "claude-code" => &[".claude", "CLAUDE.md", ".claude/skills"],
        "gemini-cli" => &[".agent", ".gemini", ".agent/skills"],
        "universal-agent" => &[".agents", ".agents/skills"],
        _ => &[],
    };

    for marker in markers {
        let p = project_dir.join(marker);
        if p.exists() {
            return Some(p);
        }
    }

    None
}

/// Scans the project directory against configured agents and returns all detected agents.
pub fn detect_agents_in_project(
    project_dir: &Path,
    agents: &BTreeMap<String, AgentConfig>,
) -> Vec<DetectedAgent> {
    let mut detected = Vec::new();

    for (id, cfg) in agents {
        if !cfg.enabled {
            continue;
        }
        if let Some(marker_path) = is_agent_detected(project_dir, id, cfg) {
            detected.push(DetectedAgent {
                id: id.clone(),
                display_name: cfg.display_name.clone(),
                marker_path,
            });
        }
    }

    detected
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_detect_agents() {
        let temp_dir = std::env::temp_dir().join(format!(
            "asm_test_detector_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(temp_dir.join(".cursor")).unwrap();
        fs::create_dir_all(temp_dir.join(".claude").join("skills")).unwrap();

        let mut agents = BTreeMap::new();
        agents.insert(
            "cursor".to_string(),
            AgentConfig {
                display_name: "Cursor".to_string(),
                enabled: true,
                global_dir: "~/.cursor/rules".to_string(),
                project_dir: ".cursor/rules".to_string(),
                packaging: crate::config::schema::PackagingStrategy::File,
                target_file: "{skill_name}.mdc".to_string(),
            },
        );
        agents.insert(
            "claude-code".to_string(),
            AgentConfig {
                display_name: "Claude Code".to_string(),
                enabled: true,
                global_dir: "~/.claude/skills".to_string(),
                project_dir: ".claude/skills".to_string(),
                packaging: crate::config::schema::PackagingStrategy::Directory,
                target_file: "SKILL.md".to_string(),
            },
        );
        agents.insert(
            "gemini-cli".to_string(),
            AgentConfig {
                display_name: "Gemini CLI".to_string(),
                enabled: true,
                global_dir: "~/.gemini/skills".to_string(),
                project_dir: ".agent/skills".to_string(),
                packaging: crate::config::schema::PackagingStrategy::Directory,
                target_file: "SKILL.md".to_string(),
            },
        );

        let detected = detect_agents_in_project(&temp_dir, &agents);
        assert_eq!(detected.len(), 2);
        assert!(detected.iter().any(|d| d.id == "cursor"));
        assert!(detected.iter().any(|d| d.id == "claude-code"));
        assert!(!detected.iter().any(|d| d.id == "gemini-cli"));

        let _ = fs::remove_dir_all(temp_dir);
    }
}
