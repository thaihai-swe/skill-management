use anyhow::Result;

use crate::config::loader::load_or_init_config;
use crate::core::source::SourceRegistry;

pub fn handle_search_cli(subargs: &[String]) -> Result<()> {
    if subargs.is_empty() {
        println!("Usage: asm search <QUERY>");
        return Ok(());
    }

    let query = &subargs[0].to_lowercase();
    let config = load_or_init_config()?;
    let source_registry = SourceRegistry::load_or_init()?;
    let cache_dir = crate::config::loader::expand_path(&config.settings.sources_cache_dir);

    let mut matches = Vec::new();
    for source in &source_registry.sources {
        if !source.enabled {
            continue;
        }
        let effective = source_registry.get_effective_path(source, &cache_dir);
        let skills = crate::core::scanner::scan_skills_in_dir(&source.alias, &effective);
        for s in skills {
            let name_match = s.name.to_lowercase().contains(query);
            let desc_match = s.description.to_lowercase().contains(query);
            let tag_match = s.tags.iter().any(|t| t.to_lowercase().contains(query));
            if name_match || desc_match || tag_match {
                matches.push(s);
            }
        }
    }

    if matches.is_empty() {
        println!(
            "No skills matching '{}' found in registered sources.",
            query
        );
        return Ok(());
    }

    println!("Found {} skill(s) matching \"{}\":", matches.len(), query);
    println!(
        "{:<28} {:<18} {:<15} DESCRIPTION",
        "NAME", "SOURCE", "VERSION"
    );
    println!("{}", "-".repeat(90));
    for s in &matches {
        let ver = s.version.as_deref().unwrap_or("--");
        let desc = if s.description.len() > 40 {
            format!("{}...", &s.description[..37])
        } else {
            s.description.clone()
        };
        println!("{:<28} {:<18} {:<15} {}", s.name, s.source_alias, ver, desc);
    }
    println!();
    println!("Install any skill with: asm install <SKILL_NAME> [--global]");
    Ok(())
}
