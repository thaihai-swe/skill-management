use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Cell, Clear, HighlightSpacing, Paragraph, Row, Table, Wrap,
    },
    Frame,
};

use crate::config::schema::DeployMode;
use crate::deploy::manifest::{DeployStatus, Scope};
use crate::tui::app::{ActiveModal, ActivePane, App, ScopeChoice};
use crate::tui::theme::Theme;

pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();
    if area.width < 60 || area.height < 14 {
        let msg = vec![
            Line::from(""),
            Line::from(Span::styled(
                "⚠️  Terminal Window Too Small",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(format!(
                "Current: {}x{}  |  Minimum: 60x14",
                area.width, area.height
            )),
            Line::from(""),
            Line::from(Span::styled(
                "Please enlarge your terminal window to continue.",
                Style::default().fg(Color::DarkGray),
            )),
        ];
        let p = Paragraph::new(msg).alignment(Alignment::Center).block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Yellow)),
        );
        f.render_widget(p, area);
        return;
    }

    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header (with right-aligned toast alert)
            Constraint::Min(10),   // 3-Pane Body
            Constraint::Length(3), // Footer: Shortcuts & Guidelines ONLY
        ])
        .split(area);

    draw_header(f, app, main_chunks[0]);
    draw_body(f, app, main_chunks[1]);
    draw_footer(f, app, main_chunks[2]);

    // Modals
    match &app.active_modal {
        ActiveModal::Deploy(modal) => draw_deploy_modal(f, modal),
        ActiveModal::Confirm(modal) => draw_confirm_modal(f, modal),
        ActiveModal::Help => draw_help_modal(f),
        ActiveModal::AddSource(modal) => draw_add_source_modal(f, modal),
        ActiveModal::None => {}
    }
}

fn draw_header(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray));
    f.render_widget(block, area);

    let inner = Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    };

    let toast_len: u16 = if let Some((ref msg, _)) = app.status_message {
        (msg.len() as u16 + 6).min(inner.width / 2)
    } else {
        0
    };

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(30), Constraint::Length(toast_len)])
        .split(inner);

    let project_name = app
        .current_project_dir
        .file_name()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| ".".to_string());

    let active_agents_count = app.config.agents.values().filter(|a| a.enabled).count();
    let detected_agents = crate::core::detector::detect_agents_in_project(
        &app.current_project_dir,
        &app.config.agents,
    );

    let mut left_spans = vec![
        Span::styled(
            " asm ",
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" Project: "),
        Span::styled(
            project_name,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!(
                " │ Sources: {} │ Agents: {}",
                app.source_registry.sources.len(),
                active_agents_count
            ),
            Style::default().fg(Color::DarkGray),
        ),
    ];

    if !detected_agents.is_empty() {
        left_spans.push(Span::styled(
            " │ Detected: ",
            Style::default().fg(Color::DarkGray),
        ));
        for (i, d) in detected_agents.iter().enumerate() {
            if i > 0 {
                left_spans.push(Span::raw(", "));
            }
            left_spans.push(Span::styled(
                &d.display_name,
                Style::default().fg(Color::Green),
            ));
        }
    }
    f.render_widget(Paragraph::new(Line::from(left_spans)), chunks[0]);

    if let Some((ref msg, _)) = app.status_message {
        let toast = Line::from(vec![
            Span::styled(
                "● ",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                format!(" {} ", msg),
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            ),
        ]);
        f.render_widget(Paragraph::new(toast).alignment(Alignment::Right), chunks[1]);
    }
}

fn draw_body(f: &mut Frame, app: &App, area: Rect) {
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(24), // Sources & Installed Scopes
            Constraint::Percentage(44), // Skills Table
            Constraint::Percentage(32), // Preview
        ])
        .split(area);

    draw_sources_pane(f, app, body_chunks[0]);
    draw_skills_pane(f, app, body_chunks[1]);
    draw_preview_pane(f, app, body_chunks[2]);
}

fn draw_sources_pane(f: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.active_pane == ActivePane::Sources;
    let border_color = if is_focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let mut lines = Vec::new();
    let project_cnt = app.get_project_skills().len();
    let global_cnt = app.get_global_skills().len();
    let all_cnt = app.catalog.skills.len();

    // 1. INSTALLED SKILLS Header
    lines.push(Line::from(Span::styled(
        " 📍 INSTALLED SKILLS",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));

    // Project Skills (idx 0)
    let is_proj_sel = app.selected_source_idx == 0;
    let proj_style = if is_proj_sel {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    lines.push(Line::from(vec![
        Span::styled(if is_proj_sel { " > " } else { "   " }, proj_style),
        Span::styled("⭐ Current Project", proj_style),
        Span::styled(
            format!(" ({})", project_cnt),
            Style::default().fg(Color::DarkGray),
        ),
    ]));

    // Global Skills (idx 1)
    let is_glob_sel = app.selected_source_idx == 1;
    let glob_style = if is_glob_sel {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    lines.push(Line::from(vec![
        Span::styled(if is_glob_sel { " > " } else { "   " }, glob_style),
        Span::styled("🌐 Global Agents", glob_style),
        Span::styled(
            format!(" ({})", global_cnt),
            Style::default().fg(Color::DarkGray),
        ),
    ]));

    lines.push(Line::from(""));

    // 2. SOURCES Header
    lines.push(Line::from(Span::styled(
        " 📦 UPSTREAM SOURCES (Global)",
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    )));

    // All Available (idx 2)
    let is_all_sel = app.selected_source_idx == 2;
    let all_style = if is_all_sel {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::White)
    };
    lines.push(Line::from(vec![
        Span::styled(if is_all_sel { " > " } else { "   " }, all_style),
        Span::styled("📁 All Available", all_style),
        Span::styled(
            format!(" ({})", all_cnt),
            Style::default().fg(Color::DarkGray),
        ),
    ]));

    // Registered individual sources
    let has_sources = app.sources_list.len() > 3;
    if has_sources {
        lines.push(Line::from(Span::styled(
            "   ── Registered Sources ────",
            Style::default().fg(Color::DarkGray),
        )));
    }

    for idx in 3..app.sources_list.len() {
        let source_key = &app.sources_list[idx];
        let alias = source_key
            .strip_prefix("source:")
            .unwrap_or(source_key.as_str());
        let cnt = app
            .catalog
            .skills
            .iter()
            .filter(|s| s.source_alias == alias)
            .count();

        let is_sel = idx == app.selected_source_idx;
        let style = if is_sel {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };

        lines.push(Line::from(vec![
            Span::styled(if is_sel { " > " } else { "   • " }, style),
            Span::styled(alias, style),
            Span::styled(format!(" ({})", cnt), Style::default().fg(Color::DarkGray)),
        ]));
    }

    if !has_sources {
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "   No sources added.",
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(vec![
            Span::raw("   Press "),
            Span::styled(
                "[+]",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" to add Git repo"),
        ]));
    }

    let p = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color))
            .title(Line::from(vec![
                Span::styled(
                    " Navigation ",
                    Style::default().add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    "[+ Add] ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
            ])),
    );
    f.render_widget(p, area);
}

fn draw_skills_pane(f: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.active_pane == ActivePane::Skills;
    let border_color = if is_focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let is_project_view = app.current_source_filter() == Some("installed:project");
    let is_global_view = app.current_source_filter() == Some("installed:global");

    let skills = app.filtered_skills();
    let header_cells = if is_project_view || is_global_view {
        [
            "Sel",
            "Skill Name",
            "Target Agent",
            "Scope",
            "Status",
            "Mode",
        ]
    } else {
        ["Sel", "Skill Name", "Source", "Scope", "Status", "Mode"]
    };

    let header = Row::new(header_cells.iter().map(|h| {
        Cell::from(*h).style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
    }))
    .height(1)
    .bottom_margin(1);

    let rows = skills.iter().enumerate().map(|(idx, skill)| {
        let is_cursor = idx == app.selected_skill_idx;
        let is_checked = app.selected_skill_ids.contains(&skill.id);
        let check_str = if is_checked { "[x]" } else { "[ ]" };

        let deployments = app.manifest.get_deployments_for_skill(&skill.id);

        let (target_agent_or_source, scope_str, status_str, mode_str) =
            if is_project_view {
                let agent_str = if let Some(dep) = deployments.iter().find(
                    |d| matches!(&d.scope, Scope::Project(p) if p == &app.current_project_dir),
                ) {
                    dep.agent_id.clone()
                } else {
                    "universal-agent".to_string()
                };
                let mode = if skill.path.is_symlink() {
                    "⚡ Symlink"
                } else {
                    "📋 Copy"
                };
                let status = if let Some(dep) = deployments.iter().find(
                    |d| matches!(&d.scope, Scope::Project(p) if p == &app.current_project_dir),
                ) {
                    match dep.status {
                        DeployStatus::Enabled => ("✓ Enabled", Color::Green),
                        DeployStatus::Disabled => ("○ Disabled", Color::DarkGray),
                    }
                } else {
                    ("● Standalone", Color::Cyan)
                };
                (
                    agent_str,
                    "Current Project".to_string(),
                    status,
                    mode.to_string(),
                )
            } else if is_global_view {
                let agent_str = if let Some(dep) = deployments
                    .iter()
                    .find(|d| matches!(&d.scope, Scope::Global))
                {
                    dep.agent_id.clone()
                } else {
                    "universal-agent".to_string()
                };
                let mode = if skill.path.is_symlink() {
                    "⚡ Symlink"
                } else {
                    "📋 Copy"
                };
                let status = if let Some(dep) = deployments
                    .iter()
                    .find(|d| matches!(&d.scope, Scope::Global))
                {
                    match dep.status {
                        DeployStatus::Enabled => ("✓ Enabled", Color::Green),
                        DeployStatus::Disabled => ("○ Disabled", Color::DarkGray),
                    }
                } else {
                    ("● Standalone", Color::Cyan)
                };
                (agent_str, "Global".to_string(), status, mode.to_string())
            } else {
                let has_proj = deployments.iter().any(
                    |d| matches!(&d.scope, Scope::Project(p) if p == &app.current_project_dir),
                );
                let has_glob = deployments
                    .iter()
                    .any(|d| matches!(&d.scope, Scope::Global));

                let (scope_str, status_str, mode_str) = if has_proj && has_glob {
                    (
                        "Project + Global".to_string(),
                        ("✓ Deployed", Color::Green),
                        "Mixed".to_string(),
                    )
                } else if has_proj {
                    let dep = deployments
                    .iter()
                    .find(
                        |d| matches!(&d.scope, Scope::Project(p) if p == &app.current_project_dir),
                    )
                    .unwrap();
                    let status = match dep.status {
                        DeployStatus::Enabled => ("✓ Enabled (Proj)", Color::Green),
                        DeployStatus::Disabled => ("○ Disabled (Proj)", Color::DarkGray),
                    };
                    let mode = match dep.mode {
                        DeployMode::Symlink => "⚡ Symlink",
                        DeployMode::Copy => "📋 Copy",
                    };
                    ("Current Project".to_string(), status, mode.to_string())
                } else if has_glob {
                    let dep = deployments
                        .iter()
                        .find(|d| matches!(&d.scope, Scope::Global))
                        .unwrap();
                    let status = match dep.status {
                        DeployStatus::Enabled => ("✓ Enabled (Glob)", Color::Green),
                        DeployStatus::Disabled => ("○ Disabled (Glob)", Color::DarkGray),
                    };
                    let mode = match dep.mode {
                        DeployMode::Symlink => "⚡ Symlink",
                        DeployMode::Copy => "📋 Copy",
                    };
                    ("Global".to_string(), status, mode.to_string())
                } else {
                    (
                        "--".to_string(),
                        ("Available", Color::DarkGray),
                        "--".to_string(),
                    )
                };
                (skill.source_alias.clone(), scope_str, status_str, mode_str)
            };

        let row_style = if is_cursor {
            Style::default()
                .bg(Color::Rgb(40, 45, 60))
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        Row::new(vec![
            Cell::from(check_str),
            Cell::from(skill.name.clone()),
            Cell::from(target_agent_or_source),
            Cell::from(scope_str),
            Cell::from(Span::styled(
                status_str.0,
                Style::default().fg(status_str.1),
            )),
            Cell::from(mode_str),
        ])
        .style(row_style)
    });

    let pane_title = if is_project_view {
        format!(" ⭐ Current Project Skills ({}) ", skills.len())
    } else if is_global_view {
        format!(" 🌐 Global Agent Skills ({}) ", skills.len())
    } else {
        format!(" 📦 Available Skills ({}) ", skills.len())
    };

    if skills.is_empty() {
        let empty_msg = if !app.search_query.is_empty() {
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    "🔍 No skills matching your search query",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::raw("Query: "),
                    Span::styled(
                        format!("\"{}\"", app.search_query),
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
                ]),
                Line::from(""),
                Line::from(Span::styled(
                    "Press [Esc] to clear search and display all skills.",
                    Style::default().fg(Color::DarkGray),
                )),
            ]
        } else if is_project_view {
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    "📂 No skills installed in this project yet",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Target agent paths: .agents/skills, .claude/skills, .cursor/rules, etc.",
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::raw("1. Press "),
                    Span::styled(
                        "[h] or [Left]",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" to switch to the Sources pane."),
                ]),
                Line::from(vec![
                    Span::raw("2. Select "),
                    Span::styled(
                        "[📁 All Available]",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" and navigate back with "),
                    Span::styled(
                        "[l] or [Tab]",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("."),
                ]),
                Line::from(vec![
                    Span::raw("3. Highlight any skill and press "),
                    Span::styled(
                        "[a]",
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" to deploy!"),
                ]),
            ]
        } else if is_global_view {
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    "🌐 No skills installed in global agent directories",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(Span::styled(
                    "Target global paths: ~/.agents/skills, ~/.claude/skills, etc.",
                    Style::default().fg(Color::DarkGray),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::raw("Select any upstream skill in "),
                    Span::styled(
                        "[📁 All Available]",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(", press "),
                    Span::styled(
                        "[a]",
                        Style::default()
                            .fg(Color::Green)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" and choose Scope: "),
                    Span::styled(
                        "Global",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("."),
                ]),
            ]
        } else {
            vec![
                Line::from(""),
                Line::from(Span::styled(
                    "📦 No skill sources registered yet",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(""),
                Line::from(vec![
                    Span::raw("Press "),
                    Span::styled(
                        "[+] or [n]",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" to add a Git repository or local folder."),
                ]),
                Line::from(""),
                Line::from(Span::styled(
                    "Example: https://github.com/thaihai-swe/Agent-Engineering-Skills",
                    Style::default().fg(Color::DarkGray),
                )),
            ]
        };

        let p = Paragraph::new(empty_msg)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(border_color))
                    .title(Span::styled(
                        pane_title,
                        Style::default().add_modifier(Modifier::BOLD),
                    )),
            )
            .alignment(Alignment::Center);
        f.render_widget(p, area);
        return;
    }

    let table = Table::new(
        rows,
        [
            Constraint::Length(5),
            Constraint::Percentage(32),
            Constraint::Percentage(20),
            Constraint::Percentage(18),
            Constraint::Percentage(15),
            Constraint::Percentage(10),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(border_color))
            .title(Span::styled(
                pane_title,
                Style::default().add_modifier(Modifier::BOLD),
            )),
    )
    .highlight_spacing(HighlightSpacing::Always);

    f.render_widget(table, area);
}

fn draw_preview_pane(f: &mut Frame, app: &App, area: Rect) {
    let is_focused = app.active_pane == ActivePane::Preview;
    let border_color = if is_focused {
        Color::Cyan
    } else {
        Color::DarkGray
    };

    let skill = app.current_selected_skill();
    let mut lines = Vec::new();

    if let Some(s) = skill {
        // Title
        lines.push(Line::from(vec![
            Span::styled("Skill: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                s.name.clone(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
        ]));

        // Metadata badges
        let mut badges = Vec::new();
        if let Some(ref ver) = s.version {
            badges.push(Span::styled(
                format!(" v{} ", ver),
                Style::default().fg(Color::Black).bg(Color::Blue),
            ));
            badges.push(Span::raw(" "));
        }
        if let Some(ref auth) = s.author {
            badges.push(Span::styled(
                format!(" @{} ", auth),
                Style::default().fg(Color::Black).bg(Color::DarkGray),
            ));
            badges.push(Span::raw(" "));
        }
        for tag in &s.tags {
            badges.push(Span::styled(
                format!(" #{} ", tag),
                Style::default().fg(Color::Yellow),
            ));
            badges.push(Span::raw(" "));
        }
        if !badges.is_empty() {
            lines.push(Line::from(badges));
        }

        lines.push(Line::from(vec![
            Span::styled("Source: ", Style::default().fg(Color::DarkGray)),
            Span::styled(s.source_alias.clone(), Style::default().fg(Color::White)),
        ]));

        lines.push(Line::from(vec![
            Span::styled("Location: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                s.path.to_string_lossy().to_string(),
                Style::default().fg(Color::DarkGray),
            ),
        ]));

        lines.push(Line::from(Span::styled(
            "─".repeat(area.width.saturating_sub(4) as usize),
            Style::default().fg(Color::DarkGray),
        )));

        // Description
        lines.push(Line::from(Span::styled(
            s.description.clone(),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::ITALIC),
        )));

        lines.push(Line::from(""));

        // Content
        for line in s.raw_content.lines().skip(app.preview_scroll) {
            if line.starts_with("# ") {
                lines.push(Line::from(Span::styled(
                    line.to_string(),
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )));
            } else if line.starts_with("## ") {
                lines.push(Line::from(Span::styled(
                    line.to_string(),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )));
            } else if line.starts_with("### ") {
                lines.push(Line::from(Span::styled(
                    line.to_string(),
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )));
            } else if line.starts_with("```") {
                lines.push(Line::from(Span::styled(
                    line.to_string(),
                    Style::default().fg(Color::Magenta),
                )));
            } else if line.starts_with("- ") || line.starts_with("* ") {
                lines.push(Line::from(vec![
                    Span::styled(" • ", Style::default().fg(Color::Cyan)),
                    Span::raw(line[2..].to_string()),
                ]));
            } else {
                lines.push(Line::from(line.to_string()));
            }
        }
    } else {
        lines.push(Line::from(Span::styled(
            "No skill selected for preview.",
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(""));
        lines.push(Line::from(Span::styled(
            "Select a skill from the table to inspect SKILL.md instructions.",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let preview_title = if app.preview_scroll > 0 {
        format!(" Skill Preview [Line {}] ", app.preview_scroll + 1)
    } else {
        " Skill Preview ".to_string()
    };

    let p = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border_color))
                .title(Span::styled(
                    preview_title,
                    Style::default().add_modifier(Modifier::BOLD),
                )),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(p, area);
}

fn draw_footer(f: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::DarkGray));

    let content = if app.is_searching {
        Line::from(vec![
            Theme::keycap("SEARCH", Color::Yellow),
            Span::raw(" "),
            Span::styled(
                &app.search_query,
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("█", Style::default().fg(Color::Yellow)),
            Span::styled(
                "   (Press [Enter] to focus results, [Esc] to cancel)",
                Style::default().fg(Color::DarkGray),
            ),
        ])
    } else {
        let shortcut_spans = match &app.active_modal {
            ActiveModal::AddSource(_) => {
                let mut v = vec![Span::styled(
                    "ADD SOURCE: ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )];
                v.extend(Theme::pill_primary("Tab / ↓", "Next Field"));
                v.extend(Theme::pill_primary("← / →", "Scope"));
                v.extend(Theme::pill_success("Enter", "Save & Sync"));
                v.extend(Theme::pill_danger("Esc", "Cancel"));
                v
            }
            ActiveModal::Deploy(_) => {
                let mut v = vec![Span::styled(
                    "DEPLOY: ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )];
                v.extend(Theme::pill_primary("Tab / ↓", "Next Field"));
                v.extend(Theme::pill_primary("← / →", "Mode/Scope"));
                v.extend(Theme::pill_warning("Space", "Toggle Agent"));
                v.extend(Theme::pill_success("Enter", "Deploy"));
                v.extend(Theme::pill_danger("Esc", "Cancel"));
                v
            }
            ActiveModal::Confirm(_) => {
                let mut v = vec![Span::styled(
                    "CONFIRM: ",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                )];
                v.extend(Theme::pill_danger("Y / Enter", "Confirm"));
                v.extend(Theme::pill_muted("N / Esc", "Cancel"));
                v
            }
            ActiveModal::Help => {
                let mut v = vec![Span::styled(
                    "HELP: ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )];
                v.extend(Theme::pill_primary("Esc / ? / q", "Close Help"));
                v
            }
            ActiveModal::None => match app.active_pane {
                ActivePane::Sources => {
                    let mut v = vec![Span::styled(
                        "SOURCES: ",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )];
                    v.extend(Theme::pill_warning("+ / a", "Add Source"));
                    v.extend(Theme::pill_primary("Space", "Toggle"));
                    v.extend(Theme::pill_danger("r / Del", "Remove"));
                    v.extend(Theme::pill_primary("s", "Sync"));
                    v.extend(Theme::pill_muted("Tab", "Skills Table"));
                    v.extend(Theme::pill_muted("?", "Help"));
                    v
                }
                ActivePane::Skills => {
                    let is_project = app.current_source_filter() == Some("installed:project");
                    let is_global = app.current_source_filter() == Some("installed:global");
                    if is_project {
                        let mut v = vec![Span::styled(
                            "PROJECT: ",
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        )];
                        v.extend(Theme::pill_danger("r / Del", "Delete"));
                        v.extend(Theme::pill_primary("d", "Toggle"));
                        v.extend(Theme::pill_success("a", "Deploy Global"));
                        v.extend(Theme::pill_warning("Space", "Select"));
                        v.extend(Theme::pill_primary("h / l", "Panes"));
                        v.extend(Theme::pill_primary("J / K", "Prev Scroll"));
                        v.extend(Theme::pill_muted("/", "Search"));
                        v
                    } else if is_global {
                        let mut v = vec![Span::styled(
                            "GLOBAL: ",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        )];
                        v.extend(Theme::pill_danger("r / Del", "Delete"));
                        v.extend(Theme::pill_primary("d", "Toggle"));
                        v.extend(Theme::pill_success("a", "Deploy Proj"));
                        v.extend(Theme::pill_warning("Space", "Select"));
                        v.extend(Theme::pill_primary("h / l", "Panes"));
                        v.extend(Theme::pill_primary("J / K", "Prev Scroll"));
                        v.extend(Theme::pill_muted("/", "Search"));
                        v
                    } else {
                        let mut v = vec![Span::styled(
                            "UPSTREAM: ",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        )];
                        v.extend(Theme::pill_success("a", "Deploy"));
                        v.extend(Theme::pill_danger("r / Del", "Undeploy"));
                        v.extend(Theme::pill_primary("d", "Toggle"));
                        v.extend(Theme::pill_warning("Space", "Select"));
                        v.extend(Theme::pill_primary("h / l", "Panes"));
                        v.extend(Theme::pill_primary("J / K", "Prev Scroll"));
                        v.extend(Theme::pill_primary("s", "Sync"));
                        v.extend(Theme::pill_muted("/", "Search"));
                        v
                    }
                }
                ActivePane::Preview => {
                    let mut v = vec![Span::styled(
                        "PREVIEW: ",
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )];
                    v.extend(Theme::pill_primary("j / k", "Scroll"));
                    v.extend(Theme::pill_primary("PgUp / PgDn", "Jump 10"));
                    v.extend(Theme::pill_muted("Tab", "Sources"));
                    v.extend(Theme::pill_muted("?", "Help"));
                    v
                }
            },
        };
        Line::from(shortcut_spans)
    };

    let p = Paragraph::new(content)
        .block(block)
        .alignment(Alignment::Left);
    f.render_widget(p, area);
}

fn draw_deploy_modal(f: &mut Frame, modal: &crate::tui::app::DeployModalState) {
    let area = centered_rect(65, 65, f.area());
    f.render_widget(Clear, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3), // 1. Mode
            Constraint::Length(3), // 2. Scope
            Constraint::Min(6),    // 3. Agents Checkbox List
            Constraint::Length(3), // 4. Submit button
        ])
        .split(area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Span::styled(
            " Deploy Skills to Coding Agents ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));
    f.render_widget(block, area);

    // 1. Mode selection
    let symlink_style = if modal.mode == DeployMode::Symlink {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let copy_style = if modal.mode == DeployMode::Copy {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let mode_border = if modal.focused_field == 0 {
        Color::Yellow
    } else {
        Color::DarkGray
    };
    let mode_content = Line::from(vec![
        Span::raw(" "),
        Span::styled(
            if modal.mode == DeployMode::Symlink {
                "[● Symlink (Live sync)]"
            } else {
                "[○ Symlink]"
            },
            symlink_style,
        ),
        Span::raw("      "),
        Span::styled(
            if modal.mode == DeployMode::Copy {
                "[● Copy (Standalone)]"
            } else {
                "[○ Copy]"
            },
            copy_style,
        ),
        Span::styled("   (← / → to switch)", Style::default().fg(Color::DarkGray)),
    ]);
    let mode_p = Paragraph::new(mode_content).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(mode_border))
            .title(" 1. Deployment Mode "),
    );
    f.render_widget(mode_p, chunks[0]);

    // 2. Scope selection
    let proj_style = if modal.scope_choice == ScopeChoice::CurrentProject {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let glob_style = if modal.scope_choice == ScopeChoice::Global {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let scope_border = if modal.focused_field == 1 {
        Color::Yellow
    } else {
        Color::DarkGray
    };
    let scope_content = Line::from(vec![
        Span::raw(" "),
        Span::styled(
            if modal.scope_choice == ScopeChoice::CurrentProject {
                "[● Current Project]"
            } else {
                "[○ Current Project]"
            },
            proj_style,
        ),
        Span::raw("    "),
        Span::styled(
            if modal.scope_choice == ScopeChoice::Global {
                "[● Global (~/.agents/skills)]"
            } else {
                "[○ Global]"
            },
            glob_style,
        ),
        Span::styled("   (← / → to switch)", Style::default().fg(Color::DarkGray)),
    ]);
    let scope_p = Paragraph::new(scope_content).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(scope_border))
            .title(" 2. Deployment Scope "),
    );
    f.render_widget(scope_p, chunks[1]);

    // 3. Agent Checkbox List
    let agents_border = if modal.focused_field == 2 {
        Color::Yellow
    } else {
        Color::DarkGray
    };

    let mut agent_lines = Vec::new();
    for (idx, agent) in modal.agents.iter().enumerate() {
        let is_cursor = modal.focused_field == 2 && idx == modal.selected_agent_idx;
        let check_box = if agent.selected { "[x] " } else { "[ ] " };
        let prefix = if is_cursor { "> " } else { "  " };

        let style = if is_cursor {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else if agent.selected {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::White)
        };

        let mut spans = vec![
            Span::styled(prefix, style),
            Span::styled(check_box, style),
            Span::styled(&agent.display_name, style),
        ];
        if agent.detected {
            spans.push(Span::styled(
                " (Detected)",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ));
        }

        agent_lines.push(Line::from(spans));
    }

    let agents_p = Paragraph::new(agent_lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(agents_border))
            .title(" 3. Target Agents [Space: toggle, j/k: navigate] "),
    );
    f.render_widget(agents_p, chunks[2]);

    // 4. Submit button
    let submit_style = if modal.focused_field == 3 {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Green)
    };
    let submit_p = Paragraph::new(Line::from(Span::styled(
        " [ Enter to Apply Deployment ] ",
        submit_style,
    )))
    .alignment(Alignment::Center);
    f.render_widget(submit_p, chunks[3]);
}

fn draw_confirm_modal(f: &mut Frame, modal: &crate::tui::app::ConfirmModalState) {
    let area = centered_rect(52, 28, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Red))
        .title(Span::styled(
            format!(" {} ", modal.title),
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ));

    let content = vec![
        Line::from(""),
        Line::from(Span::styled(
            &modal.message,
            Style::default().fg(Color::White),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "This will remove the selected skill files/symlinks from the target agent folders.",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                " [ Y / Enter to Confirm ] ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Red)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("   "),
            Span::styled(
                " [ N / Esc to Cancel ] ",
                Style::default().fg(Color::White).bg(Color::DarkGray),
            ),
        ]),
    ];

    let p = Paragraph::new(content)
        .block(block)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: true });
    f.render_widget(p, area);
}

fn draw_help_modal(f: &mut Frame) {
    let area = centered_rect(65, 75, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Span::styled(
            " Keyboard Shortcuts Guide ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));

    let help_text = vec![
        Line::from(Span::styled(
            "Global Navigation",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("Tab / BackTab   ", Style::default().fg(Color::Cyan)),
            Span::raw("Cycle active pane (Sources ⇄ Skills ⇄ Preview)"),
        ]),
        Line::from(vec![
            Span::styled("h / l or ← / →  ", Style::default().fg(Color::Cyan)),
            Span::raw("Quick switch between Sources and Skills panes"),
        ]),
        Line::from(vec![
            Span::styled("J / K or PgUp/Dn", Style::default().fg(Color::Cyan)),
            Span::raw("Scroll preview panel directly from skills table"),
        ]),
        Line::from(vec![
            Span::styled("+ / n / F2      ", Style::default().fg(Color::Cyan)),
            Span::raw("Add skill source (Git URL or local directory)"),
        ]),
        Line::from(vec![
            Span::styled("/               ", Style::default().fg(Color::Cyan)),
            Span::raw("Start live fuzzy search; Esc to cancel"),
        ]),
        Line::from(vec![
            Span::styled("?               ", Style::default().fg(Color::Cyan)),
            Span::raw("Toggle this help popup"),
        ]),
        Line::from(vec![
            Span::styled("q / Ctrl+c      ", Style::default().fg(Color::Cyan)),
            Span::raw("Quit asm"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Managing Skills & Sources",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(vec![
            Span::styled("Space           ", Style::default().fg(Color::Cyan)),
            Span::raw("Toggle multi-selection on highlighted skill"),
        ]),
        Line::from(vec![
            Span::styled("a               ", Style::default().fg(Color::Cyan)),
            Span::raw("Open Deploy modal (Symlink/Copy, Scope, Target Agents)"),
        ]),
        Line::from(vec![
            Span::styled("d               ", Style::default().fg(Color::Cyan)),
            Span::raw("Toggle Enable / Disable status on selected skill(s)"),
        ]),
        Line::from(vec![
            Span::styled("r / Delete      ", Style::default().fg(Color::Cyan)),
            Span::raw("Delete/remove skill from Project/Global, or remove source"),
        ]),
        Line::from(vec![
            Span::styled("s               ", Style::default().fg(Color::Cyan)),
            Span::raw("Sync all remote Git skill sources (git pull silently)"),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Press Esc or ? to close",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let p = Paragraph::new(help_text)
        .block(block)
        .wrap(Wrap { trim: true });
    f.render_widget(p, area);
}

fn draw_add_source_modal(f: &mut Frame, modal: &crate::tui::app::AddSourceModalState) {
    let area = centered_rect(65, 62, f.area());
    f.render_widget(Clear, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3), // 1. Alias
            Constraint::Length(3), // 2. Location
            Constraint::Length(3), // 3. Scope
            Constraint::Length(4), // Helper examples
            Constraint::Length(3), // 4. Submit button
        ])
        .split(area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Span::styled(
            " Add Skill Source (Git Repo or Local Folder) ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));
    f.render_widget(block, area);

    // 1. Alias input
    let alias_border = if modal.focused_field == 0 {
        Color::Yellow
    } else {
        Color::DarkGray
    };
    let alias_content = Line::from(vec![
        Span::styled(&modal.alias, Style::default().fg(Color::White)),
        if modal.focused_field == 0 {
            Span::styled("█", Style::default().fg(Color::Yellow))
        } else {
            Span::raw("")
        },
    ]);
    let alias_p = Paragraph::new(alias_content).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(alias_border))
            .title(Span::styled(
                " 1. Source Alias (Short Name) ",
                if modal.focused_field == 0 {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                },
            )),
    );
    f.render_widget(alias_p, chunks[0]);

    // 2. Location input
    let loc_border = if modal.focused_field == 1 {
        Color::Yellow
    } else {
        Color::DarkGray
    };
    let loc_content = Line::from(vec![
        Span::styled(&modal.location, Style::default().fg(Color::White)),
        if modal.focused_field == 1 {
            Span::styled("█", Style::default().fg(Color::Yellow))
        } else {
            Span::raw("")
        },
    ]);
    let loc_p = Paragraph::new(loc_content).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(loc_border))
            .title(Span::styled(
                " 2. Location (GitHub URL or Local Folder Path) ",
                if modal.focused_field == 1 {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                },
            )),
    );
    f.render_widget(loc_p, chunks[1]);

    // 3. Scope selection
    let scope_border = if modal.focused_field == 2 {
        Color::Yellow
    } else {
        Color::DarkGray
    };
    let global_style = if modal.scope == crate::core::source::SourceScope::Global {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let proj_style = if modal.scope == crate::core::source::SourceScope::Project {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let scope_content = Line::from(vec![
        Span::raw(" "),
        Span::styled(
            if modal.scope == crate::core::source::SourceScope::Global {
                "[● Global (Default, ~/.config/asm)]"
            } else {
                "[○ Global]"
            },
            global_style,
        ),
        Span::raw("    "),
        Span::styled(
            if modal.scope == crate::core::source::SourceScope::Project {
                "[● Current Project (.asm/)]"
            } else {
                "[○ Current Project]"
            },
            proj_style,
        ),
        Span::styled("   (← / → to switch)", Style::default().fg(Color::DarkGray)),
    ]);
    let scope_p = Paragraph::new(scope_content).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(scope_border))
            .title(Span::styled(
                " 3. Source Scope ",
                if modal.focused_field == 2 {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                },
            )),
    );
    f.render_widget(scope_p, chunks[2]);

    // 4. Helper text
    let help_lines = vec![
        Line::from(Span::styled(
            "Examples:",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(vec![
            Span::styled("  • Git:   ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                "https://github.com/thaihai-swe/Agent-Engineering-Skills",
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(vec![
            Span::styled("  • Local: ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                "~/my-skills  or  ./fixtures/skills",
                Style::default().fg(Color::Cyan),
            ),
        ]),
    ];
    f.render_widget(Paragraph::new(help_lines), chunks[3]);

    // 5. Submit button
    let submit_style = if modal.focused_field == 3 {
        Style::default()
            .fg(Color::Black)
            .bg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Green)
    };
    let submit_p = Paragraph::new(Line::from(vec![
        Span::styled(" [ Enter to Add & Sync ] ", submit_style),
        Span::raw("    "),
        Span::styled(" [Esc to Cancel]", Style::default().fg(Color::DarkGray)),
    ]))
    .alignment(Alignment::Center);
    f.render_widget(submit_p, chunks[4]);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
