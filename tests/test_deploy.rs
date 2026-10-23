use std::fs;

use asm::config::schema::{AgentConfig, DeployMode, PackagingStrategy};
use asm::core::skill::Skill;
use asm::deploy::controller::DeployController;
use asm::deploy::manifest::{DeployManifest, DeployStatus, Scope};

#[test]
fn test_symlink_and_copy_deployment() {
    let temp_dir = std::env::temp_dir().join(format!(
        "asm_test_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&temp_dir).unwrap();

    // Create a self-contained test skill source inside temp_dir
    let fixture_dir = temp_dir.join("source_skill");
    fs::create_dir_all(&fixture_dir).unwrap();
    fs::write(
        fixture_dir.join("SKILL.md"),
        "---\nname: test-skill\ndescription: Test\n---\n# Test",
    )
    .unwrap();

    let skill = Skill::parse_from_dir("official", &fixture_dir).unwrap();
    let mut manifest = DeployManifest::default();

    let agent_cfg = AgentConfig {
        display_name: "Test Agent".to_string(),
        enabled: true,
        global_dir: temp_dir.join("global").to_string_lossy().to_string(),
        project_dir: ".test/skills".to_string(),
        packaging: PackagingStrategy::Directory,
        target_file: "SKILL.md".to_string(),
    };

    let scope = Scope::Project(temp_dir.clone());

    // 1. Deploy Symlink
    let entry = DeployController::deploy(
        &mut manifest,
        &skill,
        "test-agent",
        &agent_cfg,
        scope.clone(),
        DeployMode::Symlink,
    )
    .unwrap();

    assert!(entry.target_path.exists());
    assert!(entry.target_path.is_symlink());
    assert_eq!(manifest.deployments.len(), 1);
    assert_eq!(manifest.deployments[0].status, DeployStatus::Enabled);

    // 2. Toggle to Disabled
    let status = DeployController::toggle(&mut manifest, &skill.id, "test-agent", &scope).unwrap();
    assert_eq!(status, DeployStatus::Disabled);
    assert!(!entry.target_path.exists()); // Symlink removed

    // 3. Toggle back to Enabled
    let status = DeployController::toggle(&mut manifest, &skill.id, "test-agent", &scope).unwrap();
    assert_eq!(status, DeployStatus::Enabled);
    assert!(entry.target_path.exists()); // Symlink restored

    // 4. Undeploy
    DeployController::undeploy(&mut manifest, &skill.id, "test-agent", &scope).unwrap();
    assert!(!entry.target_path.exists());
    assert_eq!(manifest.deployments.len(), 0);

    // Cleanup
    let _ = fs::remove_dir_all(&temp_dir);
}
