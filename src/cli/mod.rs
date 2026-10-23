use anyhow::Result;

pub mod info;
pub mod install;
pub mod list;
pub mod remove;
pub mod search;
pub mod source;

pub fn print_help() {
    println!("asm - Universal AI Agent Skill Manager");
    println!();
    println!("USAGE:");
    println!("  asm [PROJECT_DIR]                    Launch interactive TUI dashboard");
    println!("  asm list [--global] [--all]          List installed skills in project or globally");
    println!("  asm search <QUERY>                   Search available skills by name, tag, or description");
    println!(
        "  asm info <SKILL_NAME>                Show detailed metadata and preview for a skill"
    );
    println!(
        "  asm install <SKILL_NAME> [--global]  Install a skill into project or global agents"
    );
    println!("  asm remove <SKILL_NAME> [--global]   Remove/delete a skill from project or global agents");
    println!(
        "  asm source add <ALIAS> <URL> [--project] Add a new skill source (Global by default)"
    );
    println!("  asm source list                      List all registered skill sources");
    println!("  asm source remove <ALIAS>            Remove a registered skill source");
    println!("  asm source sync                      Sync all remote Git skill sources");
    println!();
    println!("FLAGS:");
    println!("  -h, --help                           Print help information");
    println!("  -v, --version                        Print version information");
    println!("  -g, --global                         Target global agent directories (~/.agents/skills, etc.)");
    println!();
    println!("ARGUMENTS:");
    println!("  [PROJECT_DIR]                        Path to project workspace (default: current directory)");
}

/// Dispatches CLI subcommands. Returns Ok(true) if a CLI command was handled,
/// or Ok(false) if execution should proceed to launch the TUI.
pub fn run(args: &[String]) -> Result<bool> {
    if args.is_empty() {
        return Ok(false);
    }

    match args[0].as_str() {
        "-v" | "--version" => {
            println!("asm {}", env!("CARGO_PKG_VERSION"));
            Ok(true)
        }
        "-h" | "--help" => {
            print_help();
            Ok(true)
        }
        "source" => {
            source::handle_source_cli(&args[1..])?;
            Ok(true)
        }
        "install" => {
            install::handle_install_cli(&args[1..])?;
            Ok(true)
        }
        "remove" | "rm" => {
            remove::handle_remove_cli(&args[1..])?;
            Ok(true)
        }
        "list" | "ls" => {
            list::handle_list_cli(&args[1..])?;
            Ok(true)
        }
        "search" | "find" => {
            search::handle_search_cli(&args[1..])?;
            Ok(true)
        }
        "info" | "show" => {
            info::handle_info_cli(&args[1..])?;
            Ok(true)
        }
        "sync" => {
            source::handle_sync_cli()?;
            Ok(true)
        }
        _ if args[0].starts_with('-') => {
            print_help();
            Ok(true)
        }
        _ => {
            // Positional argument (e.g. project path), launch TUI
            Ok(false)
        }
    }
}
