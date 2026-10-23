use asm::core::scanner::scan_skills_in_dir;
use asm::core::skill::Skill;
use std::path::PathBuf;

#[test]
fn test_fixture_git_commit_parsing() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture_dir = manifest_dir
        .join("fixtures")
        .join("skills")
        .join("git-commit");

    let skill = Skill::parse_from_dir("official", &fixture_dir).expect("Failed to parse skill");
    assert_eq!(skill.name, "git-commit");
    assert_eq!(skill.source_alias, "official");
    assert_eq!(skill.version, Some("1.0.0".to_string()));
    assert_eq!(skill.tags, vec!["git", "workflow", "productivity"]);
    assert_eq!(skill.author, Some("asm-team".to_string()));
    assert!(skill.description.contains("Conventional Commits"));
}

#[test]
fn test_fixture_fallback_parsing() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture_dir = manifest_dir
        .join("fixtures")
        .join("skills")
        .join("code-review");

    let skill =
        Skill::parse_from_dir("team", &fixture_dir).expect("Failed to parse fallback skill");
    assert_eq!(skill.name, "Code Reviewer");
    assert!(skill.description.contains("Analyzes pull requests"));
}

#[test]
fn test_scanner_discovers_fixtures() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixtures_root = manifest_dir.join("fixtures").join("skills");

    let skills = scan_skills_in_dir("test-source", &fixtures_root);
    assert!(skills.len() >= 2);
    let names: Vec<String> = skills.iter().map(|s| s.name.clone()).collect();
    assert!(names.contains(&"git-commit".to_string()));
    assert!(names.contains(&"Code Reviewer".to_string()));
}
