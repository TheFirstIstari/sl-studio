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
    pub file_name: String,
    pub last_modified: Option<String>, // DATETIME
    pub last_hash_check: Option<String>, // DATETIME
    pub has_extracted_text: bool,
    pub extracted_at: Option<String>, // DATETIME
    pub processed_at: Option<String>, // DATETIME
    pub processed: bool,
    pub processing_priority: i32, // 0=new, 1=modified, 2=extracted, 3=rerun
    pub retry_count: i32,
    pub extraction_quality: Option<f64>, // 0.0-1.0
    pub created_at: Option<String>, // DATETIME
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntelligenceEntry {
    pub id: i64,
    pub registry_id: i64,
    pub fingerprint: String,
    pub filename: String,
    pub source_quote: String, // Required, exact supporting quote
    pub page_number: Option<i32>,
    pub evidence_full: Option<String>,
    pub evidence_hash: Option<String>,
    pub associated_date: Option<String>,
    pub fact_summary: String,
    pub category: Option<String>,
    pub identified_crime: Option<String>,
    pub severity_score: i32,
    pub confidence: Option<f64>,
    pub quality_score: Option<f64>, // Overall extraction quality 0.0-1.0
    pub source_language: Option<String>, // ISO 639-1 language code
    pub translated_quote: Option<String>,
    pub pipeline_id: Option<String>,
    pub pass_name: Option<String>,
    pub is_deleted: bool, // Soft delete for forensic integrity
    pub deleted_at: Option<String>, // DATETIME
    pub processing_time_ms: Option<i64>,
    pub created_at: Option<String>, // DATETIME
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
                last_modified DATETIME,
                last_hash_check DATETIME,
                has_extracted_text BOOLEAN DEFAULT FALSE,
                extracted_at DATETIME,
                processed_at DATETIME,
                processed INTEGER DEFAULT 0,
                processing_priority INTEGER DEFAULT 0,
                retry_count INTEGER DEFAULT 0,
                extraction_quality REAL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
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
        reg_conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_registry_priority ON registry(processing_priority)",
            [],
        )?;
        reg_conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_registry_modified ON registry(last_modified)",
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
                source_quote TEXT NOT NULL,
                page_number INTEGER,
                evidence_full TEXT,
                evidence_hash TEXT,
                associated_date TEXT,
                fact_summary TEXT NOT NULL,
                category TEXT,
                identified_crime TEXT,
                severity_score INTEGER DEFAULT 1,
                confidence REAL,
                quality_score REAL,
                source_language TEXT,
                translated_quote TEXT,
                pipeline_id TEXT,
                pass_name TEXT,
                is_deleted BOOLEAN DEFAULT FALSE,
                deleted_at DATETIME,
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
        intel_conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_intelligence_severity ON intelligence(severity_score DESC)",
            [],
        )?;
        intel_conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_intelligence_quality ON intelligence(quality_score)",
            [],
        )?;
        intel_conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_intelligence_source_language ON intelligence(source_language)",
            [],
        )?;
        intel_conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_intelligence_pipeline ON intelligence(pipeline_id, pass_name)",
            [],
        )?;
        intel_conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_intelligence_deleted ON intelligence(is_deleted) WHERE is_deleted = FALSE",
            [],
        )?;

        // Composite index for uniqueness and efficient lookups
        intel_conn.execute(
            "CREATE UNIQUE INDEX IF NOT EXISTS idx_intelligence_unique ON intelligence(fingerprint, filename, fact_summary)",
            []
        )?;

        // Entities table (Named Entity Recognition)
        intel_conn.execute(
            "CREATE TABLE IF NOT EXISTS entities (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                intelligence_id INTEGER,
                fingerprint TEXT NOT NULL,
                entity_type TEXT NOT NULL,
                value TEXT NOT NULL,
                normalized_value TEXT,
                confidence REAL,
                position_start INTEGER,
                position_end INTEGER,
                pipeline_id TEXT,
                pass_name TEXT,
                is_deleted BOOLEAN DEFAULT FALSE,
                deleted_at DATETIME,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        intel_conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_entities_fingerprint ON entities(fingerprint)",
            [],
        )?;
        intel_conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_entities_type ON entities(entity_type)",
            [],
        )?;
        intel_conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_entities_value ON entities(value)",
            [],
        )?;
        intel_conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_entities_pipeline ON entities(pipeline_id, pass_name)",
            [],
        )?;
        intel_conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_entities_deleted ON entities(is_deleted) WHERE is_deleted = FALSE",
            [],
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

        // Error queue for retry logic
        intel_conn.execute(
            "CREATE TABLE IF NOT EXISTS error_queue (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                fingerprint TEXT NOT NULL,
                job_type TEXT NOT NULL,
                error_message TEXT NOT NULL,
                error_details TEXT,
                retry_count INTEGER DEFAULT 0,
                max_retries INTEGER DEFAULT 3,
                last_attempt DATETIME,
                next_attempt DATETIME,
                resolved BOOLEAN DEFAULT FALSE,
                resolution TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        intel_conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_error_fingerprint ON error_queue(fingerprint)",
            [],
        )?;
        intel_conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_error_pending ON error_queue(resolved, next_attempt)",
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
            "INSERT OR IGNORE INTO registry (fingerprint, path, file_type, file_size, file_name, last_modified, last_hash_check) VALUES (?1, ?2, ?3, ?4, ?5, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)",
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

    /// Update a file's registry entry after processing
    pub fn update_registry_entry(&self, fingerprint: &str, has_text: bool, processed: bool, 
                                priority: i32, quality: Option<f64>) -> Result<()> {
        let conn = self.registry_conn.lock().unwrap();
        conn.execute(
            "UPDATE registry SET 
                has_extracted_text = ?2,
                processed = ?3,
                processing_priority = ?4,
                extraction_quality = ?5,
                extracted_at = CASE WHEN ?2 THEN CURRENT_TIMESTAMP ELSE extracted_at END,
                processed_at = CASE WHEN ?3 THEN CURRENT_TIMESTAMP ELSE processed_at END
             WHERE fingerprint = ?1",
            params![fingerprint, has_text, processed, priority, quality],
        )?;
        Ok(())
    }

    /// Get files ordered by processing priority for incremental processing
    pub fn get_priority_queue(&self, limit: i64) -> Result<Vec<RegistryEntry>> {
        let conn = self.registry_conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, fingerprint, path, file_size, file_type, file_name, 
                    last_modified, last_hash_check, has_extracted_text, 
                    extracted_at, processed_at, processed, 
                    processing_priority, retry_count, extraction_quality, created_at
             FROM registry
             ORDER BY processing_priority ASC, last_modified DESC
             LIMIT ?1",
        )?;

        let entries = stmt.query_map([limit], |row| {
            Ok(RegistryEntry {
                id: row.get(0)?,
                fingerprint: row.get(1)?,
                path: row.get(2)?,
                file_size: row.get(3)?,
                file_type: row.get(4)?,
                file_name: row.get(5)?,
                last_modified: row.get(6)?,
                last_hash_check: row.get(7)?,
                has_extracted_text: row.get(8)?,
                extracted_at: row.get(9)?,
                processed_at: row.get(10)?,
                processed: row.get(11)?,
                processing_priority: row.get(12)?,
                retry_count: row.get(13)?,
                extraction_quality: row.get(14)?,
                created_at: row.get(15)?,
            })
        })?;

        entries.collect()
    }

    /// Scan for new or modified files and update registry
    pub fn scan_for_changes(&self, evidence_root: &str) -> Result<Vec<(String, i32)>> {
        use std::fs::{self, metadata};
        use std::path::Path;
        use std::time::SystemTime;

        let conn = self.registry_conn.lock().unwrap();
        let mut changes = Vec::new();

        // Get existing fingerprints
        let existing: std::collections::HashSet<String> = 
            conn.prepare("SELECT fingerprint FROM registry")?
                .query_map([], |row| row.get(0))?
                .flatten()
                .collect();

        // Walk the evidence root
        for entry in fs::read_dir(evidence_root)? {
            let entry = entry?;
            let path = entry.path();
            
            if !path.is_file() {
                continue;
            }

            let fingerprint = self.hash_file(&path)?;
            let file_name = path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();
            let file_type = path.extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("unknown")
                .to_string();
            let file_size = metadata(&path)?.len() as i64;
            let last_modified = metadata(&path)?
                .modified()?
                .duration_since(SystemTime::UNIX_EPOCH)?
                .as_secs() as i64;

            // Determine processing priority
            let priority = if !existing.contains(&fingerprint) {
                // New file
                0
            } else {
                // Check if file has been modified
                let mut stmt = conn.prepare_cached(
                    "SELECT last_modified FROM registry WHERE fingerprint = ?1"
                )?;
                let last_registered: Option<i64> = stmt.query_row(params![&fingerprint], |row| row.get(0))?;
                
                if let Some(last_mod) = last_registered {
                    if last_modified > last_mod {
                        // Modified file
                        1
                    } else {
                        // Check if we have extracted text but not processed
                        let mut stmt = conn.prepare_cached(
                            "SELECT has_extracted_text, processed FROM registry WHERE fingerprint = ?1"
                        )?;
                        let result = stmt.query_row(params![&fingerprint], |row| {
                            Ok((row.get::<_, bool>(0)?, row.get::<_, bool>(1)?))
                        })?;
                        
                        match result {
                            (true, false) => {
                                // Extracted but not processed
                                2
                            }
                            _ => {
                                // Already processed, may rerun for accuracy
                                3
                            }
                        }
                    }
                } else {
                    // Shouldn't happen, but treat as new
                    0
                }
            };

            // Update or insert the registry entry
            conn.execute(
                "INSERT OR REPLACE INTO registry 
                 (fingerprint, path, file_type, file_size, file_name, 
                  last_modified, last_hash_check, processing_priority)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    &fingerprint,
                    path.to_string_lossy(),
                    &file_type,
                    file_size,
                    &file_name,
                    last_modified,
                    last_modified, // last_hash_check same as last_modified for now
                    priority
                ],
            )?;

            changes.push((fingerprint, priority));
        }

        Ok(changes)
    }

    /// Hash a file for change detection
    fn hash_file(&self, path: &Path) -> Result<String> {
        use std::fs::File;
        use std::io::{self, Read};
        use std::hash::{Hash, Hasher};
        use twox_hash::XxHash64;

        let mut file = File::open(path)?;
        let mut hasher = XxHash64::default();
        let mut buffer = [0; 8192];

        loop {
            let count = file.read(&mut buffer)?;
            if count == 0 {
                break;
            }
            hasher.write(&buffer[..count]);
        }

        Ok(format!("{:x}", hasher.finish()))
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
             (registry_id, fingerprint, filename, source_quote, page_number, evidence_full, evidence_hash,
              associated_date, fact_summary, category, identified_crime, severity_score, 
              confidence, quality_score, source_language, translated_quote, pipeline_id, pass_name,
              is_deleted, deleted_at, processing_time_ms, created_at)
              VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23)",
            params![
                entry.registry_id,
                entry.fingerprint,
                entry.filename,
                entry.source_quote,
                entry.page_number,
                entry.evidence_full,
                entry.evidence_hash,
                entry.associated_date,
                entry.fact_summary,
                entry.category,
                entry.identified_crime,
                entry.severity_score,
                entry.confidence,
                entry.quality_score,
                entry.source_language,
                entry.translated_quote,
                entry.pipeline_id,
                entry.pass_name,
                entry.is_deleted,
                entry.deleted_at,
                entry.processing_time_ms,
                entry.created_at
            ],
        )?;
        Ok(())
    }

    pub fn get_intelligence(&self, limit: i64, offset: i64) -> Result<Vec<IntelligenceEntry>> {
        let conn = self.intelligence_conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, registry_id, fingerprint, filename, source_quote, page_number, evidence_full, evidence_hash,
                    associated_date, fact_summary, category, identified_crime, severity_score, confidence, quality_score,
                    source_language, translated_quote, pipeline_id, pass_name, is_deleted, deleted_at, processing_time_ms, created_at
             FROM intelligence
             WHERE is_deleted = FALSE
             ORDER BY severity_score DESC, created_at DESC
             LIMIT ?1 OFFSET ?2",
        )?;

        let entries = stmt.query_map(params![limit, offset], |row| {
            Ok(IntelligenceEntry {
                id: row.get(0)?,
                registry_id: row.get(1)?,
                fingerprint: row.get(2)?,
                filename: row.get(3)?,
                source_quote: row.get(4)?,
                page_number: row.get(5)?,
                evidence_full: row.get(6)?,
                evidence_hash: row.get(7)?,
                associated_date: row.get(8)?,
                fact_summary: row.get(9)?,
                category: row.get(10)?,
                identified_crime: row.get(11)?,
                severity_score: row.get(12)?,
                confidence: row.get(13)?,
                quality_score: row.get(14)?,
                source_language: row.get(15)?,
                translated_quote: row.get(16)?,
                pipeline_id: row.get(17)?,
                pass_name: row.get(18)?,
                is_deleted: row.get(19)?,
                deleted_at: row.get(20)?,
                processing_time_ms: row.get(21)?,
                created_at: row.get(22)?,
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

    // Error queue operations
    pub fn add_error(&self, fingerprint: &str, job_type: &str, error_message: &str, error_details: &str) -> Result<()> {
        let conn = self.intelligence_conn.lock().unwrap();
        conn.execute(
            "INSERT INTO error_queue (fingerprint, job_type, error_message, error_details, next_attempt)
             VALUES (?1, ?2, ?3, ?4, datetime('now'))",
            params![fingerprint, job_type, error_message, error_details],
        )?;
        Ok(())
    }

    pub fn get_pending_errors(&self, limit: i64) -> Result<Vec<ErrorQueueEntry>> {
        let conn = self.intelligence_conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, fingerprint, job_type, error_message, error_details, retry_count, max_retries, last_attempt, next_attempt, resolved, resolution, created_at
             FROM error_queue
             WHERE resolved = 0 AND datetime(next_attempt) <= datetime('now')
             ORDER BY next_attempt ASC
             LIMIT ?1",
        )?;

        let entries = stmt.query_map([limit], |row| {
            Ok(ErrorQueueEntry {
                id: row.get(0)?,
                fingerprint: row.get(1)?,
                job_type: row.get(2)?,
                error_message: row.get(3)?,
                error_details: row.get(4)?,
                retry_count: row.get(5)?,
                max_retries: row.get(6)?,
                last_attempt: row.get(7)?,
                next_attempt: row.get(8)?,
                resolved: row.get(9)?,
                resolution: row.get(10)?,
                created_at: row.get(11)?,
            })
        })?;

        entries.collect()
    }

    pub fn update_error(&self, error_id: i64, retry_count: i32, error_message: &str, next_attempt: Option<String>) -> Result<()> {
        let conn = self.intelligence_conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "UPDATE error_queue SET 
                 retry_count = ?2,
                 error_message = ?3,
                 last_attempt = CURRENT_TIMESTAMP,
                 next_attempt = COALESCE(?4, datetime('now', '+' || (retry_count * 2) || ' minutes')),
                 updated_at = CURRENT_TIMESTAMP
             WHERE id = ?1",
        )?;
        stmt.execute(params![error_id, retry_count, error_message, next_attempt])?;
        Ok(())
    }

    pub fn resolve_error(&self, error_id: i64, resolution: &str, resolved_by: &str) -> Result<()> {
        let conn = self.intelligence_conn.lock().unwrap();
        conn.execute(
            "UPDATE error_queue SET 
                 resolved = 1,
                 resolution = ?2,
                 resolved_by = ?3,
                 updated_at = CURRENT_TIMESTAMP
             WHERE id = ?1",
            params![error_id, resolution, resolved_by],
        )?;
        Ok(())
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorQueueEntry {
    pub id: i64,
    pub fingerprint: String,
    pub job_type: String,
    pub error_message: String,
    pub error_details: String,
    pub retry_count: i32,
    pub max_retries: i32,
    pub last_attempt: Option<String>,
    pub next_attempt: Option<String>,
    pub resolved: bool,
    pub resolution: Option<String>,
    pub created_at: Option<String>,
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
