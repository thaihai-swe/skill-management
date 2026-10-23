use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::config::loader::get_asm_dir;
use crate::config::schema::DeployMode;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Scope {
    Global,
    Project(PathBuf),
}

impl std::fmt::Display for Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Scope::Global => write!(f, "Global"),
            Scope::Project(p) => {
                let name = p
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| p.to_string_lossy().to_string());
                write!(f, "Project ({})", name)
            }
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum DeployStatus {
    Enabled,
    Disabled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeploymentEntry {
    pub skill_id: String,
    pub source_alias: String,
    pub agent_id: String,
    pub scope: Scope,
    pub source_path: PathBuf,
    pub target_path: PathBuf,
    pub mode: DeployMode,
    pub status: DeployStatus,
    pub vault_path: Option<PathBuf>,
    pub deployed_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DeployManifest {
    pub deployments: Vec<DeploymentEntry>,
}

impl DeployManifest {
    pub fn get_manifest_path() -> PathBuf {
        get_asm_dir().join("deployments.json")
    }

    pub fn load_or_init() -> Result<Self> {
        let path = Self::get_manifest_path();
        if !path.exists() {
            let manifest = DeployManifest::default();
            manifest.save()?;
            return Ok(manifest);
        }
        let content = fs::read_to_string(&path)?;
        let manifest: DeployManifest = serde_json::from_str(&content)?;
        Ok(manifest)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::get_manifest_path();
        let serialized = serde_json::to_string_pretty(self)?;
        crate::deploy::fs_ops::write_atomic(&path, &serialized)?;
        Ok(())
    }

    pub fn find_entry(
        &self,
        skill_id: &str,
        agent_id: &str,
        scope: &Scope,
    ) -> Option<&DeploymentEntry> {
        self.deployments
            .iter()
            .find(|d| d.skill_id == skill_id && d.agent_id == agent_id && d.scope == *scope)
    }

    pub fn find_entry_mut(
        &mut self,
        skill_id: &str,
        agent_id: &str,
        scope: &Scope,
    ) -> Option<&mut DeploymentEntry> {
        self.deployments
            .iter_mut()
            .find(|d| d.skill_id == skill_id && d.agent_id == agent_id && d.scope == *scope)
    }

    pub fn get_deployments_for_skill(&self, skill_id: &str) -> Vec<&DeploymentEntry> {
        self.deployments
            .iter()
            .filter(|d| d.skill_id == skill_id)
            .collect()
    }

    pub fn upsert(&mut self, entry: DeploymentEntry) -> Result<()> {
        self.deployments.retain(|d| {
            !(d.skill_id == entry.skill_id
                && d.agent_id == entry.agent_id
                && d.scope == entry.scope)
        });
        self.deployments.push(entry);
        self.save()
    }

    pub fn remove(&mut self, skill_id: &str, agent_id: &str, scope: &Scope) -> Result<()> {
        self.deployments
            .retain(|d| !(d.skill_id == skill_id && d.agent_id == agent_id && d.scope == *scope));
        self.save()
    }
}
