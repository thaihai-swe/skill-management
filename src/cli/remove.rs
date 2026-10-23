use anyhow::Result;

use crate::config::loader::load_or_init_config;
use crate::deploy::manifest::DeployManifest;

pub fn handle_remove_cli(subargs: &[String]) -> Result<()> {
    let positional: Vec<&String> = subargs.iter().filter(|a| !a.starts_with('-')).collect();
    if positional.is_empty() {
        println!("Usage: asm remove <SKILL_NAME> [--global]");
        return Ok(());
    }

    let skill_query = positional[0];
    let is_global = subargs.iter().any(|a| a == "--global" || a == "-g");
    let config = load_or_init_config()?;
    let mut manifest = DeployManifest::load_or_init()?;
    let current_dir = std::env::current_dir()?;

    let mut removed = 0;

    if is_global {
        let to_remove: Vec<(String, String, crate::deploy::manifest::Scope)> = manifest
            .deployments
            .iter()
            .filter(|d| {
                let in_global = matches!(&d.scope, crate::deploy::manifest::Scope::Global);
                let matches_name = d.skill_id.contains(skill_query)
                    || d.target_path.to_string_lossy().contains(skill_query);
                in_global && matches_name
            })
            .map(|d| (d.skill_id.clone(), d.agent_id.clone(), d.scope.clone()))
            .collect();

        for (s_id, a_id, sc) in to_remove {
            crate::deploy::controller::DeployController::undeploy(
                &mut manifest,
                &s_id,
                &a_id,
                &sc,
            )?;
            removed += 1;
        }

        for agent_cfg in config.agents.values() {
            let p = crate::config::loader::expand_path(&agent_cfg.global_dir);
            let standalone =
                crate::core::scanner::scan_installed_agent_skills(agent_cfg, &p, "global");
            for s in standalone {
                if s.name.eq_ignore_ascii_case(skill_query) || s.id.contains(skill_query) {
                    let _ = crate::deploy::fs_ops::remove_path(&s.path);
                    removed += 1;
                }
            }
        }

        if removed > 0 {
            println!(
                "Successfully removed '{}' from {} global agent location(s).",
                skill_query, removed
            );
        } else {
            println!(
                "No deployed skill matching '{}' found in global agent directories.",
                skill_query
            );
        }
    } else {
        let to_remove: Vec<(String, String, crate::deploy::manifest::Scope)> = manifest
            .deployments
            .iter()
            .filter(|d| {
                let in_project = matches!(&d.scope, crate::deploy::manifest::Scope::Project(p) if p == &current_dir);
                let matches_name = d.skill_id.contains(skill_query) || d.target_path.to_string_lossy().contains(skill_query);
                in_project && matches_name
            })
            .map(|d| (d.skill_id.clone(), d.agent_id.clone(), d.scope.clone()))
            .collect();

        for (s_id, a_id, sc) in to_remove {
            crate::deploy::controller::DeployController::undeploy(
                &mut manifest,
                &s_id,
                &a_id,
                &sc,
            )?;
            removed += 1;
        }

        for agent_cfg in config.agents.values() {
            let p = current_dir.join(&agent_cfg.project_dir);
            let standalone =
                crate::core::scanner::scan_installed_agent_skills(agent_cfg, &p, "project");
            for s in standalone {
                if s.name.eq_ignore_ascii_case(skill_query) || s.id.contains(skill_query) {
                    let _ = crate::deploy::fs_ops::remove_path(&s.path);
                    removed += 1;
                }
            }
        }

        if removed > 0 {
            println!(
                "Successfully removed '{}' from {} project agent location(s).",
                skill_query, removed
            );
        } else {
            println!(
                "No deployed skill matching '{}' found in this project.",
                skill_query
            );
        }
    }

    Ok(())
}
