use anyhow::Result;

use crate::config::loader::load_or_init_config;
use crate::core::source::SourceRegistry;
use crate::deploy::manifest::DeployManifest;

pub fn handle_install_cli(subargs: &[String]) -> Result<()> {
    if subargs.is_empty() {
        println!(
            "Usage: asm install <SKILL_NAME> [--agent <AGENT>] [--mode <symlink|copy>] [--global]"
        );
        return Ok(());
    }

    let skill_query = &subargs[0];
    let config = load_or_init_config()?;
    let source_registry = SourceRegistry::load_or_init()?;
    let mut manifest = DeployManifest::load_or_init()?;
    let cache_dir = crate::config::loader::expand_path(&config.settings.sources_cache_dir);
    let current_dir = std::env::current_dir()?;

    let mut is_global = false;
    let mut selected_agent = None;
    let mut mode = config.settings.default_deployment_mode.clone();

    let mut i = 1;
    while i < subargs.len() {
        match subargs[i].as_str() {
            "--global" | "-g" => is_global = true,
            "--agent" | "-a" if i + 1 < subargs.len() => {
                selected_agent = Some(subargs[i + 1].clone());
                i += 1;
            }
            "--mode" | "-m" if i + 1 < subargs.len() => {
                if subargs[i + 1] == "copy" {
                    mode = crate::config::schema::DeployMode::Copy;
                } else {
                    mode = crate::config::schema::DeployMode::Symlink;
                }
                i += 1;
            }
            _ => {}
        }
        i += 1;
    }

    let scope = if is_global {
        crate::deploy::manifest::Scope::Global
    } else {
        crate::deploy::manifest::Scope::Project(current_dir.clone())
    };

    // Scan all sources to find matching skill
    let mut matched_skill = None;
    for source in &source_registry.sources {
        if !source.enabled {
            continue;
        }
        let effective = source_registry.get_effective_path(source, &cache_dir);
        let skills = crate::core::scanner::scan_skills_in_dir(&source.alias, &effective);
        for s in skills {
            if s.name.eq_ignore_ascii_case(skill_query) || s.id.eq_ignore_ascii_case(skill_query) {
                matched_skill = Some(s);
                break;
            }
        }
        if matched_skill.is_some() {
            break;
        }
    }

    let skill = match matched_skill {
        Some(s) => s,
        None => {
            println!(
                "Skill '{}' not found in any registered source.",
                skill_query
            );
            return Ok(());
        }
    };

    // Agent target determination with Auto-Detection fallback
    let target_agents: Vec<String> = if let Some(a) = selected_agent {
        vec![a]
    } else if !is_global {
        let detected =
            crate::core::detector::detect_agents_in_project(&current_dir, &config.agents);
        if !detected.is_empty() {
            println!(
                "Auto-detected {} active agent(s) in this project:",
                detected.len()
            );
            for d in &detected {
                println!("  • {} ({})", d.display_name, d.marker_path.display());
            }
            detected.into_iter().map(|d| d.id).collect()
        } else {
            config
                .agents
                .iter()
                .filter(|(_, a)| a.enabled)
                .map(|(k, _)| k.clone())
                .collect()
        }
    } else {
        config
            .agents
            .iter()
            .filter(|(_, a)| a.enabled)
            .map(|(k, _)| k.clone())
            .collect()
    };

    println!(
        "Deploying '{}' ({}) via {:?} to {}...",
        skill.name, skill.id, mode, scope
    );
    for agent_id in target_agents {
        if let Some(agent_cfg) = config.agents.get(&agent_id) {
            let entry = crate::deploy::controller::DeployController::deploy(
                &mut manifest,
                &skill,
                &agent_id,
                agent_cfg,
                scope.clone(),
                mode.clone(),
            )?;
            println!(
                "  -> Deployed to {}: {:?}",
                agent_cfg.display_name, entry.target_path
            );
        }
    }
    println!("Deployment complete!");
    Ok(())
}
