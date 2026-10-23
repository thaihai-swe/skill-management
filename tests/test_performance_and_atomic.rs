use asm::config::schema::{AgentConfig, AppConfig, PackagingStrategy};
use asm::core::source::SourceRegistry;
use asm::deploy::fs_ops::write_atomic;
use asm::deploy::manifest::DeployManifest;
use asm::tui::app::App;
use std::fs;

#[test]
fn test_write_atomic_functionality() {
    let temp_dir = std::env::temp_dir().join(format!(
        "asm_atomic_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&temp_dir).unwrap();

    let target_file = temp_dir.join("nested").join("sub").join("data.json");
    let content1 = r#"{"version": 1, "status": "init"}"#;

    // Test write to non-existent nested path
    write_atomic(&target_file, content1).expect("write_atomic should succeed on nested path");
    assert!(target_file.exists());
    let read1 = fs::read_to_string(&target_file).unwrap();
    assert_eq!(read1, content1);

    // Test atomic overwrite
    let content2 = r#"{"version": 2, "status": "updated"}"#;
    write_atomic(&target_file, content2).expect("write_atomic overwrite should succeed");
    let read2 = fs::read_to_string(&target_file).unwrap();
    assert_eq!(read2, content2);

    // Verify no stray .tmp.* files left in directory
    let dir_entries: Vec<_> = fs::read_dir(target_file.parent().unwrap())
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(dir_entries.len(), 1, "Only target file should remain");

    // Clean up
    let _ = fs::remove_dir_all(temp_dir);
}

#[test]
fn test_app_state_caching_and_zero_disk_churn() {
    let temp_dir = std::env::temp_dir().join(format!(
        "asm_cache_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&temp_dir).unwrap();

    let project_dir = temp_dir.join("project");
    let global_dir = temp_dir.join("global");
    fs::create_dir_all(&project_dir).unwrap();
    fs::create_dir_all(&global_dir).unwrap();

    // Create a mock skill in project dir
    let proj_skill_dir = project_dir
        .join(".agents")
        .join("skills")
        .join("proj-skill");
    fs::create_dir_all(&proj_skill_dir).unwrap();
    fs::write(
        proj_skill_dir.join("SKILL.md"),
        "---\nname: proj-skill\ndescription: Project Skill\n---\n# Project Skill Doc",
    )
    .unwrap();

    // Create a mock skill in global dir
    let glob_skill_dir = global_dir.join("glob-skill");
    fs::create_dir_all(&glob_skill_dir).unwrap();
    fs::write(
        glob_skill_dir.join("SKILL.md"),
        "---\nname: glob-skill\ndescription: Global Skill\n---\n# Global Skill Doc",
    )
    .unwrap();

    let mut config = AppConfig::default();
    config.agents.insert(
        "agent-a".to_string(),
        AgentConfig {
            display_name: "Agent A".to_string(),
            enabled: true,
            global_dir: global_dir.to_string_lossy().to_string(),
            project_dir: ".agents/skills".to_string(),
            packaging: PackagingStrategy::Directory,
            target_file: "SKILL.md".to_string(),
        },
    );

    let source_registry = SourceRegistry::default();
    let manifest = DeployManifest::default();

    let app = App::new(config, source_registry, manifest, project_dir.clone());

    // 1. App should have pre-cached skills during App::new()
    assert_eq!(
        app.project_skills.len(),
        1,
        "Should have cached 1 project skill"
    );
    assert_eq!(app.project_skills[0].name, "proj-skill");
    assert_eq!(
        app.project_skills[0].raw_content,
        "---\nname: proj-skill\ndescription: Project Skill\n---\n# Project Skill Doc"
    );

    assert_eq!(
        app.global_skills.len(),
        1,
        "Should have cached 1 global skill"
    );
    assert_eq!(app.global_skills[0].name, "glob-skill");
    assert_eq!(
        app.global_skills[0].raw_content,
        "---\nname: glob-skill\ndescription: Global Skill\n---\n# Global Skill Doc"
    );

    // 2. get_project_skills & get_global_skills return cached slices without disk I/O
    assert_eq!(app.get_project_skills().len(), 1);
    assert_eq!(app.get_global_skills().len(), 1);

    // 3. filtered_skills on Project view returns cached project skills
    assert_eq!(app.selected_source_idx, 0); // Default is Project Skills
    let filtered = app.filtered_skills();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].name, "proj-skill");

    // Clean up
    let _ = fs::remove_dir_all(temp_dir);
}
