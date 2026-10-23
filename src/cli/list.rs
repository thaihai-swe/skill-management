use anyhow::Result;

use crate::config::loader::load_or_init_config;
use crate::deploy::manifest::DeployManifest;

pub fn handle_list_cli(subargs: &[String]) -> Result<()> {
    let config = load_or_init_config()?;
    let manifest = DeployManifest::load_or_init()?;
    let current_dir = std::env::current_dir()?;

    let show_global = subargs.iter().any(|a| a == "--global" || a == "-g");
    let show_all = subargs.iter().any(|a| a == "--all");
    let show_project = !show_global || show_all;
    let show_global_effective = show_global || show_all;

    if show_project {
        println!(
            "Skills Installed in Current Project ({}):",
            current_dir.display()
        );
        println!(
            "{:<28} {:<15} {:<10} {:<10} PATH",
            "SKILL", "AGENT", "STATUS", "MODE"
        );
        println!("{}", "-".repeat(85));

        let mut found = 0;
        for dep in &manifest.deployments {
            if matches!(&dep.scope, crate::deploy::manifest::Scope::Project(p) if p == &current_dir)
            {
                let status = match dep.status {
                    crate::deploy::manifest::DeployStatus::Enabled => "Enabled",
                    crate::deploy::manifest::DeployStatus::Disabled => "Disabled",
                };
                let mode = match dep.mode {
                    crate::config::schema::DeployMode::Symlink => "Symlink",
                    crate::config::schema::DeployMode::Copy => "Copy",
                };
                println!(
                    "{:<28} {:<15} {:<10} {:<10} {:?}",
                    dep.skill_id, dep.agent_id, status, mode, dep.target_path
                );
                found += 1;
            }
        }

        for agent_cfg in config.agents.values() {
            if !agent_cfg.enabled {
                continue;
            }
            let p = current_dir.join(&agent_cfg.project_dir);
            let standalone =
                crate::core::scanner::scan_installed_agent_skills(agent_cfg, &p, "project");
            for s in standalone {
                let already_shown = manifest
                    .deployments
                    .iter()
                    .any(|d| d.skill_id == s.id || d.target_path == s.path);
                if !already_shown {
                    println!(
                        "{:<28} {:<15} {:<10} {:<10} {:?}",
                        s.name, agent_cfg.display_name, "Unmanaged", "--", s.path
                    );
                    found += 1;
                }
            }
        }

        if found == 0 {
            println!("No skills currently installed in this project.");
        }
        println!();
    }

    if show_global_effective {
        println!("Skills Installed Globally (~/.agents/skills, ~/.claude/skills, etc.):");
        println!(
            "{:<28} {:<15} {:<10} {:<10} PATH",
            "SKILL", "AGENT", "STATUS", "MODE"
        );
        println!("{}", "-".repeat(85));

        let mut found = 0;
        for dep in &manifest.deployments {
            if matches!(&dep.scope, crate::deploy::manifest::Scope::Global) {
                let status = match dep.status {
                    crate::deploy::manifest::DeployStatus::Enabled => "Enabled",
                    crate::deploy::manifest::DeployStatus::Disabled => "Disabled",
                };
                let mode = match dep.mode {
                    crate::config::schema::DeployMode::Symlink => "Symlink",
                    crate::config::schema::DeployMode::Copy => "Copy",
                };
                println!(
                    "{:<28} {:<15} {:<10} {:<10} {:?}",
                    dep.skill_id, dep.agent_id, status, mode, dep.target_path
                );
                found += 1;
            }
        }

        for agent_cfg in config.agents.values() {
            if !agent_cfg.enabled {
                continue;
            }
            let p = crate::config::loader::expand_path(&agent_cfg.global_dir);
            let standalone =
                crate::core::scanner::scan_installed_agent_skills(agent_cfg, &p, "global");
            for s in standalone {
                let already_shown = manifest
                    .deployments
                    .iter()
                    .any(|d| d.skill_id == s.id || d.target_path == s.path);
                if !already_shown {
                    println!(
                        "{:<28} {:<15} {:<10} {:<10} {:?}",
                        s.name, agent_cfg.display_name, "Unmanaged", "--", s.path
                    );
                    found += 1;
                }
            }
        }

        if found == 0 {
            println!("No skills currently installed globally.");
        }
        println!();
    }

    Ok(())
}
