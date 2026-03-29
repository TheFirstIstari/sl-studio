use steinline_lib::core::database::{Database, IntelligenceEntry};
use tempfile::TempDir;

#[test]
fn test_database_initialization() {
    let tmp_dir = TempDir::new().unwrap();
    let reg_path = tmp_dir.path().join("registry.db");
    let intel_path = tmp_dir.path().join("intel.db");

    let db = Database::new(reg_path.to_str().unwrap(), intel_path.to_str().unwrap()).unwrap();

    assert_eq!(db.get_registry_count().unwrap(), 0);
    assert_eq!(db.get_intelligence_count().unwrap(), 0);
    assert_eq!(db.get_processed_count().unwrap(), 0);
}

#[test]
fn test_database_insert_fingerprint() {
    let tmp_dir = TempDir::new().unwrap();
    let reg_path = tmp_dir.path().join("registry.db");
    let intel_path = tmp_dir.path().join("intel.db");

    let db = Database::new(reg_path.to_str().unwrap(), intel_path.to_str().unwrap()).unwrap();

    let id = db
        .insert_fingerprint("abc123", "/test/path.pdf", "pdf", 1024, "path.pdf")
        .unwrap();

    assert!(id > 0);
    assert_eq!(db.get_registry_count().unwrap(), 1);
}

#[test]
fn test_database_duplicate_fingerprint() {
    let tmp_dir = TempDir::new().unwrap();
    let reg_path = tmp_dir.path().join("registry.db");
    let intel_path = tmp_dir.path().join("intel.db");

    let db = Database::new(reg_path.to_str().unwrap(), intel_path.to_str().unwrap()).unwrap();

    db.insert_fingerprint("abc123", "/test/path.pdf", "pdf", 1024, "path.pdf")
        .unwrap();
    db.insert_fingerprint("abc123", "/test/path.pdf", "pdf", 1024, "path.pdf")
        .unwrap();

    assert_eq!(db.get_registry_count().unwrap(), 1);
}

#[test]
fn test_database_mark_processed() {
    let tmp_dir = TempDir::new().unwrap();
    let reg_path = tmp_dir.path().join("registry.db");
    let intel_path = tmp_dir.path().join("intel.db");

    let db = Database::new(reg_path.to_str().unwrap(), intel_path.to_str().unwrap()).unwrap();

    db.insert_fingerprint("abc123", "/test/path.pdf", "pdf", 1024, "path.pdf")
        .unwrap();
    db.mark_processed("abc123").unwrap();

    assert_eq!(db.get_processed_count().unwrap(), 1);
}

#[test]
fn test_database_get_unprocessed_files() {
    let tmp_dir = TempDir::new().unwrap();
    let reg_path = tmp_dir.path().join("registry.db");
    let intel_path = tmp_dir.path().join("intel.db");

    let db = Database::new(reg_path.to_str().unwrap(), intel_path.to_str().unwrap()).unwrap();

    db.insert_fingerprint("abc123", "/test/path1.pdf", "pdf", 1024, "path1.pdf")
        .unwrap();
    db.insert_fingerprint("def456", "/test/path2.pdf", "pdf", 1024, "path2.pdf")
        .unwrap();

    let unprocessed = db.get_unprocessed_files(10).unwrap();
    assert_eq!(unprocessed.len(), 2);
}

#[test]
fn test_database_insert_intelligence() {
    let tmp_dir = TempDir::new().unwrap();
    let reg_path = tmp_dir.path().join("registry.db");
    let intel_path = tmp_dir.path().join("intel.db");

    let db = Database::new(reg_path.to_str().unwrap(), intel_path.to_str().unwrap()).unwrap();

    db.insert_fingerprint("abc123", "/test/path.pdf", "pdf", 1024, "path.pdf")
        .unwrap();

    let entry = IntelligenceEntry {
        id: 0,
        registry_id: 1,
        fingerprint: "abc123".to_string(),
        filename: "path.pdf".to_string(),
        evidence_quote: Some("test quote".to_string()),
        evidence_full: Some("full text".to_string()),
        associated_date: Some("2024-01-01".to_string()),
        fact_summary: "Test fact".to_string(),
        category: Some("Test".to_string()),
        identified_crime: Some("None".to_string()),
        severity_score: 5,
        confidence: Some(0.9),
        processing_time_ms: Some(100),
    };

    db.insert_intelligence(&entry).unwrap();
    assert_eq!(db.get_intelligence_count().unwrap(), 1);
}

#[test]
fn test_database_get_intelligence() {
    let tmp_dir = TempDir::new().unwrap();
    let reg_path = tmp_dir.path().join("registry.db");
    let intel_path = tmp_dir.path().join("intel.db");

    let db = Database::new(reg_path.to_str().unwrap(), intel_path.to_str().unwrap()).unwrap();

    db.insert_fingerprint("abc123", "/test/path.pdf", "pdf", 1024, "path.pdf")
        .unwrap();

    let entry = IntelligenceEntry {
        id: 0,
        registry_id: 1,
        fingerprint: "abc123".to_string(),
        filename: "path.pdf".to_string(),
        evidence_quote: Some("test quote".to_string()),
        evidence_full: None,
        associated_date: None,
        fact_summary: "Test fact".to_string(),
        category: None,
        identified_crime: None,
        severity_score: 5,
        confidence: None,
        processing_time_ms: None,
    };

    db.insert_intelligence(&entry).unwrap();

    let results = db.get_intelligence(10, 0).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].fact_summary, "Test fact");
}

#[test]
fn test_database_checkpoints() {
    let tmp_dir = TempDir::new().unwrap();
    let reg_path = tmp_dir.path().join("registry.db");
    let intel_path = tmp_dir.path().join("intel.db");

    let db = Database::new(reg_path.to_str().unwrap(), intel_path.to_str().unwrap()).unwrap();

    let job_id = db.checkpoint_start("analysis", "job-001").unwrap();
    assert!(job_id > 0);

    db.checkpoint_update("job-001", "fingerprint123", 10)
        .unwrap();

    let checkpoint = db.get_active_checkpoint("analysis").unwrap();
    assert!(checkpoint.is_some());
    assert_eq!(checkpoint.unwrap().total_processed, 10);

    db.checkpoint_complete("job-001").unwrap();
}

#[test]
fn test_database_audit_log() {
    let tmp_dir = TempDir::new().unwrap();
    let reg_path = tmp_dir.path().join("registry.db");
    let intel_path = tmp_dir.path().join("intel.db");

    let db = Database::new(reg_path.to_str().unwrap(), intel_path.to_str().unwrap()).unwrap();

    db.log_audit("test_action", "test details", Some(100))
        .unwrap();
}
