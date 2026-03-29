use steinline_lib::config::ProjectFile;
use tempfile::TempDir;

#[test]
fn test_project_default_creation() {
    let project = ProjectFile::default();

    assert_eq!(project.version, "1.0.0");
    assert_eq!(project.investigator.name, "");
    assert_eq!(project.paths.evidence_root, "");
    assert!(project.metadata.total_files == 0);
}

#[test]
fn test_project_save_and_load() {
    let tmp_dir = TempDir::new().unwrap();
    let project_path = tmp_dir.path().join("test.sls");

    let mut project = ProjectFile::default();
    project.investigator.name = "John Doe".to_string();
    project.investigator.case_number = "CASE-2024-001".to_string();
    project.paths.evidence_root = "/path/to/evidence".to_string();
    project.metadata.total_files = 100;

    project.save(&project_path).unwrap();
    assert!(project_path.exists());

    let loaded = ProjectFile::load(&project_path).unwrap();

    assert_eq!(loaded.version, "1.0.0");
    assert_eq!(loaded.investigator.name, "John Doe");
    assert_eq!(loaded.investigator.case_number, "CASE-2024-001");
    assert_eq!(loaded.paths.evidence_root, "/path/to/evidence");
    assert_eq!(loaded.metadata.total_files, 100);
}

#[test]
fn test_project_serialization() {
    let project = ProjectFile::default();

    let json = serde_json::to_string_pretty(&project).unwrap();
    assert!(json.contains("version"));
    assert!(json.contains("investigator"));
    assert!(json.contains("paths"));
    assert!(json.contains("model"));
    assert!(json.contains("hardware"));
    assert!(json.contains("processing"));
    assert!(json.contains("metadata"));
}

#[test]
fn test_project_update_modified() {
    let tmp_dir = TempDir::new().unwrap();
    let _project_path = tmp_dir.path().join("test.sls");

    let mut project = ProjectFile::default();
    let original_modified = project.modified_at;

    std::thread::sleep(std::time::Duration::from_millis(10));

    project.update_modified();

    assert!(project.modified_at > original_modified);
}
