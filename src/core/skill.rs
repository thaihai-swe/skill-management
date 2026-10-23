use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SkillFrontmatter {
    pub name: Option<String>,
    pub description: Option<String>,
    pub version: Option<String>,
    pub tags: Option<Vec<String>>,
    pub author: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Skill {
    pub id: String,
    pub name: String,
    pub source_alias: String,
    pub path: PathBuf,
    pub description: String,
    pub version: Option<String>,
    pub tags: Vec<String>,
    pub author: Option<String>,
    pub raw_content: String,
}

impl Skill {
    pub fn parse_from_dir<P: AsRef<Path>>(source_alias: &str, dir_path: P) -> Result<Self> {
        let dir = dir_path.as_ref();
        let skill_md_path = dir.join("SKILL.md");
        let content = if skill_md_path.exists() {
            fs::read_to_string(&skill_md_path)?
        } else {
            String::new()
        };

        let folder_name = dir
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "unnamed-skill".to_string());

        let (frontmatter, body) = parse_frontmatter_and_body(&content);

        let name = frontmatter
            .as_ref()
            .and_then(|f| f.name.clone())
            .unwrap_or_else(|| {
                // Try fallback from first markdown header
                extract_first_header(&body).unwrap_or(folder_name.clone())
            });

        let description = frontmatter
            .as_ref()
            .and_then(|f| f.description.clone())
            .unwrap_or_else(|| {
                extract_first_paragraph(&body)
                    .unwrap_or_else(|| "No description provided.".to_string())
            });

        let version = frontmatter.as_ref().and_then(|f| f.version.clone());
        let tags = frontmatter
            .as_ref()
            .and_then(|f| f.tags.clone())
            .unwrap_or_default();
        let author = frontmatter.as_ref().and_then(|f| f.author.clone());

        let id = format!("{}/{}", source_alias, folder_name);

        Ok(Skill {
            id,
            name,
            source_alias: source_alias.to_string(),
            path: dir.to_path_buf(),
            description,
            version,
            tags,
            author,
            raw_content: content,
        })
    }
}

fn parse_frontmatter_and_body(content: &str) -> (Option<SkillFrontmatter>, String) {
    let trimmed = content.trim_start();
    if let Some(stripped) = trimmed.strip_prefix("---") {
        if let Some(end_idx) = stripped.find("---") {
            let yaml_str = &stripped[..end_idx];
            let body = stripped[end_idx + 3..].trim_start().to_string();
            if let Ok(fm) = serde_yaml::from_str::<SkillFrontmatter>(yaml_str) {
                return (Some(fm), body);
            }
        }
    }
    (None, content.to_string())
}

fn extract_first_header(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            let clean = trimmed.trim_start_matches('#').trim();
            if !clean.is_empty() {
                return Some(clean.to_string());
            }
        }
    }
    None
}

fn extract_first_paragraph(content: &str) -> Option<String> {
    let mut in_paragraph = false;
    let mut paragraph_lines = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            if in_paragraph {
                break;
            }
            continue;
        }

        if trimmed.is_empty() {
            if in_paragraph {
                break;
            }
        } else {
            in_paragraph = true;
            paragraph_lines.push(trimmed);
        }
    }

    if !paragraph_lines.is_empty() {
        Some(paragraph_lines.join(" "))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frontmatter_parsing() {
        let raw = r#"---
name: my-cool-skill
description: Does awesome things
version: 1.0.0
tags: [test, skill]
author: alice
---
# Content Header
Body content here.
"#;
        let (fm, body) = parse_frontmatter_and_body(raw);
        assert!(fm.is_some());
        let fm = fm.unwrap();
        assert_eq!(fm.name.unwrap(), "my-cool-skill");
        assert_eq!(fm.description.unwrap(), "Does awesome things");
        assert_eq!(fm.version.unwrap(), "1.0.0");
        assert_eq!(fm.tags.unwrap(), vec!["test", "skill"]);
        assert_eq!(fm.author.unwrap(), "alice");
        assert!(body.contains("Body content here."));
    }

    #[test]
    fn test_markdown_fallback() {
        let raw = r#"# Fallback Title
This is the first paragraph of the description.

This is the second paragraph.
"#;
        let (fm, body) = parse_frontmatter_and_body(raw);
        assert!(fm.is_none());
        assert_eq!(
            extract_first_header(&body),
            Some("Fallback Title".to_string())
        );
        assert_eq!(
            extract_first_paragraph(&body),
            Some("This is the first paragraph of the description.".to_string())
        );
    }
}
