use std::path::Path;
use walkdir::WalkDir;

use crate::config::schema::{AgentConfig, PackagingStrategy};
use crate::core::skill::Skill;

const IGNORED_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "dist",
    ".venv",
    "venv",
    "vendor",
    ".idea",
    ".vscode",
];

pub fn scan_skills_in_dir(source_alias: &str, root_dir: &Path) -> Vec<Skill> {
    let mut skills = Vec::new();
    if !root_dir.exists() || !root_dir.is_dir() {
        return skills;
    }

    let it = WalkDir::new(root_dir)
        .min_depth(0)
        .max_depth(6)
        .into_iter()
        .filter_entry(|entry| {
            if let Some(name) = entry.file_name().to_str() {
                !IGNORED_DIRS.contains(&name)
            } else {
                true
            }
        });

    for entry in it.flatten() {
        if entry.file_type().is_dir() {
            let skill_md = entry.path().join("SKILL.md");
            if skill_md.is_file() {
                if let Ok(skill) = Skill::parse_from_dir(source_alias, entry.path()) {
                    skills.push(skill);
                }
            }
        }
    }

    skills
}

pub fn scan_installed_agent_skills(
    agent_cfg: &AgentConfig,
    base_dir: &Path,
    scope_tag: &str,
) -> Vec<Skill> {
    let mut skills = Vec::new();
    if !base_dir.exists() || !base_dir.is_dir() {
        return skills;
    }

    match agent_cfg.packaging {
        PackagingStrategy::Directory => {
            if let Ok(entries) = std::fs::read_dir(base_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let skill_md = path.join("SKILL.md");
                    if skill_md.is_file() {
                        if let Ok(skill) = Skill::parse_from_dir(scope_tag, &path) {
                            skills.push(skill);
                        }
                    }
                }
            }
        }
        PackagingStrategy::File => {
            if let Ok(entries) = std::fs::read_dir(base_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
                        if ext == "mdc" || ext == "md" {
                            let name = path
                                .file_stem()
                                .map(|s| s.to_string_lossy().to_string())
                                .unwrap_or_default();
                            let content = std::fs::read_to_string(&path).unwrap_or_default();
                            skills.push(Skill {
                                id: format!("{}/{}", scope_tag, name),
                                name: name.clone(),
                                source_alias: scope_tag.to_string(),
                                path: path.clone(),
                                description: format!(
                                    "Rule deployed for {}",
                                    agent_cfg.display_name
                                ),
                                version: None,
                                tags: vec!["file-rule".to_string()],
                                author: None,
                                raw_content: content,
                            });
                        }
                    }
                }
            }
        }
    }

    skills
}
