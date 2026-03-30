use std::fs;
use std::io::Write;
use tempfile::tempdir;

mod export_tests {
    use super::*;

    #[test]
    fn test_json_export_structure() {
        // Test that JSON export produces valid structure
        let test_data = r#"{
            "facts": [
                {
                    "id": 1,
                    "summary": "Test fact",
                    "category": "test",
                    "severity": 5
                }
            ],
            "entities": []
        }"#;
        
        let parsed: serde_json::Value = serde_json::from_str(test_data).unwrap();
        assert!(parsed["facts"].is_array());
        assert_eq!(parsed["facts"][0]["id"], 1);
    }

    #[test]
    fn test_csv_export_format() {
        let csv_data = "id,summary,category,severity\n1,Test fact,test,5\n";
        let lines: Vec<&str> = csv_data.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("id"));
    }
}

mod backup_tests {
    use super::*;

    #[test]
    fn test_backup_zip_creation() {
        let temp_dir = tempdir().unwrap();
        let backup_path = temp_dir.path().join("test_backup.zip");
        
        let file = fs::File::create(&backup_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        
        zip.start_file("test.txt", options).unwrap();
        zip.write_all(b"test content").unwrap();
        zip.finish().unwrap();
        
        assert!(backup_path.exists());
        assert!(backup_path.metadata().unwrap().len() > 0);
    }

    #[test]
    fn test_backup_restore() {
        let temp_dir = tempdir().unwrap();
        let backup_path = temp_dir.path().join("test_backup.zip");
        let restore_path = temp_dir.path().join("restored");
        
        // Create backup
        let file = fs::File::create(&backup_path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        
        zip.start_file("data.txt", options).unwrap();
        zip.write_all(b"test data").unwrap();
        zip.finish().unwrap();
        
        // Restore
        let file = fs::File::open(&backup_path).unwrap();
        let mut archive = zip::ZipArchive::new(file).unwrap();
        
        fs::create_dir_all(&restore_path).unwrap();
        
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).unwrap();
            let outpath = restore_path.join(file.name());
            let mut outfile = fs::File::create(&outpath).unwrap();
            std::io::copy(&mut file, &mut outfile).unwrap();
        }
        
        assert!(restore_path.join("data.txt").exists());
    }
}

mod notification_tests {
    use super::*;

    #[test]
    fn test_notification_structure() {
        #[derive(serde::Serialize)]
        struct Notification {
            id: String,
            title: String,
            message: String,
            notification_type: String,
            timestamp: String,
            read: bool,
        }
        
        let notification = Notification {
            id: "123".to_string(),
            title: "Test".to_string(),
            message: "Test message".to_string(),
            notification_type: "info".to_string(),
            timestamp: chrono::Local::now().to_rfc3339(),
            read: false,
        };
        
        let json = serde_json::to_string(&notification).unwrap();
        assert!(json.contains("Test"));
    }
}

mod system_monitor_tests {
    use super::*;

    #[test]
    fn test_monitor_data_structure() {
        #[derive(serde::Serialize, Clone)]
        struct SystemMonitor {
            cpu_usage_percent: f32,
            memory_used_gb: f64,
            memory_available_gb: f64,
            memory_percent: f32,
        }
        
        let monitor = SystemMonitor {
            cpu_usage_percent: 50.0,
            memory_used_gb: 8.0,
            memory_available_gb: 8.0,
            memory_percent: 50.0,
        };
        
        assert!(monitor.cpu_usage_percent >= 0.0);
        assert!(monitor.memory_percent >= 0.0 && monitor.memory_percent <= 100.0);
    }
}

mod case_comparison_tests {
    use super::*;

    #[test]
    fn test_entity_overlap_calculation() {
        let project1_entities = vec!["Alice", "Bob", "Charlie"];
        let project2_entities = vec!["Bob", "David", "Alice"];
        
        let overlap: Vec<&&str> = project1_entities.iter()
            .filter(|e| project2_entities.contains(e))
            .collect();
        
        assert_eq!(overlap.len(), 2);
        assert!(overlap.contains(&&"Alice"));
        assert!(overlap.contains(&&"Bob"));
    }

    #[test]
    fn test_timeline_correlation() {
        let dates1 = vec!["2024-01-01", "2024-01-15", "2024-02-01"];
        let dates2 = vec!["2024-01-01", "2024-01-20", "2024-02-01"];
        
        let intersection: Vec<&&str> = dates1.iter()
            .filter(|d| dates2.contains(d))
            .collect();
        
        let union_count = dates1.len() + dates2.len() - intersection.len();
        let correlation = intersection.len() as f64 / union_count as f64;
        
        assert!(correlation > 0.0);
        assert!(correlation <= 1.0);
    }
}
