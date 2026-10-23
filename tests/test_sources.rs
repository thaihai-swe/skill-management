use asm::core::source::{SourceRegistry, SourceScope, SourceType};
use std::path::PathBuf;

#[test]
fn test_source_registry_operations() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture_dir = manifest_dir.join("fixtures").join("skills");

    let mut registry = SourceRegistry::default();
    registry
        .add(
            "fixtures".to_string(),
            fixture_dir.to_string_lossy().to_string(),
            SourceType::LocalFolder,
            SourceScope::Global,
        )
        .unwrap();

    assert_eq!(registry.sources.len(), 1);
    assert_eq!(registry.sources[0].alias, "fixtures");
    assert!(registry.sources[0].enabled);

    // Toggle
    let status = registry.toggle("fixtures").unwrap();
    assert!(!status);

    let status = registry.toggle("fixtures").unwrap();
    assert!(status);

    // Remove
    registry.remove("fixtures").unwrap();
    assert_eq!(registry.sources.len(), 0);
}
