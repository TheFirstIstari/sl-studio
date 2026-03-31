use steinline_lib::config::AppConfig;
use tempfile::TempDir;

#[test]
fn test_config_default_creation() {
    let config = AppConfig::default();
    assert_eq!(config.version, "0.2.0");
    assert_eq!(config.project.name, "New Investigation");
    assert_eq!(config.hardware.cpu_workers > 0, true);
}

#[test]
fn test_config_save_and_load() {
    let tmp_dir = TempDir::new().unwrap();
    let temp_path = tmp_dir.path().to_string_lossy().to_string();

    let mut config = AppConfig::default();
    config.project.evidence_root = temp_path.clone();
    config.project.registry_db = format!("{}/registry.db", temp_path);
    config.project.intelligence_db = format!("{}/intelligence.db", temp_path);

    let result = config.validate();
    assert_eq!(result.valid, true, "validation errors: {:?}", result.errors);
    assert_eq!(result.errors.len(), 0);
}

#[test]
fn test_config_validation_valid() {
    let tmp_dir = TempDir::new().unwrap();
    let temp_path = tmp_dir.path().to_string_lossy().to_string();

    let mut config = AppConfig::default();
    config.project.evidence_root = temp_path.clone();
    config.project.registry_db = format!("{}/registry.db", temp_path);
    config.project.intelligence_db = format!("{}/intelligence.db", temp_path);

    let result = config.validate();
    assert_eq!(result.valid, true, "validation errors: {:?}", result.errors);
    assert_eq!(result.errors.len(), 0);
}

#[test]
fn test_config_validation_missing_paths() {
    let config = AppConfig::default();
    let result = config.validate();

    assert_eq!(result.valid, false);
    assert!(result.errors.len() > 0);
}

#[test]
fn test_config_serialization() {
    let config = AppConfig::default();
    let json = serde_json::to_string(&config).unwrap();
    let loaded: AppConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(config.version, loaded.version);
    assert_eq!(config.project.name, loaded.project.name);
}
