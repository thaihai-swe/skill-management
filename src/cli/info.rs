use anyhow::Result;

use crate::config::loader::load_or_init_config;
use crate::core::source::SourceRegistry;

pub fn handle_info_cli(subargs: &[String]) -> Result<()> {
    if subargs.is_empty() {
        println!("Usage: asm info <SKILL_NAME>");
        return Ok(());
    }

    let query = &subargs[0];
    let config = load_or_init_config()?;
    let source_registry = SourceRegistry::load_or_init()?;
    let cache_dir = crate::config::loader::expand_path(&config.settings.sources_cache_dir);
    let current_dir = std::env::current_dir()?;

    // Search upstream sources first
    let mut target_skill = None;
    for source in &source_registry.sources {
        if !source.enabled {
            continue;
        }
        let effective = source_registry.get_effective_path(source, &cache_dir);
        let skills = crate::core::scanner::scan_skills_in_dir(&source.alias, &effective);
        for s in skills {
            if s.name.eq_ignore_ascii_case(query) || s.id.eq_ignore_ascii_case(query) {
                target_skill = Some(s);
                break;
            }
        }
        if target_skill.is_some() {
            break;
        }
    }

    // If not found in sources, search project / global installed
    if target_skill.is_none() {
        for agent_cfg in config.agents.values() {
            if !agent_cfg.enabled {
                continue;
            }
            let p_dir = current_dir.join(&agent_cfg.project_dir);
            for s in crate::core::scanner::scan_installed_agent_skills(agent_cfg, &p_dir, "project")
            {
                if s.name.eq_ignore_ascii_case(query) || s.id.contains(query) {
                    target_skill = Some(s);
                    break;
                }
            }
            if target_skill.is_some() {
                break;
            }
            let g_dir = crate::config::loader::expand_path(&agent_cfg.global_dir);
            for s in crate::core::scanner::scan_installed_agent_skills(agent_cfg, &g_dir, "global")
            {
                if s.name.eq_ignore_ascii_case(query) || s.id.contains(query) {
                    target_skill = Some(s);
                    break;
                }
            }
            if target_skill.is_some() {
                break;
            }
        }
    }

    let skill = match target_skill {
        Some(s) => s,
        None => {
            println!(
                "Skill '{}' not found in upstream sources or installed agents.",
                query
            );
            return Ok(());
        }
    };

    println!("Skill Information: {}", skill.name);
    println!("{}", "=".repeat(60));
    println!("  Name:        {}", skill.name);
    println!("  ID:          {}", skill.id);
    println!("  Source:      {}", skill.source_alias);
    println!("  Location:    {}", skill.path.display());
    println!(
        "  Version:     {}",
        skill.version.as_deref().unwrap_or("N/A")
    );
    println!(
        "  Author:      {}",
        skill.author.as_deref().unwrap_or("N/A")
    );
    println!(
        "  Tags:        {}",
        if skill.tags.is_empty() {
            "None".to_string()
        } else {
            skill.tags.join(", ")
        }
    );
    println!("  Description: {}", skill.description);
    println!();
    println!("Preview (SKILL.md):");
    println!("{}", "-".repeat(60));
    if skill.raw_content.is_empty() {
        println!("  (No raw content available)");
    } else {
        let lines: Vec<&str> = skill.raw_content.lines().collect();
        let preview_lines = lines.iter().take(35);
        for line in preview_lines {
            println!("  {}", line);
        }
        if lines.len() > 35 {
            println!(
                "  ... [{} more lines - use asm TUI to read full preview]",
                lines.len() - 35
            );
        }
    }
    println!();
    Ok(())
}
