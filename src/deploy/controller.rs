use anyhow::{Context, Result};
use chrono::Utc;
use std::fs;
use std::path::PathBuf;

use crate::config::loader::{expand_path, get_asm_dir};
use crate::config::schema::{AgentConfig, DeployMode, PackagingStrategy};
use crate::core::skill::Skill;
use crate::deploy::fs_ops::{copy_dir_recursive, create_symlink, remove_path};
use crate::deploy::manifest::{DeployManifest, DeployStatus, DeploymentEntry, Scope};

pub struct DeployController;

impl DeployController {
    pub fn resolve_target_path(
        skill: &Skill,
        agent_config: &AgentConfig,
        scope: &Scope,
    ) -> PathBuf {
        let base_dir = match scope {
            Scope::Global => expand_path(&agent_config.global_dir),
            Scope::Project(proj) => proj.join(&agent_config.project_dir),
        };

        match agent_config.packaging {
            PackagingStrategy::Directory => {
                let folder_name = skill
                    .path
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| skill.name.clone());
                base_dir.join(folder_name)
            }
            PackagingStrategy::File => {
                let filename = agent_config
                    .target_file
                    .replace("{skill_name}", &skill.name);
                base_dir.join(filename)
            }
        }
    }

    pub fn deploy(
        manifest: &mut DeployManifest,
        skill: &Skill,
        agent_id: &str,
        agent_config: &AgentConfig,
        scope: Scope,
        mode: DeployMode,
    ) -> Result<DeploymentEntry> {
        let target_path = Self::resolve_target_path(skill, agent_config, &scope);

        let source_target = match agent_config.packaging {
            PackagingStrategy::Directory => skill.path.clone(),
            PackagingStrategy::File => skill.path.join("SKILL.md"),
        };

        match mode {
            DeployMode::Symlink => {
                create_symlink(&source_target, &target_path).with_context(|| {
                    format!("Failed to symlink {:?} to {:?}", source_target, target_path)
                })?;
            }
            DeployMode::Copy => {
                copy_dir_recursive(&source_target, &target_path).with_context(|| {
                    format!("Failed to copy {:?} to {:?}", source_target, target_path)
                })?;
            }
        }

        let entry = DeploymentEntry {
            skill_id: skill.id.clone(),
            source_alias: skill.source_alias.clone(),
            agent_id: agent_id.to_string(),
            scope,
            source_path: source_target,
            target_path,
            mode,
            status: DeployStatus::Enabled,
            vault_path: None,
            deployed_at: Utc::now().to_rfc3339(),
        };

        manifest.upsert(entry.clone())?;
        Ok(entry)
    }

    pub fn undeploy(
        manifest: &mut DeployManifest,
        skill_id: &str,
        agent_id: &str,
        scope: &Scope,
    ) -> Result<()> {
        if let Some(entry) = manifest.find_entry(skill_id, agent_id, scope).cloned() {
            if entry.target_path.exists() || entry.target_path.is_symlink() {
                remove_path(&entry.target_path)?;
            }
            if let Some(vault) = entry.vault_path {
                if vault.exists() {
                    remove_path(&vault)?;
                }
            }
            manifest.remove(skill_id, agent_id, scope)?;
        }
        Ok(())
    }

    pub fn toggle(
        manifest: &mut DeployManifest,
        skill_id: &str,
        agent_id: &str,
        scope: &Scope,
    ) -> Result<DeployStatus> {
        let entry = manifest
            .find_entry_mut(skill_id, agent_id, scope)
            .with_context(|| format!("Deployment entry not found for {}", skill_id))?;

        match entry.status {
            DeployStatus::Enabled => {
                // Disable it
                match entry.mode {
                    DeployMode::Symlink => {
                        // Remove symlink from target
                        if entry.target_path.exists() || entry.target_path.is_symlink() {
                            remove_path(&entry.target_path)?;
                        }
                    }
                    DeployMode::Copy => {
                        // Move to vault
                        let vault_dir = get_asm_dir().join("vault");
                        fs::create_dir_all(&vault_dir)?;
                        let vault_name = format!(
                            "{}_{}_{}",
                            agent_id,
                            skill_id.replace('/', "_"),
                            Utc::now().timestamp()
                        );
                        let vault_path = vault_dir.join(vault_name);
                        if fs::rename(&entry.target_path, &vault_path).is_err() {
                            copy_dir_recursive(&entry.target_path, &vault_path)?;
                            remove_path(&entry.target_path)?;
                        }
                        entry.vault_path = Some(vault_path);
                    }
                }
                entry.status = DeployStatus::Disabled;
            }
            DeployStatus::Disabled => {
                // Re-enable it
                match entry.mode {
                    DeployMode::Symlink => {
                        create_symlink(&entry.source_path, &entry.target_path)?;
                    }
                    DeployMode::Copy => {
                        if let Some(vault_path) = &entry.vault_path {
                            if vault_path.exists() {
                                if let Some(parent) = entry.target_path.parent() {
                                    fs::create_dir_all(parent)?;
                                }
                                if fs::rename(vault_path, &entry.target_path).is_err() {
                                    copy_dir_recursive(vault_path, &entry.target_path)?;
                                    let _ = remove_path(vault_path);
                                }
                            } else {
                                // Vault copy missing; re-copy from source
                                copy_dir_recursive(&entry.source_path, &entry.target_path)?;
                            }
                        } else {
                            copy_dir_recursive(&entry.source_path, &entry.target_path)?;
                        }
                        entry.vault_path = None;
                    }
                }
                entry.status = DeployStatus::Enabled;
            }
        }

        let new_status = entry.status.clone();
        manifest.save()?;
        Ok(new_status)
    }
}
