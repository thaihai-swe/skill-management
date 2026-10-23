use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::collections::HashSet;
use std::path::PathBuf;

use crate::config::loader::expand_path;
use crate::config::schema::{AppConfig, DeployMode};
use crate::core::catalog::Catalog;
use crate::core::scanner::{scan_installed_agent_skills, scan_skills_in_dir};
use crate::core::skill::Skill;
use crate::core::source::{SourceRegistry, SourceScope, SourceType};
use crate::deploy::controller::DeployController;
use crate::deploy::fs_ops::remove_path;
use crate::deploy::manifest::{DeployManifest, Scope};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ActivePane {
    Sources,
    Skills,
    Preview,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScopeChoice {
    Global,
    CurrentProject,
}

#[derive(Debug, Clone)]
pub struct DeployAgentItem {
    pub id: String,
    pub display_name: String,
    pub selected: bool,
    pub detected: bool,
}

#[derive(Debug, Clone)]
pub struct DeployModalState {
    pub mode: DeployMode,
    pub scope_choice: ScopeChoice,
    pub agents: Vec<DeployAgentItem>,
    pub focused_field: usize, // 0: mode, 1: scope, 2: agents, 3: submit
    pub selected_agent_idx: usize,
}

#[derive(Debug, Clone)]
pub struct AddSourceModalState {
    pub alias: String,
    pub location: String,
    pub scope: SourceScope,
    pub focused_field: usize, // 0: alias, 1: location, 2: scope, 3: submit
}

impl Default for AddSourceModalState {
    fn default() -> Self {
        Self {
            alias: String::new(),
            location: String::new(),
            scope: SourceScope::Global,
            focused_field: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ConfirmAction {
    UndeployProject(Vec<Skill>),
    UndeployGlobal(Vec<Skill>),
    UndeployAll(Vec<Skill>),
    RemoveSource(String),
}

#[derive(Debug, Clone)]
pub struct ConfirmModalState {
    pub title: String,
    pub message: String,
    pub action: ConfirmAction,
}

#[derive(Debug, Clone)]
pub enum ActiveModal {
    None,
    Deploy(DeployModalState),
    Confirm(ConfirmModalState),
    Help,
    AddSource(AddSourceModalState),
}

pub struct App {
    pub config: AppConfig,
    pub source_registry: SourceRegistry,
    pub manifest: DeployManifest,
    pub catalog: Catalog,
    pub current_project_dir: PathBuf,

    pub project_skills: Vec<Skill>,
    pub global_skills: Vec<Skill>,

    pub active_pane: ActivePane,
    pub active_modal: ActiveModal,

    pub sources_list: Vec<String>, // "installed:project", "installed:global", "all", followed by alias names
    pub selected_source_idx: usize,

    pub selected_skill_idx: usize,
    pub selected_skill_ids: HashSet<String>,

    pub preview_scroll: usize,

    pub search_query: String,
    pub is_searching: bool,

    pub status_message: Option<(String, usize)>,
    pub should_quit: bool,
}

impl App {
    pub fn new(
        config: AppConfig,
        source_registry: SourceRegistry,
        manifest: DeployManifest,
        current_project_dir: PathBuf,
    ) -> Self {
        let mut app = Self {
            config,
            source_registry,
            manifest,
            catalog: Catalog::default(),
            current_project_dir,

            project_skills: Vec::new(),
            global_skills: Vec::new(),

            active_pane: ActivePane::Skills,
            active_modal: ActiveModal::None,

            sources_list: vec![
                "installed:project".to_string(),
                "installed:global".to_string(),
                "all".to_string(),
            ],
            selected_source_idx: 0,

            selected_skill_idx: 0,
            selected_skill_ids: HashSet::new(),

            preview_scroll: 0,

            search_query: String::new(),
            is_searching: false,

            status_message: None,
            should_quit: false,
        };

        app.reload_sources();
        app.reload_catalog();
        app
    }

    pub fn reload_sources(&mut self) {
        let mut list = vec![
            "installed:project".to_string(),
            "installed:global".to_string(),
            "all".to_string(),
        ];
        for source in &self.source_registry.sources {
            if source.enabled {
                list.push(format!("source:{}", source.alias));
            }
        }
        self.sources_list = list;
        if self.selected_source_idx >= self.sources_list.len() {
            self.selected_source_idx = 0;
        }
    }

    pub fn reload_catalog(&mut self) {
        let cache_dir = expand_path(&self.config.settings.sources_cache_dir);
        let mut all_skills = Vec::new();

        // Scan all active sources
        for source in &self.source_registry.sources {
            if !source.enabled {
                continue;
            }
            let effective_path = self.source_registry.get_effective_path(source, &cache_dir);
            let skills = scan_skills_in_dir(&source.alias, &effective_path);
            for s in skills {
                all_skills.push(s);
            }
        }

        self.catalog = Catalog::new(all_skills);
        self.project_skills = self.scan_project_skills();
        self.global_skills = self.scan_global_skills();

        if self.selected_skill_idx >= self.filtered_skills().len() && self.selected_skill_idx > 0 {
            self.selected_skill_idx = self.filtered_skills().len().saturating_sub(1);
        }
        self.preview_scroll = 0;
    }

    fn scan_project_skills(&self) -> Vec<Skill> {
        let mut skills = Vec::new();
        let mut seen_names = HashSet::new();

        // 1. Check deployments manifest for Project scope
        for dep in &self.manifest.deployments {
            if matches!(&dep.scope, Scope::Project(p) if p == &self.current_project_dir)
                && (dep.target_path.exists() || dep.target_path.is_symlink())
            {
                let name = dep
                    .target_path
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| dep.skill_id.clone());
                if !seen_names.contains(&name) {
                    seen_names.insert(name.clone());
                    let desc = format!("Active deployment for {}", dep.agent_id);
                    let mut skill = Skill {
                        id: dep.skill_id.clone(),
                        name: name.clone(),
                        source_alias: dep.source_alias.clone(),
                        path: dep.target_path.clone(),
                        description: desc,
                        version: None,
                        tags: vec!["project-deployed".to_string()],
                        author: None,
                        raw_content: String::new(),
                    };
                    Self::load_content_if_empty(&mut skill);
                    skills.push(skill);
                }
            }
        }

        // 2. Scan physical project agent directories for unmanaged skills
        for agent_cfg in self.config.agents.values() {
            if !agent_cfg.enabled {
                continue;
            }
            let proj_dir = self.current_project_dir.join(&agent_cfg.project_dir);
            let standalone = scan_installed_agent_skills(agent_cfg, &proj_dir, "project");
            for mut s in standalone {
                if !seen_names.contains(&s.name) {
                    seen_names.insert(s.name.clone());
                    Self::load_content_if_empty(&mut s);
                    skills.push(s);
                }
            }
        }

        skills
    }

    fn scan_global_skills(&self) -> Vec<Skill> {
        let mut skills = Vec::new();
        let mut seen_names = HashSet::new();

        // 1. Check deployments manifest for Global scope
        for dep in &self.manifest.deployments {
            if matches!(&dep.scope, Scope::Global)
                && (dep.target_path.exists() || dep.target_path.is_symlink())
            {
                let name = dep
                    .target_path
                    .file_name()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| dep.skill_id.clone());
                if !seen_names.contains(&name) {
                    seen_names.insert(name.clone());
                    let desc = format!("Active global deployment for {}", dep.agent_id);
                    let mut skill = Skill {
                        id: dep.skill_id.clone(),
                        name: name.clone(),
                        source_alias: dep.source_alias.clone(),
                        path: dep.target_path.clone(),
                        description: desc,
                        version: None,
                        tags: vec!["global-deployed".to_string()],
                        author: None,
                        raw_content: String::new(),
                    };
                    Self::load_content_if_empty(&mut skill);
                    skills.push(skill);
                }
            }
        }

        // 2. Scan physical global agent directories for unmanaged skills
        for agent_cfg in self.config.agents.values() {
            if !agent_cfg.enabled {
                continue;
            }
            let glob_dir = expand_path(&agent_cfg.global_dir);
            let standalone = scan_installed_agent_skills(agent_cfg, &glob_dir, "global");
            for mut s in standalone {
                if !seen_names.contains(&s.name) {
                    seen_names.insert(s.name.clone());
                    Self::load_content_if_empty(&mut s);
                    skills.push(s);
                }
            }
        }

        skills
    }

    fn load_content_if_empty(skill: &mut Skill) {
        if skill.raw_content.is_empty() {
            let skill_md = if skill.path.is_dir() {
                skill.path.join("SKILL.md")
            } else {
                skill.path.clone()
            };
            if let Ok(content) = std::fs::read_to_string(&skill_md) {
                skill.raw_content = content;
            }
        }
    }

    pub fn get_project_skills(&self) -> &[Skill] {
        &self.project_skills
    }

    pub fn get_global_skills(&self) -> &[Skill] {
        &self.global_skills
    }

    pub fn current_source_filter(&self) -> Option<&str> {
        self.sources_list
            .get(self.selected_source_idx)
            .map(|s| s.as_str())
    }

    pub fn filtered_skills(&self) -> Vec<Skill> {
        let filter = self.current_source_filter();
        match filter {
            Some("installed:project") => {
                Catalog::new(self.project_skills.clone()).filter(None, &self.search_query)
            }
            Some("installed:global") => {
                Catalog::new(self.global_skills.clone()).filter(None, &self.search_query)
            }
            Some("all") | None => self.catalog.filter(None, &self.search_query),
            Some(src) if src.starts_with("source:") => {
                let alias = &src["source:".len()..];
                self.catalog.filter(Some(alias), &self.search_query)
            }
            Some(other) => self.catalog.filter(Some(other), &self.search_query),
        }
    }

    pub fn current_selected_skill(&self) -> Option<Skill> {
        let skills = self.filtered_skills();
        skills.get(self.selected_skill_idx).cloned()
    }

    pub fn set_status(&mut self, msg: impl Into<String>) {
        self.status_message = Some((msg.into(), 20)); // ~5 seconds at 250ms tick rate
    }

    pub fn on_tick(&mut self) {
        if let Some((_, ref mut ticks)) = self.status_message {
            if *ticks > 0 {
                *ticks -= 1;
            } else {
                self.status_message = None;
            }
        }
    }

    pub fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        if let ActiveModal::Deploy(_) = self.active_modal {
            self.handle_deploy_modal_key(key)?;
            return Ok(());
        }

        if let ActiveModal::AddSource(_) = self.active_modal {
            self.handle_add_source_modal_key(key)?;
            return Ok(());
        }

        if let ActiveModal::Confirm(ref modal) = self.active_modal {
            let action = modal.action.clone();
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
                    self.execute_confirm_action(action)?;
                    self.active_modal = ActiveModal::None;
                }
                KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                    self.active_modal = ActiveModal::None;
                }
                _ => {}
            }
            return Ok(());
        }

        if let ActiveModal::Help = self.active_modal {
            if matches!(
                key.code,
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?')
            ) {
                self.active_modal = ActiveModal::None;
            }
            return Ok(());
        }

        if self.is_searching {
            match key.code {
                KeyCode::Esc => {
                    self.is_searching = false;
                    self.search_query.clear();
                    self.selected_skill_idx = 0;
                }
                KeyCode::Enter => {
                    self.is_searching = false;
                    self.active_pane = ActivePane::Skills;
                }
                KeyCode::Backspace => {
                    self.search_query.pop();
                    self.selected_skill_idx = 0;
                }
                KeyCode::Char(c) => {
                    self.search_query.push(c);
                    self.selected_skill_idx = 0;
                }
                _ => {}
            }
            return Ok(());
        }

        // Global shortcuts
        match key.code {
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
                return Ok(());
            }
            KeyCode::Char('q') => {
                self.should_quit = true;
                return Ok(());
            }
            KeyCode::Char('+') | KeyCode::Char('n') | KeyCode::F(2) => {
                self.active_modal = ActiveModal::AddSource(AddSourceModalState::default());
                return Ok(());
            }
            KeyCode::Char('?') => {
                self.active_modal = ActiveModal::Help;
                return Ok(());
            }
            KeyCode::Char('/') => {
                self.is_searching = true;
                self.search_query.clear();
                return Ok(());
            }
            KeyCode::Tab => {
                self.active_pane = match self.active_pane {
                    ActivePane::Sources => ActivePane::Skills,
                    ActivePane::Skills => ActivePane::Preview,
                    ActivePane::Preview => ActivePane::Sources,
                };
                return Ok(());
            }
            KeyCode::BackTab => {
                self.active_pane = match self.active_pane {
                    ActivePane::Sources => ActivePane::Preview,
                    ActivePane::Skills => ActivePane::Sources,
                    ActivePane::Preview => ActivePane::Skills,
                };
                return Ok(());
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.active_pane = match self.active_pane {
                    ActivePane::Preview => ActivePane::Skills,
                    ActivePane::Skills => ActivePane::Sources,
                    ActivePane::Sources => ActivePane::Sources,
                };
                return Ok(());
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.active_pane = match self.active_pane {
                    ActivePane::Sources => ActivePane::Skills,
                    ActivePane::Skills => ActivePane::Preview,
                    ActivePane::Preview => ActivePane::Preview,
                };
                return Ok(());
            }
            _ => {}
        }

        // Pane-specific navigation
        match self.active_pane {
            ActivePane::Sources => self.handle_sources_pane_key(key)?,
            ActivePane::Skills => self.handle_skills_pane_key(key)?,
            ActivePane::Preview => self.handle_preview_pane_key(key)?,
        }

        Ok(())
    }

    fn handle_sources_pane_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Char('+') | KeyCode::Char('n') => {
                self.active_modal = ActiveModal::AddSource(AddSourceModalState::default());
            }
            KeyCode::Char('r') | KeyCode::Delete => {
                if let Some(src_key) = self.sources_list.get(self.selected_source_idx).cloned() {
                    if let Some(alias) = src_key.strip_prefix("source:") {
                        let alias = alias.to_string();
                        self.active_modal = ActiveModal::Confirm(ConfirmModalState {
                            title: "Remove Skill Source".to_string(),
                            message: format!(
                                "Remove source registry entry '{}' from ASM? (y/n)",
                                alias
                            ),
                            action: ConfirmAction::RemoveSource(alias),
                        });
                    } else {
                        self.set_status("System view folders cannot be removed");
                    }
                }
            }
            KeyCode::Char(' ') | KeyCode::Char('d') => {
                if let Some(src_key) = self.sources_list.get(self.selected_source_idx).cloned() {
                    if let Some(alias) = src_key.strip_prefix("source:") {
                        let status = self.source_registry.toggle(alias)?;
                        self.reload_sources();
                        self.reload_catalog();
                        self.set_status(format!(
                            "Source '{}' is now {}",
                            alias,
                            if status { "Enabled" } else { "Disabled" }
                        ));
                    }
                }
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_source_idx > 0 {
                    self.selected_source_idx -= 1;
                    self.selected_skill_idx = 0;
                    self.preview_scroll = 0;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected_source_idx + 1 < self.sources_list.len() {
                    self.selected_source_idx += 1;
                    self.selected_skill_idx = 0;
                    self.preview_scroll = 0;
                }
            }
            KeyCode::Enter => {
                self.active_pane = ActivePane::Skills;
            }
            KeyCode::Char('s') => {
                self.sync_remote_sources()?;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_add_source_modal_key(&mut self, key: KeyEvent) -> Result<()> {
        let (submit, cancel) = if let ActiveModal::AddSource(ref mut modal) = self.active_modal {
            match key.code {
                KeyCode::Esc => (false, true),
                KeyCode::Tab | KeyCode::Down => {
                    modal.focused_field = (modal.focused_field + 1) % 4;
                    (false, false)
                }
                KeyCode::BackTab | KeyCode::Up => {
                    modal.focused_field = if modal.focused_field == 0 {
                        3
                    } else {
                        modal.focused_field - 1
                    };
                    (false, false)
                }
                KeyCode::Left | KeyCode::Right => {
                    if modal.focused_field == 2 {
                        modal.scope = match modal.scope {
                            SourceScope::Global => SourceScope::Project,
                            SourceScope::Project => SourceScope::Global,
                        };
                    }
                    (false, false)
                }
                KeyCode::Backspace => {
                    if modal.focused_field == 0 {
                        modal.alias.pop();
                    } else if modal.focused_field == 1 {
                        modal.location.pop();
                    }
                    (false, false)
                }
                KeyCode::Char(' ') => {
                    if modal.focused_field == 2 {
                        modal.scope = match modal.scope {
                            SourceScope::Global => SourceScope::Project,
                            SourceScope::Project => SourceScope::Global,
                        };
                    } else if modal.focused_field == 0 {
                        modal.alias.push(' ');
                    } else if modal.focused_field == 1 {
                        modal.location.push(' ');
                    }
                    (false, false)
                }
                KeyCode::Char(c) => {
                    if modal.focused_field == 0 {
                        modal.alias.push(c);
                    } else if modal.focused_field == 1 {
                        modal.location.push(c);
                    }
                    (false, false)
                }
                KeyCode::Enter => {
                    if modal.focused_field == 3
                        || (!modal.alias.is_empty() && !modal.location.is_empty())
                    {
                        (true, false)
                    } else {
                        modal.focused_field = (modal.focused_field + 1) % 4;
                        (false, false)
                    }
                }
                _ => (false, false),
            }
        } else {
            (false, false)
        };

        if cancel {
            self.active_modal = ActiveModal::None;
        } else if submit {
            self.execute_add_source()?;
            self.active_modal = ActiveModal::None;
        }
        Ok(())
    }

    fn execute_add_source(&mut self) -> Result<()> {
        let (alias, location, scope) = if let ActiveModal::AddSource(ref modal) = self.active_modal
        {
            (
                modal.alias.trim().to_string(),
                modal.location.trim().to_string(),
                modal.scope.clone(),
            )
        } else {
            return Ok(());
        };

        if alias.is_empty() || location.is_empty() {
            self.set_status("Alias and location cannot be empty");
            return Ok(());
        }

        let source_type = if location.starts_with("http://")
            || location.starts_with("https://")
            || location.starts_with("git@")
        {
            SourceType::RemoteGit
        } else {
            let path = PathBuf::from(&location);
            if path.join(".git").exists() {
                SourceType::LocalGit
            } else {
                SourceType::LocalFolder
            }
        };

        self.source_registry
            .add(alias.clone(), location, source_type.clone(), scope)?;

        let cache_dir = expand_path(&self.config.settings.sources_cache_dir);
        if source_type == SourceType::RemoteGit {
            if let Some(src) = self
                .source_registry
                .sources
                .iter()
                .find(|s| s.alias == alias)
            {
                let _ = self.source_registry.sync_remote_source(src, &cache_dir);
            }
        }

        self.reload_sources();
        self.reload_catalog();
        self.set_status(format!("Source '{}' added successfully", alias));
        Ok(())
    }

    fn handle_skills_pane_key(&mut self, key: KeyEvent) -> Result<()> {
        let skills_count = self.filtered_skills().len();
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected_skill_idx > 0 {
                    self.selected_skill_idx -= 1;
                    self.preview_scroll = 0;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected_skill_idx + 1 < skills_count {
                    self.selected_skill_idx += 1;
                    self.preview_scroll = 0;
                }
            }
            KeyCode::Char('K') | KeyCode::PageUp => {
                self.preview_scroll = self.preview_scroll.saturating_sub(5);
            }
            KeyCode::Char('J') | KeyCode::PageDown => {
                self.preview_scroll += 5;
            }
            KeyCode::Char(' ') => {
                if let Some(skill) = self.current_selected_skill() {
                    if self.selected_skill_ids.contains(&skill.id) {
                        self.selected_skill_ids.remove(&skill.id);
                    } else {
                        self.selected_skill_ids.insert(skill.id);
                    }
                }
            }
            KeyCode::Char('a') => {
                self.open_deploy_modal();
            }
            KeyCode::Char('d') => {
                self.toggle_selected_skills()?;
            }
            KeyCode::Char('r') | KeyCode::Delete => {
                self.open_confirm_undeploy();
            }
            KeyCode::Char('s') => {
                self.sync_remote_sources()?;
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_preview_pane_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.preview_scroll > 0 {
                    self.preview_scroll -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.preview_scroll += 1;
            }
            KeyCode::PageUp => {
                self.preview_scroll = self.preview_scroll.saturating_sub(10);
            }
            KeyCode::PageDown => {
                self.preview_scroll += 10;
            }
            _ => {}
        }
        Ok(())
    }

    fn open_deploy_modal(&mut self) {
        let mut target_skills = Vec::new();
        if !self.selected_skill_ids.is_empty() {
            target_skills.extend(self.selected_skill_ids.iter().cloned());
        } else if let Some(skill) = self.current_selected_skill() {
            target_skills.push(skill.id.clone());
        }

        if target_skills.is_empty() {
            self.set_status("No skills available to deploy");
            return;
        }

        let detected_ids: HashSet<String> = crate::core::detector::detect_agents_in_project(
            &self.current_project_dir,
            &self.config.agents,
        )
        .into_iter()
        .map(|d| d.id)
        .collect();

        let mut agents = Vec::new();
        for (id, agent) in &self.config.agents {
            if agent.enabled {
                let is_detected = detected_ids.contains(id);
                agents.push(DeployAgentItem {
                    id: id.clone(),
                    display_name: agent.display_name.clone(),
                    selected: is_detected,
                    detected: is_detected,
                });
            }
        }

        let default_scope = ScopeChoice::CurrentProject;

        self.active_modal = ActiveModal::Deploy(DeployModalState {
            mode: self.config.settings.default_deployment_mode.clone(),
            scope_choice: default_scope,
            agents,
            focused_field: 0,
            selected_agent_idx: 0,
        });
    }

    fn handle_deploy_modal_key(&mut self, key: KeyEvent) -> Result<()> {
        let (execute_deploy, cancel) = if let ActiveModal::Deploy(ref mut modal) = self.active_modal
        {
            match key.code {
                KeyCode::Esc => (false, true),
                KeyCode::Tab | KeyCode::Down => {
                    modal.focused_field = (modal.focused_field + 1) % 4;
                    (false, false)
                }
                KeyCode::BackTab | KeyCode::Up => {
                    modal.focused_field = if modal.focused_field == 0 {
                        3
                    } else {
                        modal.focused_field - 1
                    };
                    (false, false)
                }
                KeyCode::Left | KeyCode::Right => {
                    if modal.focused_field == 0 {
                        modal.mode = match modal.mode {
                            DeployMode::Symlink => DeployMode::Copy,
                            DeployMode::Copy => DeployMode::Symlink,
                        };
                    } else if modal.focused_field == 1 {
                        modal.scope_choice = match modal.scope_choice {
                            ScopeChoice::CurrentProject => ScopeChoice::Global,
                            ScopeChoice::Global => ScopeChoice::CurrentProject,
                        };
                    }
                    (false, false)
                }
                KeyCode::Char(' ') => {
                    if modal.focused_field == 2 {
                        if let Some(agent) = modal.agents.get_mut(modal.selected_agent_idx) {
                            agent.selected = !agent.selected;
                        }
                    }
                    (false, false)
                }
                KeyCode::Char('k') => {
                    if modal.focused_field == 2 && modal.selected_agent_idx > 0 {
                        modal.selected_agent_idx -= 1;
                    }
                    (false, false)
                }
                KeyCode::Char('j') => {
                    if modal.focused_field == 2 && modal.selected_agent_idx + 1 < modal.agents.len()
                    {
                        modal.selected_agent_idx += 1;
                    }
                    (false, false)
                }
                KeyCode::Enter
                    if modal.focused_field == 3
                        || modal.focused_field == 0
                        || modal.focused_field == 1 =>
                {
                    (true, false)
                }
                _ => (false, false),
            }
        } else {
            (false, false)
        };

        if cancel {
            self.active_modal = ActiveModal::None;
        } else if execute_deploy {
            self.execute_deployment()?;
            self.active_modal = ActiveModal::None;
        }

        Ok(())
    }

    fn execute_deployment(&mut self) -> Result<()> {
        let (mode, scope_choice, selected_agent_ids) =
            if let ActiveModal::Deploy(ref modal) = self.active_modal {
                let agents: Vec<String> = modal
                    .agents
                    .iter()
                    .filter(|a| a.selected)
                    .map(|a| a.id.clone())
                    .collect();
                (modal.mode.clone(), modal.scope_choice, agents)
            } else {
                return Ok(());
            };

        if selected_agent_ids.is_empty() {
            self.set_status("No agents selected. Press [Space] on an agent to select it.");
            return Ok(());
        }

        let scope = match scope_choice {
            ScopeChoice::Global => Scope::Global,
            ScopeChoice::CurrentProject => Scope::Project(self.current_project_dir.clone()),
        };

        let skills_to_deploy: Vec<Skill> = if !self.selected_skill_ids.is_empty() {
            self.filtered_skills()
                .into_iter()
                .filter(|s| self.selected_skill_ids.contains(&s.id))
                .collect()
        } else if let Some(skill) = self.current_selected_skill() {
            vec![skill]
        } else {
            Vec::new()
        };

        let mut deployed_count = 0;
        for skill in &skills_to_deploy {
            for agent_id in &selected_agent_ids {
                if let Some(agent_cfg) = self.config.agents.get(agent_id) {
                    DeployController::deploy(
                        &mut self.manifest,
                        skill,
                        agent_id,
                        agent_cfg,
                        scope.clone(),
                        mode.clone(),
                    )?;
                    deployed_count += 1;
                }
            }
        }

        self.selected_skill_ids.clear();
        self.reload_catalog();
        self.set_status(format!(
            "Successfully deployed {} skill instance(s) using {:?}",
            deployed_count, mode
        ));
        Ok(())
    }

    fn open_confirm_undeploy(&mut self) {
        let filter = self.current_source_filter();
        let target_skills: Vec<Skill> = if !self.selected_skill_ids.is_empty() {
            self.filtered_skills()
                .into_iter()
                .filter(|s| self.selected_skill_ids.contains(&s.id))
                .collect()
        } else if let Some(skill) = self.current_selected_skill() {
            vec![skill]
        } else {
            Vec::new()
        };

        if target_skills.is_empty() {
            self.set_status("No skills selected for removal");
            return;
        }

        let skill_names: Vec<String> = target_skills.iter().map(|s| s.name.clone()).collect();
        let names_str = if skill_names.len() == 1 {
            format!("'{}'", skill_names[0])
        } else {
            format!("{} skill(s)", skill_names.len())
        };

        match filter {
            Some("installed:project") => {
                self.active_modal = ActiveModal::Confirm(ConfirmModalState {
                    title: "Delete Skill from Project".to_string(),
                    message: format!(
                        "Permanently delete {} from Current Project agent directories? (y/n)",
                        names_str
                    ),
                    action: ConfirmAction::UndeployProject(target_skills),
                });
            }
            Some("installed:global") => {
                self.active_modal = ActiveModal::Confirm(ConfirmModalState {
                    title: "Delete Skill from Global".to_string(),
                    message: format!(
                        "Permanently delete {} from Global agent directories (~/.agents/skills)? (y/n)",
                        names_str
                    ),
                    action: ConfirmAction::UndeployGlobal(target_skills),
                });
            }
            _ => {
                self.active_modal = ActiveModal::Confirm(ConfirmModalState {
                    title: "Undeploy Skill".to_string(),
                    message: format!(
                        "Remove/undeploy {} from all active deployed agents? (y/n)",
                        names_str
                    ),
                    action: ConfirmAction::UndeployAll(target_skills),
                });
            }
        }
    }

    fn execute_confirm_action(&mut self, action: ConfirmAction) -> Result<()> {
        match action {
            ConfirmAction::UndeployProject(skills) => {
                let mut removed_count = 0;
                for skill in skills {
                    // 1. Remove from manifest for current project
                    let to_remove: Vec<(String, String, Scope)> = self
                        .manifest
                        .deployments
                        .iter()
                        .filter(|d| {
                            let in_proj = matches!(&d.scope, Scope::Project(p) if p == &self.current_project_dir);
                            let matches_name = d.skill_id == skill.id
                                || d.skill_id.ends_with(&format!("/{}", skill.name))
                                || d.target_path == skill.path;
                            in_proj && matches_name
                        })
                        .map(|d| (d.skill_id.clone(), d.agent_id.clone(), d.scope.clone()))
                        .collect();

                    for (s_id, a_id, sc) in to_remove {
                        DeployController::undeploy(&mut self.manifest, &s_id, &a_id, &sc)?;
                        removed_count += 1;
                    }

                    // 2. Remove physical project files/symlinks
                    for agent_cfg in self.config.agents.values() {
                        let proj_agent_dir = self.current_project_dir.join(&agent_cfg.project_dir);
                        let target = proj_agent_dir.join(&skill.name);
                        if target.exists() || target.is_symlink() {
                            let _ = remove_path(&target);
                            removed_count += 1;
                        }
                        let file_target = proj_agent_dir
                            .join(agent_cfg.target_file.replace("{skill_name}", &skill.name));
                        if file_target.exists() {
                            let _ = remove_path(&file_target);
                            removed_count += 1;
                        }
                    }
                }
                self.selected_skill_ids.clear();
                self.reload_catalog();
                self.set_status(format!("Removed {} project skill target(s)", removed_count));
            }
            ConfirmAction::UndeployGlobal(skills) => {
                let mut removed_count = 0;
                for skill in skills {
                    // 1. Remove from manifest for global
                    let to_remove: Vec<(String, String, Scope)> = self
                        .manifest
                        .deployments
                        .iter()
                        .filter(|d| {
                            let in_glob = matches!(&d.scope, Scope::Global);
                            let matches_name = d.skill_id == skill.id
                                || d.skill_id.ends_with(&format!("/{}", skill.name))
                                || d.target_path == skill.path;
                            in_glob && matches_name
                        })
                        .map(|d| (d.skill_id.clone(), d.agent_id.clone(), d.scope.clone()))
                        .collect();

                    for (s_id, a_id, sc) in to_remove {
                        DeployController::undeploy(&mut self.manifest, &s_id, &a_id, &sc)?;
                        removed_count += 1;
                    }

                    // 2. Remove physical global files/symlinks
                    for agent_cfg in self.config.agents.values() {
                        let glob_agent_dir = expand_path(&agent_cfg.global_dir);
                        let target = glob_agent_dir.join(&skill.name);
                        if target.exists() || target.is_symlink() {
                            let _ = remove_path(&target);
                            removed_count += 1;
                        }
                        let file_target = glob_agent_dir
                            .join(agent_cfg.target_file.replace("{skill_name}", &skill.name));
                        if file_target.exists() {
                            let _ = remove_path(&file_target);
                            removed_count += 1;
                        }
                    }
                }
                self.selected_skill_ids.clear();
                self.reload_catalog();
                self.set_status(format!("Removed {} global skill target(s)", removed_count));
            }
            ConfirmAction::UndeployAll(skills) => {
                let mut removed_count = 0;
                for skill in skills {
                    let deployments = self.manifest.get_deployments_for_skill(&skill.id);
                    let to_remove: Vec<(String, String, Scope)> = deployments
                        .into_iter()
                        .map(|d| (d.skill_id.clone(), d.agent_id.clone(), d.scope.clone()))
                        .collect();

                    for (s_id, a_id, sc) in to_remove {
                        DeployController::undeploy(&mut self.manifest, &s_id, &a_id, &sc)?;
                        removed_count += 1;
                    }
                }
                self.selected_skill_ids.clear();
                self.reload_catalog();
                self.set_status(format!("Undeployed {} skill instance(s)", removed_count));
            }
            ConfirmAction::RemoveSource(alias) => {
                self.source_registry.remove(&alias)?;
                self.reload_sources();
                self.reload_catalog();
                self.set_status(format!("Removed source '{}' from registry", alias));
            }
        }
        Ok(())
    }

    fn toggle_selected_skills(&mut self) -> Result<()> {
        let is_proj = self.current_source_filter() == Some("installed:project");
        let is_glob = self.current_source_filter() == Some("installed:global");

        let target_skills: Vec<Skill> = if !self.selected_skill_ids.is_empty() {
            self.filtered_skills()
                .into_iter()
                .filter(|s| self.selected_skill_ids.contains(&s.id))
                .collect()
        } else if let Some(skill) = self.current_selected_skill() {
            vec![skill]
        } else {
            Vec::new()
        };

        let mut toggled_count = 0;
        for skill in target_skills {
            let deployments = self.manifest.get_deployments_for_skill(&skill.id);
            let to_toggle: Vec<(String, String, Scope)> = deployments
                .into_iter()
                .filter(|d| {
                    if is_proj {
                        matches!(&d.scope, Scope::Project(p) if p == &self.current_project_dir)
                    } else if is_glob {
                        matches!(&d.scope, Scope::Global)
                    } else {
                        true
                    }
                })
                .map(|d| (d.skill_id.clone(), d.agent_id.clone(), d.scope.clone()))
                .collect();

            for (s_id, a_id, sc) in to_toggle {
                DeployController::toggle(&mut self.manifest, &s_id, &a_id, &sc)?;
                toggled_count += 1;
            }
        }

        if toggled_count > 0 {
            self.set_status(format!("Toggled state for {} deployment(s)", toggled_count));
        } else {
            self.set_status("Selected skill has no active deployment to toggle");
        }
        Ok(())
    }

    pub fn sync_remote_sources(&mut self) -> Result<()> {
        let cache_dir = expand_path(&self.config.settings.sources_cache_dir);
        let mut synced = 0;
        for source in &self.source_registry.sources {
            if source.source_type == SourceType::RemoteGit && source.enabled {
                self.source_registry
                    .sync_remote_source(source, &cache_dir)?;
                synced += 1;
            }
        }
        self.reload_catalog();
        self.set_status(format!("Synchronized {} remote source(s)", synced));
        Ok(())
    }
}
