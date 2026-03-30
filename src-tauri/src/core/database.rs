use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
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
    pub last_modified: Option<String>,   //DATETIME
    pub last_hash_check: Option<String>, //DATETIME
    pub has_extracted_text: bool,
    pub extracted_at: Option<String>, //DATETIME
    pub processed_at: Option<String>, //DATETIME
    pub processed: bool,
    pub processing_priority: i32, // 0=new, 1=modified, 2=extracted, 3=rerun
    pub retry_count: i32,
    pub extraction_quality: Option<f64>, // 0.0-1.0
    pub created_at: Option<String>,      //DATETIME
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntelligenceEntry {
    pub id: i64,
    pub registry_id: i64,
    pub fingerprint: String,
    pub filename: String,
    pub source_quote: String,
    pub page_number: Option<i32>,
    pub evidence_full: Option<String>,
    pub evidence_hash: Option<String>,
    pub associated_date: Option<String>,
    pub fact_summary: String,
    pub category: Option<String>,
    pub identified_crime: Option<String>,
    pub severity_score: i32,
    pub confidence: Option<f64>,
    pub quality_score: Option<f64>,
    pub source_language: Option<String>,
    pub translated_quote: Option<String>,
    pub pipeline_id: Option<String>,
    pub pass_name: Option<String>,
    pub is_deleted: bool,
    pub deleted_at: Option<String>,
    pub processing_time_ms: Option<i64>,
    pub created_at: Option<String>,
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

        // FTS5 for facts full-text search
        intel_conn.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS facts_fts USING fts5(
                fact_summary,
                source_quote,
                category,
                content='intelligence',
                content_rowid='id'
            )",
            [],
        )?;

        // FTS5 for entities full-text search
        intel_conn.execute(
            "CREATE VIRTUAL TABLE IF NOT EXISTS entities_fts USING fts5(
                value,
                normalized_value,
                entity_type,
                content='entities',
                content_rowid='id'
            )",
            [],
        )?;

        // Entity aliases table for entity resolution
        intel_conn.execute(
            "CREATE TABLE IF NOT EXISTS entity_aliases (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                canonical_entity_id INTEGER NOT NULL,
                alias_value TEXT NOT NULL,
                alias_type TEXT NOT NULL,
                confidence REAL DEFAULT 1.0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (canonical_entity_id) REFERENCES entities(id)
            )",
            [],
        )?;

        intel_conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_aliases_canonical ON entity_aliases(canonical_entity_id)",
            [],
        )?;
        intel_conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_aliases_value ON entity_aliases(alias_value)",
            [],
        )?;

        // Evidence chains table
        intel_conn.execute(
            "CREATE TABLE IF NOT EXISTS evidence_chains (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                chain_name TEXT NOT NULL,
                chain_type TEXT NOT NULL,
                description TEXT,
                created_by TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        intel_conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_chains_name ON evidence_chains(chain_name)",
            [],
        )?;

        // Evidence chain links table
        intel_conn.execute(
            "CREATE TABLE IF NOT EXISTS evidence_chain_links (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                chain_id INTEGER NOT NULL,
                intelligence_id INTEGER NOT NULL,
                relationship_type TEXT NOT NULL,
                relationship_strength REAL DEFAULT 1.0,
                notes TEXT,
                linked_by TEXT,
                linked_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (chain_id) REFERENCES evidence_chains(id),
                FOREIGN KEY (intelligence_id) REFERENCES intelligence(id)
            )",
            [],
        )?;

        intel_conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_chain_links_chain ON evidence_chain_links(chain_id)",
            [],
        )?;
        intel_conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_chain_links_intel ON evidence_chain_links(intelligence_id)",
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
    pub fn update_registry_entry(
        &self,
        fingerprint: &str,
        has_text: bool,
        processed: bool,
        priority: i32,
        quality: Option<f64>,
    ) -> Result<()> {
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
    pub fn scan_for_changes(
        &self,
        evidence_root: &str,
    ) -> std::result::Result<Vec<(String, i32)>, Box<dyn std::error::Error>> {
        use std::fs::{self, metadata};
        use std::time::SystemTime;

        let conn = self.registry_conn.lock().unwrap();
        let mut changes = Vec::new();

        // Get existing fingerprints
        let existing: std::collections::HashSet<String> = conn
            .prepare("SELECT fingerprint FROM registry")?
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
            let file_name = path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string();
            let file_type = path
                .extension()
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
                let mut stmt = conn
                    .prepare_cached("SELECT last_modified FROM registry WHERE fingerprint = ?1")?;
                let last_registered: Option<i64> =
                    stmt.query_row(params![&fingerprint], |row| row.get(0))?;

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
    fn hash_file(&self, path: &Path) -> std::io::Result<String> {
        use std::fs::File;
        use std::io::Read;

        let metadata = std::fs::metadata(path)?;
        let mut file = File::open(path)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;

        // Simple hash using std's default hasher
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        metadata.len().hash(&mut hasher);
        metadata.modified()?.hash(&mut hasher);
        buffer.hash(&mut hasher);
        let hash = hasher.finish();

        Ok(format!("{:016x}-{}", hash, metadata.len()))
    }

    pub fn get_unprocessed_files(&self, limit: i64) -> Result<Vec<RegistryEntry>> {
        let conn = self.registry_conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, fingerprint, path, file_size, file_type, file_name,
                    last_modified, last_hash_check, has_extracted_text,
                    extracted_at, processed_at, processed,
                    processing_priority, retry_count, extraction_quality, created_at
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

    pub fn insert_intelligence(&self, entry: &IntelligenceEntry) -> Result<()> {
        let conn = self.intelligence_conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO intelligence
             (registry_id, fingerprint, filename, source_quote, page_number, evidence_full, evidence_hash,
              associated_date, fact_summary, category, identified_crime, severity_score,
              confidence, quality_score, source_language, translated_quote, pipeline_id, pass_name,
              is_deleted, deleted_at, processing_time_ms, created_at)
              VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22)",
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
    pub fn add_error(
        &self,
        fingerprint: &str,
        job_type: &str,
        error_message: &str,
        error_details: &str,
    ) -> Result<()> {
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

    pub fn update_error(
        &self,
        error_id: i64,
        retry_count: i32,
        error_message: &str,
        next_attempt: Option<String>,
    ) -> Result<()> {
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

    // Search methods using FTS5 with Boolean and phrase support
    pub fn search_facts(&self, query: &str, limit: i64) -> Result<Vec<SearchResult>> {
        self.search_facts_with_filters(query, limit, None, None, None, None)
    }

    pub fn search_facts_with_filters(
        &self,
        query: &str,
        limit: i64,
        categories: Option<&[String]>,
        min_severity: Option<i32>,
        start_date: Option<&str>,
        end_date: Option<&str>,
    ) -> Result<Vec<SearchResult>> {
        let conn = self.intelligence_conn.lock().unwrap();
        
        let fts_query = Self::parse_search_query(query);
        
        let mut sql = String::from(
            "SELECT i.id, i.fingerprint, i.filename, i.fact_summary, i.category, i.severity_score, i.confidence, i.created_at,
                    bm25(facts_fts) as rank
             FROM facts_fts f
             JOIN intelligence i ON f.rowid = i.id
             WHERE facts_fts MATCH ?1"
        );
        
        let mut conditions = Vec::new();
        let mut param_idx = 2;
        
        if let Some(cats) = categories {
            if !cats.is_empty() {
                let placeholders: Vec<String> = cats.iter().map(|_| format!("?{}", param_idx)).collect();
                conditions.push(format!("category IN ({})", placeholders.join(",")));
                param_idx += cats.len() as i32;
            }
        }
        
        if let Some(severity) = min_severity {
            conditions.push(format!("severity_score >= ?{}", param_idx));
            param_idx += 1;
        }
        
        if let Some(start) = start_date {
            conditions.push(format!("associated_date >= ?{}", param_idx));
            param_idx += 1;
        }
        
        if let Some(end) = end_date {
            conditions.push(format!("associated_date <= ?{}", param_idx));
        }
        
        if !conditions.is_empty() {
            sql.push_str(" AND ");
            sql.push_str(&conditions.join(" AND "));
        }
        
        sql.push_str(" ORDER BY rank LIMIT ?");
        
        let mut stmt = conn.prepare(&sql)?;
        
        let mut params_vec: Vec<Box<dyn rusqlite::ToSql>> = vec![Box::new(fts_query.clone()), Box::new(limit)];
        let mut param_refs: Vec<&dyn rusqlite::ToSql> = vec![&*params_vec[0], &*params_vec[1]];
        
        if let Some(cats) = categories {
            for cat in cats {
                params_vec.push(Box::new(cat.clone()));
                param_refs.push(&*params_vec.last().unwrap());
            }
        }
        
        if let Some(severity) = min_severity {
            params_vec.push(Box::new(severity));
            param_refs.push(&*params_vec.last().unwrap());
        }
        
        if let Some(start) = start_date {
            params_vec.push(Box::new(start.to_string()));
            param_refs.push(&*params_vec.last().unwrap());
        }
        
        if let Some(end) = end_date {
            params_vec.push(Box::new(end.to_string()));
            param_refs.push(&*params_vec.last().unwrap());
        }
        
        let entries = stmt.query_map(rusqlite::params_from_iter(param_refs.iter()), |row| {
            Ok(SearchResult {
                id: row.get(0)?,
                fingerprint: row.get(1)?,
                filename: row.get(2)?,
                summary: row.get(3)?,
                category: row.get(4)?,
                severity: row.get(5)?,
                confidence: row.get(6)?,
                rank: row.get(7)?,
                result_type: "fact".to_string(),
            })
        })?;

        entries.collect()
    }

    pub fn search_entities(&self, query: &str, limit: i64) -> Result<Vec<EntitySearchResult>> {
        self.search_entities_with_filters(query, limit, None, None)
    }

    pub fn search_entities_with_filters(
        &self,
        query: &str,
        limit: i64,
        entity_types: Option<&[String]>,
        min_confidence: Option<f64>,
    ) -> Result<Vec<EntitySearchResult>> {
        let conn = self.intelligence_conn.lock().unwrap();
        
        let fts_query = Self::parse_search_query(query);
        
        let mut sql = String::from(
            "SELECT e.id, e.fingerprint, e.entity_type, e.value, e.normalized_value, e.confidence,
                    i.filename, bm25(entities_fts) as rank
             FROM entities_fts f
             JOIN entities e ON f.rowid = e.id
             JOIN intelligence i ON e.fingerprint = i.fingerprint
             WHERE entities_fts MATCH ?1"
        );
        
        let mut conditions = Vec::new();
        let mut param_idx = 2;
        
        if let Some(types) = entity_types {
            if !types.is_empty() {
                let placeholders: Vec<String> = types.iter().map(|_| format!("?{}", param_idx)).collect();
                conditions.push(format!("e.entity_type IN ({})", placeholders.join(",")));
                param_idx += types.len() as i32;
            }
        }
        
        if let Some(conf) = min_confidence {
            conditions.push(format!("e.confidence >= ?{}", param_idx));
        }
        
        if !conditions.is_empty() {
            sql.push_str(" AND ");
            sql.push_str(&conditions.join(" AND "));
        }
        
        sql.push_str(" ORDER BY rank LIMIT ?");
        
        let mut stmt = conn.prepare(&sql)?;
        
        let param_refs: Vec<&dyn rusqlite::ToSql> = if let Some(types) = entity_types {
            if let Some(conf) = min_confidence {
                let mut params: Vec<&dyn rusqlite::ToSql> = vec![&fts_query, &limit];
                for t in types {
                    params.push(t);
                }
                params.push(&conf);
                params
            } else {
                let mut params: Vec<&dyn rusqlite::ToSql> = vec![&fts_query, &limit];
                for t in types {
                    params.push(t);
                }
                params
            }
        } else {
            vec![&fts_query, &limit]
        };
        
        let entries = stmt.query_map(rusqlite::params_from_iter(param_refs.iter()), |row| {
            Ok(EntitySearchResult {
                id: row.get(0)?,
                fingerprint: row.get(1)?,
                entity_type: row.get(2)?,
                value: row.get(3)?,
                normalized_value: row.get(4)?,
                confidence: row.get(5)?,
                source_file: row.get(6)?,
                rank: row.get(7)?,
            })
        })?;

        entries.collect()
    }

    fn parse_search_query(input: &str) -> String {
        let mut result = String::new();
        let mut in_phrase = false;
        let mut chars = input.chars().peekable();
        
        while let Some(c) = chars.next() {
            if c == '"' {
                if in_phrase {
                    result.push('"');
                    in_phrase = false;
                } else {
                    result.push('"');
                    in_phrase = true;
                }
            } else if c == ' ' && !in_phrase {
                if !result.ends_with(' ') {
                    result.push(' ');
                }
            } else if c == '(' || c == ')' {
                result.push(c);
            } else if c.to_ascii_uppercase() == 'A' && result.ends_with(' ') {
                if chars.clone().take(2).collect::<String>() == "ND" {
                    result.push_str("AND ");
                    chars.next();
                    chars.next();
                    continue;
                }
                result.push(c);
            } else if c.to_ascii_uppercase() == 'O' && result.ends_with(' ') {
                if chars.clone().take(1).collect::<String>() == "R" {
                    result.push_str("OR ");
                    chars.next();
                    continue;
                }
                result.push(c);
            } else if c.to_ascii_uppercase() == 'N' && result.ends_with(' ') {
                if chars.clone().take(2).collect::<String>() == "OT" {
                    result.push_str("NOT ");
                    chars.next();
                    chars.next();
                    continue;
                }
                result.push(c);
            } else {
                result.push(c);
            }
        }
        
        result.trim().to_string()
    }

    pub fn search_combined(&self, query: &str, limit: i64) -> Result<Vec<CombinedSearchResult>> {
        let facts = self.search_facts(query, limit)?;
        let entities = self.search_entities(query, limit)?;

        let mut combined: Vec<CombinedSearchResult> = facts
            .into_iter()
            .map(|f| CombinedSearchResult {
                id: f.id,
                result_type: f.result_type,
                fingerprint: f.fingerprint,
                filename: f.filename,
                title: f.summary.clone(),
                description: Some(f.summary),
                category: f.category,
                severity: f.severity,
                confidence: f.confidence,
                rank: f.rank,
            })
            .collect();

        for e in entities {
            combined.push(CombinedSearchResult {
                id: e.id,
                result_type: "entity".to_string(),
                fingerprint: e.fingerprint,
                filename: e.source_file,
                title: e.value.clone(),
                description: e.normalized_value,
                category: Some(e.entity_type),
                severity: None,
                confidence: e.confidence,
                rank: e.rank,
            });
        }

        combined.sort_by(|a, b| a.rank.partial_cmp(&b.rank).unwrap_or(std::cmp::Ordering::Equal));
        combined.truncate(limit as usize);
        
        Ok(combined)
    }

    // Entity alias methods for entity resolution
    pub fn add_entity_alias(&self, canonical_id: i64, alias: &str, alias_type: &str, confidence: f64) -> Result<()> {
        let conn = self.intelligence_conn.lock().unwrap();
        conn.execute(
            "INSERT INTO entity_aliases (canonical_entity_id, alias_value, alias_type, confidence) VALUES (?1, ?2, ?3, ?4)",
            params![canonical_id, alias, alias_type, confidence],
        )?;
        Ok(())
    }

    pub fn resolve_entity(&self, alias: &str) -> Result<Vec<ResolvedEntity>> {
        let conn = self.intelligence_conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT e.id, e.entity_type, e.value, e.normalized_value, e.fingerprint, a.confidence
             FROM entity_aliases a
             JOIN entities e ON a.canonical_entity_id = e.id
             WHERE a.alias_value = ?1
             ORDER BY a.confidence DESC"
        )?;

        let entries = stmt.query_map(params![alias], |row| {
            Ok(ResolvedEntity {
                entity_id: row.get(0)?,
                entity_type: row.get(1)?,
                value: row.get(2)?,
                normalized_value: row.get(3)?,
                fingerprint: row.get(4)?,
                confidence: row.get(5)?,
            })
        })?;

        entries.collect()
    }

    // Evidence chain methods
    pub fn create_chain(&self, name: &str, chain_type: &str, description: &str, created_by: &str) -> Result<i64> {
        let conn = self.intelligence_conn.lock().unwrap();
        conn.execute(
            "INSERT INTO evidence_chains (chain_name, chain_type, description, created_by) VALUES (?1, ?2, ?3, ?4)",
            params![name, chain_type, description, created_by],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn add_to_chain(&self, chain_id: i64, intelligence_id: i64, relationship_type: &str, 
                       strength: f64, notes: &str, linked_by: &str) -> Result<()> {
        let conn = self.intelligence_conn.lock().unwrap();
        conn.execute(
            "INSERT INTO evidence_chain_links (chain_id, intelligence_id, relationship_type, relationship_strength, notes, linked_by)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![chain_id, intelligence_id, relationship_type, strength, notes, linked_by],
        )?;
        Ok(())
    }

    pub fn get_chain(&self, chain_id: i64) -> Result<Option<EvidenceChain>> {
        let conn = self.intelligence_conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, chain_name, chain_type, description, created_by, created_at, updated_at
             FROM evidence_chains WHERE id = ?1"
        )?;

        let mut rows = stmt.query(params![chain_id])?;
        if let Some(row) = rows.next()? {
            Ok(Some(EvidenceChain {
                id: row.get(0)?,
                chain_name: row.get(1)?,
                chain_type: row.get(2)?,
                description: row.get(3)?,
                created_by: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
                items: Vec::new(),
            }))
        } else {
            Ok(None)
        }
    }

    pub fn get_chain_items(&self, chain_id: i64) -> Result<Vec<ChainItem>> {
        let conn = self.intelligence_conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT l.id, l.intelligence_id, l.relationship_type, l.relationship_strength, l.notes, l.linked_by, l.linked_at,
                    i.filename, i.fact_summary, i.category
             FROM evidence_chain_links l
             JOIN intelligence i ON l.intelligence_id = i.id
             WHERE l.chain_id = ?1
             ORDER BY l.linked_at DESC"
        )?;

        let entries = stmt.query_map(params![chain_id], |row| {
            Ok(ChainItem {
                link_id: row.get(0)?,
                intelligence_id: row.get(1)?,
                relationship_type: row.get(2)?,
                relationship_strength: row.get(3)?,
                notes: row.get(4)?,
                linked_by: row.get(5)?,
                linked_at: row.get(6)?,
                filename: row.get(7)?,
                fact_summary: row.get(8)?,
                category: row.get(9)?,
            })
        })?;

        entries.collect()
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: i64,
    pub fingerprint: String,
    pub filename: String,
    pub summary: String,
    pub category: Option<String>,
    pub severity: i32,
    pub confidence: Option<f64>,
    pub rank: f64,
    pub result_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntitySearchResult {
    pub id: i64,
    pub fingerprint: String,
    pub entity_type: String,
    pub value: String,
    pub normalized_value: Option<String>,
    pub confidence: Option<f64>,
    pub source_file: String,
    pub rank: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombinedSearchResult {
    pub id: i64,
    pub result_type: String,
    pub fingerprint: String,
    pub filename: String,
    pub title: String,
    pub description: Option<String>,
    pub category: Option<String>,
    pub severity: Option<i32>,
    pub confidence: Option<f64>,
    pub rank: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedEntity {
    pub entity_id: i64,
    pub entity_type: String,
    pub value: String,
    pub normalized_value: Option<String>,
    pub fingerprint: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceChain {
    pub id: i64,
    pub chain_name: String,
    pub chain_type: String,
    pub description: Option<String>,
    pub created_by: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub items: Vec<ChainItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainItem {
    pub link_id: i64,
    pub intelligence_id: i64,
    pub relationship_type: String,
    pub relationship_strength: f64,
    pub notes: Option<String>,
    pub linked_by: Option<String>,
    pub linked_at: Option<String>,
    pub filename: String,
    pub fact_summary: String,
    pub category: Option<String>,
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
