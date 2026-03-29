use std::fs;
use steinline_lib::config::{AppConfig, ValidationResult};
use tempfile::TempDir;

#[test]
fn test_config_default_creation() {
    let config = AppConfig::default();
    assert_eq!(config.version, "0.1.0");
    assert_eq!(config.project.name, "New Investigation");
    assert_eq!(config.hardware.cpu_workers > 0, true);
}

#[test]
fn test_config_save_and_load() {
    let tmp_dir = TempDir::new().unwrap();
    let config_path = tmp_dir.path().join("config.json");

    let mut config = AppConfig::default();
    config.project.evidence_root = "/tmp".to_string();
    config.project.registry_db = "/tmp/registry.db".to_string();
    config.project.intelligence_db = "/tmp/intelligence.db".to_string();

    let result = config.validate();
    assert_eq!(result.valid, true);
    assert_eq!(result.errors.len(), 0);
}

#[test]
fn test_config_validation_valid() {
    let mut config = AppConfig::default();
    config.project.evidence_root = "/tmp".to_string();
    config.project.registry_db = "/tmp/registry.db".to_string();
    config.project.intelligence_db = "/tmp/intelligence.db".to_string();

    let result = config.validate();
    assert_eq!(result.valid, true);
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
