use crate::config::loader::load_or_init_config;
use crate::core::source::SourceRegistry;
use anyhow::Result;

pub fn handle_sync_cli() -> Result<()> {
    let source_registry = SourceRegistry::load_or_init()?;
    let config = load_or_init_config()?;
    let cache_dir = crate::config::loader::expand_path(&config.settings.sources_cache_dir);

    println!("Synchronizing remote skill sources...");
    let mut count = 0;
    for s in &source_registry.sources {
        if s.source_type == crate::core::source::SourceType::RemoteGit && s.enabled {
            print!("Syncing {}... ", s.alias);
            match source_registry.sync_remote_source(s, &cache_dir) {
                Ok(_) => println!("OK"),
                Err(e) => println!("Error: {}", e),
            }
            count += 1;
        }
    }
    println!("Synced {} source(s).", count);
    Ok(())
}

pub fn handle_source_cli(subargs: &[String]) -> Result<()> {
    let mut source_registry = SourceRegistry::load_or_init()?;
    let config = load_or_init_config()?;
    let cache_dir = crate::config::loader::expand_path(&config.settings.sources_cache_dir);

    if subargs.is_empty() || subargs[0] == "list" {
        println!(
            "Registered Skill Sources ({}):",
            source_registry.sources.len()
        );
        println!(
            "{:<28} {:<10} {:<12} {:<10} LOCATION",
            "ALIAS", "SCOPE", "TYPE", "STATUS"
        );
        println!("{}", "-".repeat(85));
        for s in &source_registry.sources {
            let type_str = match s.source_type {
                crate::core::source::SourceType::LocalFolder => "LocalFolder",
                crate::core::source::SourceType::LocalGit => "LocalGit",
                crate::core::source::SourceType::RemoteGit => "RemoteGit",
            };
            let scope_str = match s.scope {
                crate::core::source::SourceScope::Global => "Global",
                crate::core::source::SourceScope::Project => "Project",
            };
            let status_str = if s.enabled { "Enabled" } else { "Disabled" };
            println!(
                "{:<28} {:<10} {:<12} {:<10} {}",
                s.alias, scope_str, type_str, status_str, s.location
            );
        }
        return Ok(());
    }

    match subargs[0].as_str() {
        "add" => {
            let positional: Vec<&String> = subargs
                .iter()
                .skip(1)
                .filter(|a| !a.starts_with('-'))
                .collect();
            if positional.len() < 2 {
                println!("Usage: asm source add <ALIAS> <URL_OR_PATH> [--project]");
                println!("Default source scope is Global (~/.config/asm/sources.json)");
                return Ok(());
            }
            let alias = positional[0];
            let location = positional[1];
            let is_project = subargs.iter().any(|a| a == "--project");
            let scope = if is_project {
                crate::core::source::SourceScope::Project
            } else {
                crate::core::source::SourceScope::Global
            };

            let source_type = if location.starts_with("http://")
                || location.starts_with("https://")
                || location.starts_with("git@")
            {
                crate::core::source::SourceType::RemoteGit
            } else {
                let p = std::path::PathBuf::from(location);
                if p.join(".git").exists() {
                    crate::core::source::SourceType::LocalGit
                } else {
                    crate::core::source::SourceType::LocalFolder
                }
            };

            source_registry.add(
                alias.to_string(),
                location.to_string(),
                source_type.clone(),
                scope.clone(),
            )?;
            println!("Registered source '{}' (Scope: {:?}).", alias, scope);

            if source_type == crate::core::source::SourceType::RemoteGit {
                println!("Cloning / syncing remote repository...");
                let src = source_registry
                    .sources
                    .iter()
                    .find(|s| s.alias == *alias)
                    .unwrap();
                source_registry.sync_remote_source(src, &cache_dir)?;
            }

            let effective = source_registry.get_effective_path(
                source_registry
                    .sources
                    .iter()
                    .find(|s| s.alias == *alias)
                    .unwrap(),
                &cache_dir,
            );
            let skills = crate::core::scanner::scan_skills_in_dir(alias, &effective);
            println!(
                "Successfully discovered {} skill(s) in source '{}':",
                skills.len(),
                alias
            );
            for skill in &skills {
                println!("  - {:<28} {}", skill.name, skill.description);
            }
        }
        "remove" | "rm" => {
            if subargs.len() < 2 {
                println!("Usage: asm source remove <ALIAS>");
                return Ok(());
            }
            source_registry.remove(&subargs[1])?;
            println!("Removed source '{}'.", subargs[1]);
        }
        "sync" => {
            handle_sync_cli()?;
        }
        _ => {
            println!(
                "Unknown source command '{}'. Try 'add', 'list', 'remove', or 'sync'.",
                subargs[0]
            );
        }
    }
    Ok(())
}
