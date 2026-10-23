use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io;
use std::panic;
use std::time::Duration;

mod cli;
mod config;
mod core;
mod deploy;
mod tui;

use crate::config::loader::load_or_init_config;
use crate::core::source::SourceRegistry;
use crate::deploy::manifest::DeployManifest;
use crate::tui::app::App;
use crate::tui::event::{Event, EventHandler};

fn setup_panic_hook() {
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
        original_hook(panic_info);
    }));
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 && cli::run(&args[1..])? {
        return Ok(());
    }

    setup_panic_hook();

    let project_dir = if args.len() > 1 && !args[1].starts_with('-') {
        std::path::PathBuf::from(&args[1])
            .canonicalize()
            .unwrap_or_else(|_| std::path::PathBuf::from(&args[1]))
    } else {
        std::env::current_dir()?
    };

    // Load configs and state
    let config = load_or_init_config()?;
    let source_registry = SourceRegistry::load_or_init()?;
    let manifest = DeployManifest::load_or_init()?;

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // App state
    let mut app = App::new(config, source_registry, manifest, project_dir);
    let events = EventHandler::new(Duration::from_millis(250));

    // Run loop
    while !app.should_quit {
        terminal.draw(|f| tui::ui::draw(f, &mut app))?;

        match events.next()? {
            Event::Tick => app.on_tick(),
            Event::Key(key) => app.handle_key_event(key)?,
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    Ok(())
}
