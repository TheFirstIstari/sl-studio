use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryEntry {
    pub id: i64,
    pub fingerprint: String,
    pub path: String,
    pub file_size: Option<i64>,
    pub file_type: Option<String>,
    pub processed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntelligenceEntry {
    pub id: i64,
    pub registry_id: i64,
    pub fingerprint: String,
    pub filename: String,
    pub evidence_quote: Option<String>,
    pub evidence_full: Option<String>,
    pub associated_date: Option<String>,
    pub fact_summary: String,
    pub category: Option<String>,
    pub identified_crime: Option<String>,
    pub severity_score: i32,
    pub confidence: Option<f64>,
    pub processing_time_ms: Option<i64>,
}

pub struct Database {
    registry_conn: Mutex<Connection>,
    intelligence_conn: Mutex<Connection>,
}

impl Database {
    pub fn new(registry_path: &str, intelligence_path: &str) -> Result<Self> {
        let reg_conn = Connection::open(registry_path)?;
        let intel_conn = Connection::open(intelligence_path)?;

        let db = Database {
            registry_conn: Mutex::new(reg_conn),
            intelligence_conn: Mutex::new(intel_conn),
        };

        db.init_schema()?;
        Ok(db)
    }

    fn init_schema(&self) -> Result<()> {
        let reg_conn = self.registry_conn.lock().unwrap();
        let intel_conn = self.intelligence_conn.lock().unwrap();

        // Registry schema - optimized for fingerprint lookup and file tracking
        reg_conn.execute(
            "CREATE TABLE IF NOT EXISTS registry (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                fingerprint TEXT NOT NULL UNIQUE,
                path TEXT NOT NULL,
                file_size INTEGER,
                file_type TEXT,
                file_name TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                processed_at DATETIME,
                processed INTEGER DEFAULT 0
            )",
            [],
        )?;

        // Efficient indexes
        reg_conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_registry_fingerprint ON registry(fingerprint)",
            [],
        )?;
        reg_conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_registry_filetype ON registry(file_type)",
            [],
        )?;
        reg_conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_registry_path ON registry(path)",
            [],
        )?;
        reg_conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_registry_processed ON registry(processed)",
            [],
        )?;
        // Composite index for efficient processed file queries
        reg_conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_registry_processed_id ON registry(processed, id)",
            [],
        )?;

        // Intelligence schema - optimized for fact retrieval
        intel_conn.execute(
            "CREATE TABLE IF NOT EXISTS intelligence (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                registry_id INTEGER NOT NULL,
                fingerprint TEXT NOT NULL,
                filename TEXT NOT NULL,
                evidence_quote TEXT,
                evidence_full TEXT,
                associated_date TEXT,
                fact_summary TEXT NOT NULL,
                category TEXT,
                identified_crime TEXT,
                severity_score INTEGER DEFAULT 1,
                confidence REAL,
                processing_time_ms INTEGER,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        intel_conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_intelligence_registry ON intelligence(registry_id)",
            [],
        )?;
        intel_conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_intelligence_fingerprint ON intelligence(fingerprint)",
            [],
        )?;
        intel_conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_intelligence_category ON intelligence(category)",
            [],
        )?;
        intel_conn.execute("CREATE INDEX IF NOT EXISTS idx_intelligence_severity ON intelligence(severity_score DESC)", [])?;

        // Composite index for uniqueness and efficient lookups
        intel_conn.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_intelligence_unique ON intelligence(fingerprint, filename, fact_summary)",
            []
        )?;

        // Checkpoints for job resumption
        intel_conn.execute(
            "CREATE TABLE IF NOT EXISTS checkpoints (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                job_type TEXT NOT NULL,
                job_id TEXT,
                last_fingerprint TEXT,
                total_processed INTEGER DEFAULT 0,
                status TEXT NOT NULL DEFAULT 'pending',
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                completed_at DATETIME
            )",
            [],
        )?;

        intel_conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_checkpoints_job ON checkpoints(job_type, status)",
            [],
        )?;
        intel_conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_checkpoints_job_id ON checkpoints(job_id)",
            [],
        )?;

        // Audit log
        intel_conn.execute(
            "CREATE TABLE IF NOT EXISTS audit_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                action TEXT NOT NULL,
                details TEXT,
                duration_ms INTEGER,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        intel_conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_audit_action ON audit_log(action)",
            [],
        )?;
        intel_conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON audit_log(timestamp)",
            [],
        )?;

        // Text cache for extracted text
        reg_conn.execute(
            "CREATE TABLE IF NOT EXISTS text_cache (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                fingerprint TEXT NOT NULL UNIQUE,
                file_name TEXT NOT NULL,
                extracted_text TEXT,
                text_hash TEXT,
                extraction_time_ms INTEGER,
                quality_score REAL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        reg_conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_text_cache_fingerprint ON text_cache(fingerprint)",
            [],
        )?;
        reg_conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_text_cache_hash ON text_cache(text_hash)",
            [],
        )?;

        // Metadata extraction cache
        reg_conn.execute(
            "CREATE TABLE IF NOT EXISTS metadata_cache (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                fingerprint TEXT NOT NULL UNIQUE,
                metadata_type TEXT NOT NULL,
                metadata_json TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        reg_conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_metadata_fingerprint ON metadata_cache(fingerprint)",
            [],
        )?;
        reg_conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_metadata_type ON metadata_cache(metadata_type)",
            [],
        )?;

        info!("Database schema initialized");
        Ok(())
    }

    pub fn insert_fingerprint(
        &self,
        fingerprint: &str,
        path: &str,
        file_type: &str,
        file_size: i64,
        file_name: &str,
    ) -> Result<i64> {
        let conn = self.registry_conn.lock().unwrap();
        conn.execute(
            "INSERT OR IGNORE INTO registry (fingerprint, path, file_type, file_size, file_name) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![fingerprint, path, file_type, file_size, file_name],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn insert_fingerprints_batch(
        &self,
        entries: &[(String, String, String, i64, String)],
    ) -> Result<usize> {
        let mut conn = self.registry_conn.lock().unwrap();
        let tx = conn.transaction()?;
        let mut count = 0;

        {
            let mut stmt = tx.prepare_cached(
                "INSERT OR IGNORE INTO registry (fingerprint, path, file_type, file_size, file_name) VALUES (?1, ?2, ?3, ?4, ?5)"
            )?;

            for (fingerprint, path, file_type, file_size, file_name) in entries {
                if stmt
                    .execute(params![fingerprint, path, file_type, file_size, file_name])
                    .is_ok()
                {
                    count += 1;
                }
            }
        }

        tx.commit()?;
        Ok(count)
    }

    pub fn get_all_fingerprints(&self) -> Result<std::collections::HashSet<String>> {
        let conn = self.registry_conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT fingerprint FROM registry")?;
        let fingerprints = stmt.query_map([], |row| row.get(0))?;

        let mut set = std::collections::HashSet::new();
        for f in fingerprints.flatten() {
            set.insert(f);
        }
        Ok(set)
    }

    pub fn mark_processed(&self, fingerprint: &str) -> Result<()> {
        let conn = self.registry_conn.lock().unwrap();
        conn.execute(
            "UPDATE registry SET processed = 1, processed_at = CURRENT_TIMESTAMP WHERE fingerprint = ?1",
            params![fingerprint],
        )?;
        Ok(())
    }

    pub fn get_unprocessed_files(&self, limit: i64) -> Result<Vec<RegistryEntry>> {
        let conn = self.registry_conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, fingerprint, path, file_size, file_type, processed
             FROM registry
             WHERE processed = 0
             LIMIT ?1",
        )?;

        let entries = stmt.query_map([limit], |row| {
            Ok(RegistryEntry {
                id: row.get(0)?,
                fingerprint: row.get(1)?,
                path: row.get(2)?,
                file_size: row.get(3)?,
                file_type: row.get(4)?,
                processed: row.get(5)?,
            })
        })?;

        entries.collect()
    }

    pub fn insert_intelligence(&self, entry: &IntelligenceEntry) -> Result<()> {
        let conn = self.intelligence_conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO intelligence 
             (registry_id, fingerprint, filename, evidence_quote, evidence_full, 
              associated_date, fact_summary, category, identified_crime, severity_score, 
              confidence, processing_time_ms)
              VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)",
            params![
                entry.registry_id,
                entry.fingerprint,
                entry.filename,
                entry.evidence_quote,
                entry.evidence_full,
                entry.associated_date,
                entry.fact_summary,
                entry.category,
                entry.identified_crime,
                entry.severity_score,
                entry.confidence,
                entry.processing_time_ms
            ],
        )?;
        Ok(())
    }

    pub fn get_intelligence(&self, limit: i64, offset: i64) -> Result<Vec<IntelligenceEntry>> {
        let conn = self.intelligence_conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, registry_id, fingerprint, filename, evidence_quote, evidence_full,
                    associated_date, fact_summary, category, identified_crime,
                    severity_score, confidence, processing_time_ms
             FROM intelligence
             ORDER BY severity_score DESC, created_at DESC
             LIMIT ?1 OFFSET ?2",
        )?;

        let entries = stmt.query_map(params![limit, offset], |row| {
            Ok(IntelligenceEntry {
                id: row.get(0)?,
                registry_id: row.get(1)?,
                fingerprint: row.get(2)?,
                filename: row.get(3)?,
                evidence_quote: row.get(4)?,
                evidence_full: row.get(5)?,
                associated_date: row.get(6)?,
                fact_summary: row.get(7)?,
                category: row.get(8)?,
                identified_crime: row.get(9)?,
                severity_score: row.get(10)?,
                confidence: row.get(11)?,
                processing_time_ms: row.get(12)?,
            })
        })?;

        entries.collect()
    }

    pub fn checkpoint_start(&self, job_type: &str, job_id: &str) -> Result<i64> {
        let conn = self.intelligence_conn.lock().unwrap();
        conn.execute(
            "INSERT INTO checkpoints (job_type, job_id, status) VALUES (?1, ?2, 'running')",
            params![job_type, job_id],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn checkpoint_update(
        &self,
        job_id: &str,
        last_fingerprint: &str,
        total_processed: i64,
    ) -> Result<()> {
        let conn = self.intelligence_conn.lock().unwrap();
        conn.execute(
            "UPDATE checkpoints SET last_fingerprint = ?1, total_processed = ?2, updated_at = CURRENT_TIMESTAMP WHERE job_id = ?3",
            params![last_fingerprint, total_processed, job_id],
        )?;
        Ok(())
    }

    pub fn checkpoint_complete(&self, job_id: &str) -> Result<()> {
        let conn = self.intelligence_conn.lock().unwrap();
        conn.execute(
            "UPDATE checkpoints SET status = 'completed', completed_at = CURRENT_TIMESTAMP, updated_at = CURRENT_TIMESTAMP WHERE job_id = ?1",
            params![job_id],
        )?;
        Ok(())
    }

    pub fn get_active_checkpoint(&self, job_type: &str) -> Result<Option<JobCheckpoint>> {
        let conn = self.intelligence_conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, job_type, job_id, last_fingerprint, total_processed, status
             FROM checkpoints
             WHERE job_type = ?1 AND status = 'running'
             ORDER BY created_at DESC
             LIMIT 1",
        )?;

        let mut rows = stmt.query(params![job_type])?;
        if let Some(row) = rows.next()? {
            Ok(Some(JobCheckpoint {
                id: row.get(0)?,
                job_type: row.get(1)?,
                job_id: row.get(2)?,
                last_fingerprint: row.get(3)?,
                total_processed: row.get(4)?,
                status: row.get(5)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn log_audit(&self, action: &str, details: &str, duration_ms: Option<i64>) -> Result<()> {
        let conn = self.intelligence_conn.lock().unwrap();
        conn.execute(
            "INSERT INTO audit_log (action, details, duration_ms) VALUES (?1, ?2, ?3)",
            params![action, details, duration_ms],
        )?;
        Ok(())
    }

    pub fn get_registry_count(&self) -> Result<i64> {
        let conn = self.registry_conn.lock().unwrap();
        conn.query_row("SELECT COUNT(*) FROM registry", [], |row| row.get(0))
    }

    pub fn get_processed_count(&self) -> Result<i64> {
        let conn = self.registry_conn.lock().unwrap();
        conn.query_row(
            "SELECT COUNT(*) FROM registry WHERE processed = 1",
            [],
            |row| row.get(0),
        )
    }

    pub fn get_intelligence_count(&self) -> Result<i64> {
        let conn = self.intelligence_conn.lock().unwrap();
        conn.query_row("SELECT COUNT(*) FROM intelligence", [], |row| row.get(0))
    }

    pub fn get_all_counts(&self) -> Result<AllCounts> {
        let reg_conn = self.registry_conn.lock().unwrap();
        let intel_conn = self.intelligence_conn.lock().unwrap();

        let registry_count: i64 =
            reg_conn.query_row("SELECT COUNT(*) FROM registry", [], |row| row.get(0))?;
        let processed_count: i64 = reg_conn.query_row(
            "SELECT COUNT(*) FROM registry WHERE processed = 1",
            [],
            |row| row.get(0),
        )?;
        let intelligence_count: i64 =
            intel_conn.query_row("SELECT COUNT(*) FROM intelligence", [], |row| row.get(0))?;

        Ok(AllCounts {
            registry_count,
            processed_count,
            intelligence_count,
        })
    }

    // Text cache operations
    pub fn save_text_cache(
        &self,
        fingerprint: &str,
        file_name: &str,
        text: &str,
        text_hash: &str,
        extraction_time_ms: i64,
        quality_score: f64,
    ) -> Result<()> {
        let conn = self.registry_conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO text_cache (fingerprint, file_name, extracted_text, text_hash, extraction_time_ms, quality_score, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, CURRENT_TIMESTAMP)",
            params![fingerprint, file_name, text, text_hash, extraction_time_ms, quality_score],
        )?;
        Ok(())
    }

    pub fn get_text_cache(&self, fingerprint: &str) -> Result<Option<TextCacheEntry>> {
        let conn = self.registry_conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, fingerprint, file_name, extracted_text, text_hash, extraction_time_ms, quality_score
             FROM text_cache WHERE fingerprint = ?1"
        )?;

        let mut rows = stmt.query(params![fingerprint])?;
        if let Some(row) = rows.next()? {
            Ok(Some(TextCacheEntry {
                id: row.get(0)?,
                fingerprint: row.get(1)?,
                file_name: row.get(2)?,
                extracted_text: row.get(3)?,
                text_hash: row.get(4)?,
                extraction_time_ms: row.get(5)?,
                quality_score: row.get(6)?,
            }))
        } else {
            Ok(None)
        }
    }

    pub fn get_text_cache_count(&self) -> Result<i64> {
        let conn = self.registry_conn.lock().unwrap();
        conn.query_row("SELECT COUNT(*) FROM text_cache", [], |row| row.get(0))
    }

    // Metadata cache operations
    pub fn save_metadata_cache(
        &self,
        fingerprint: &str,
        metadata_type: &str,
        metadata_json: &str,
    ) -> Result<()> {
        let conn = self.registry_conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO metadata_cache (fingerprint, metadata_type, metadata_json)
             VALUES (?1, ?2, ?3)",
            params![fingerprint, metadata_type, metadata_json],
        )?;
        Ok(())
    }

    pub fn get_metadata_cache(
        &self,
        fingerprint: &str,
        metadata_type: &str,
    ) -> Result<Option<MetadataCacheEntry>> {
        let conn = self.registry_conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, fingerprint, metadata_type, metadata_json
             FROM metadata_cache WHERE fingerprint = ?1 AND metadata_type = ?2",
        )?;

        let mut rows = stmt.query(params![fingerprint, metadata_type])?;
        if let Some(row) = rows.next()? {
            Ok(Some(MetadataCacheEntry {
                id: row.get(0)?,
                fingerprint: row.get(1)?,
                metadata_type: row.get(2)?,
                metadata_json: row.get(3)?,
            }))
        } else {
            Ok(None)
        }
    }

    // Migration support
    pub fn get_schema_version(&self) -> Result<i32> {
        let conn = self.registry_conn.lock().unwrap();
        conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='schema_version'",
            [],
            |_| Ok(0),
        )
        .or_else(|_| {
            conn.execute(
                "CREATE TABLE IF NOT EXISTS schema_version (version INTEGER PRIMARY KEY)",
                [],
            )?;
            conn.execute("INSERT INTO schema_version (version) VALUES (1)", [])?;
            Ok(1)
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextCacheEntry {
    pub id: i64,
    pub fingerprint: String,
    pub file_name: String,
    pub extracted_text: String,
    pub text_hash: String,
    pub extraction_time_ms: i64,
    pub quality_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetadataCacheEntry {
    pub id: i64,
    pub fingerprint: String,
    pub metadata_type: String,
    pub metadata_json: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllCounts {
    pub registry_count: i64,
    pub processed_count: i64,
    pub intelligence_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobCheckpoint {
    pub id: i64,
    pub job_type: String,
    pub job_id: String,
    pub last_fingerprint: Option<String>,
    pub total_processed: i64,
    pub status: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_database_creation() {
        let tmp_dir = std::env::temp_dir().join("slstudio_test_db");
        fs::create_dir_all(&tmp_dir).unwrap();

        let reg_path = tmp_dir.join("test_registry.db");
        let intel_path = tmp_dir.join("test_intel.db");

        let db = Database::new(reg_path.to_str().unwrap(), intel_path.to_str().unwrap()).unwrap();

        assert!(db.get_registry_count().unwrap() == 0);
        assert!(db.get_processed_count().unwrap() == 0);
        assert!(db.get_intelligence_count().unwrap() == 0);

        let _ = fs::remove_dir_all(tmp_dir);
    }
}
