use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::loader::{expand_path, get_asm_dir};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SourceType {
    LocalFolder,
    LocalGit,
    RemoteGit,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum SourceScope {
    Global,
    Project,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Source {
    pub alias: String,
    pub location: String,
    pub source_type: SourceType,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default = "default_global_scope")]
    pub scope: SourceScope,
}

fn default_true() -> bool {
    true
}

fn default_global_scope() -> SourceScope {
    SourceScope::Global
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SourceRegistry {
    pub sources: Vec<Source>,
    #[serde(skip)]
    pub project_dir: Option<PathBuf>,
}

impl SourceRegistry {
    pub fn get_registry_path() -> PathBuf {
        get_asm_dir().join("sources.json")
    }

    pub fn load_with_project(project_dir: Option<&Path>) -> Result<Self> {
        let global_path = Self::get_registry_path();
        let mut sources = Vec::new();

        if global_path.exists() {
            let content = fs::read_to_string(&global_path)?;
            if let Ok(reg) = serde_json::from_str::<SourceRegistry>(&content) {
                for mut s in reg.sources {
                    s.scope = SourceScope::Global;
                    sources.push(s);
                }
            }
        }

        if let Some(p) = project_dir {
            let proj_sources_path = p.join(".asm").join("sources.json");
            if proj_sources_path.exists() {
                if let Ok(content) = fs::read_to_string(&proj_sources_path) {
                    if let Ok(reg) = serde_json::from_str::<SourceRegistry>(&content) {
                        for mut s in reg.sources {
                            s.scope = SourceScope::Project;
                            if !sources.iter().any(|existing| existing.alias == s.alias) {
                                sources.push(s);
                            }
                        }
                    }
                }
            }
        }

        Ok(Self {
            sources,
            project_dir: project_dir.map(|p| p.to_path_buf()),
        })
    }

    pub fn load_or_init() -> Result<Self> {
        let curr_dir = std::env::current_dir().ok();
        let mut reg = Self::load_with_project(curr_dir.as_deref())?;

        let global_path = Self::get_registry_path();
        if !global_path.exists() && reg.sources.is_empty() {
            let local_fixtures = PathBuf::from("./fixtures/skills");
            if local_fixtures.exists() {
                let _ = reg.add(
                    "demo-skills".to_string(),
                    "./fixtures/skills".to_string(),
                    SourceType::LocalFolder,
                    SourceScope::Global,
                );
            }
            reg.save()?;
        }
        Ok(reg)
    }

    pub fn save(&self) -> Result<()> {
        // 1. Save global sources
        let global_path = Self::get_registry_path();
        let global_sources: Vec<Source> = self
            .sources
            .iter()
            .filter(|s| s.scope == SourceScope::Global)
            .cloned()
            .collect();
        let global_reg = SourceRegistry {
            sources: global_sources,
            project_dir: None,
        };
        crate::deploy::fs_ops::write_atomic(
            &global_path,
            &serde_json::to_string_pretty(&global_reg)?,
        )?;

        // 2. Save project sources if project_dir is set
        if let Some(ref p) = self.project_dir {
            let proj_path = p.join(".asm").join("sources.json");
            let proj_sources: Vec<Source> = self
                .sources
                .iter()
                .filter(|s| s.scope == SourceScope::Project)
                .cloned()
                .collect();
            if !proj_sources.is_empty() || proj_path.exists() {
                let proj_reg = SourceRegistry {
                    sources: proj_sources,
                    project_dir: None,
                };
                crate::deploy::fs_ops::write_atomic(
                    &proj_path,
                    &serde_json::to_string_pretty(&proj_reg)?,
                )?;
            }
        }

        Ok(())
    }

    #[allow(dead_code)]
    pub fn add(
        &mut self,
        alias: String,
        location: String,
        source_type: SourceType,
        scope: SourceScope,
    ) -> Result<()> {
        if self.sources.iter().any(|s| s.alias == alias) {
            anyhow::bail!("Source with alias '{}' already exists", alias);
        }
        self.sources.push(Source {
            alias,
            location,
            source_type,
            enabled: true,
            scope,
        });
        self.save()
    }

    #[allow(dead_code)]
    pub fn remove(&mut self, alias: &str) -> Result<()> {
        self.sources.retain(|s| s.alias != alias);
        self.save()
    }

    #[allow(dead_code)]
    pub fn toggle(&mut self, alias: &str) -> Result<bool> {
        if let Some(source) = self.sources.iter_mut().find(|s| s.alias == alias) {
            source.enabled = !source.enabled;
            let status = source.enabled;
            self.save()?;
            Ok(status)
        } else {
            anyhow::bail!("Source with alias '{}' not found", alias);
        }
    }

    pub fn get_effective_path(&self, source: &Source, cache_dir: &Path) -> PathBuf {
        match source.source_type {
            SourceType::LocalFolder | SourceType::LocalGit => expand_path(&source.location),
            SourceType::RemoteGit => cache_dir.join(&source.alias),
        }
    }

    pub fn sync_remote_source(&self, source: &Source, cache_dir: &Path) -> Result<()> {
        if source.source_type != SourceType::RemoteGit {
            return Ok(());
        }

        let target_dir = cache_dir.join(&source.alias);
        fs::create_dir_all(cache_dir)?;

        if target_dir.exists() && target_dir.join(".git").exists() {
            // Git pull - capture output to prevent terminal corruption
            let output = Command::new("git")
                .arg("-C")
                .arg(&target_dir)
                .args(["pull", "--ff-only"])
                .output()
                .with_context(|| format!("Failed to pull updates for {}", source.alias))?;
            if !output.status.success() {
                let err = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!(
                    "Git pull failed for source '{}': {}",
                    source.alias,
                    err.trim()
                );
            }
        } else {
            // Git clone - capture output to prevent terminal corruption
            let output = Command::new("git")
                .args(["clone", "--depth", "1", &source.location])
                .arg(&target_dir)
                .output()
                .with_context(|| format!("Failed to clone repository {}", source.location))?;
            if !output.status.success() {
                let err = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!(
                    "Git clone failed for source '{}': {}",
                    source.alias,
                    err.trim()
                );
            }
        }
        Ok(())
    }
}
