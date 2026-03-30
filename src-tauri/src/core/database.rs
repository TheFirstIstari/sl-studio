use rusqlite::{params, Connection, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tracing::info;

struct CacheEntry<T: Clone> {
    data: T,
    expires_at: Instant,
}

impl<T: Clone> CacheEntry<T> {
    fn new(data: T, ttl: Duration) -> Self {
        Self {
            data,
            expires_at: Instant::now() + ttl,
        }
    }

    fn is_valid(&self) -> bool {
        Instant::now() < self.expires_at
    }
}

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
    category_cache: Mutex<Option<CacheEntry<Vec<CategoryStats>>>>,
    severity_cache: Mutex<Option<CacheEntry<Vec<SeverityStats>>>>,
    overall_stats_cache: Mutex<Option<CacheEntry<OverallStatistics>>>,
}

impl Database {
    pub fn new(registry_path: &str, intelligence_path: &str) -> Result<Self> {
        let reg_conn = Connection::open(registry_path)?;
        let intel_conn = Connection::open(intelligence_path)?;

        let db = Database {
            registry_conn: Mutex::new(reg_conn),
            intelligence_conn: Mutex::new(intel_conn),
            category_cache: Mutex::new(None),
            severity_cache: Mutex::new(None),
            overall_stats_cache: Mutex::new(None),
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
                tags TEXT,
                is_deleted BOOLEAN DEFAULT FALSE,
                deleted_at DATETIME,
                processing_time_ms INTEGER,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        // Annotations table
        intel_conn.execute(
            "CREATE TABLE IF NOT EXISTS annotations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                intelligence_id INTEGER NOT NULL,
                content TEXT NOT NULL,
                annotation_type TEXT DEFAULT 'general',
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (intelligence_id) REFERENCES intelligence(id)
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

    // Cache invalidation helper
    pub fn invalidate_cache(&self) {
        *self.category_cache.lock().unwrap() = None;
        *self.severity_cache.lock().unwrap() = None;
        *self.overall_stats_cache.lock().unwrap() = None;
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
        
        // Invalidate cache since data changed
        self.invalidate_cache();
        
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
                let placeholders: Vec<String> =
                    cats.iter().map(|_| format!("?{}", param_idx)).collect();
                conditions.push(format!("category IN ({})", placeholders.join(",")));
                param_idx += cats.len() as i32;
            }
        }

        if min_severity.is_some() {
            conditions.push(format!("severity_score >= ?{}", param_idx));
            param_idx += 1;
        }

        if start_date.is_some() {
            conditions.push(format!("associated_date >= ?{}", param_idx));
            param_idx += 1;
        }

        if end_date.is_some() {
            conditions.push(format!("associated_date <= ?{}", param_idx));
        }

        if !conditions.is_empty() {
            sql.push_str(" AND ");
            sql.push_str(&conditions.join(" AND "));
        }

        sql.push_str(" ORDER BY rank LIMIT ?");

        let mut stmt = conn.prepare(&sql)?;

        let mut params: Vec<Box<dyn rusqlite::ToSql>> =
            vec![Box::new(fts_query.clone()), Box::new(limit)];

        if let Some(cats) = categories {
            for cat in cats {
                params.push(Box::new(cat.clone()));
            }
        }

        if let Some(severity) = min_severity {
            params.push(Box::new(severity));
        }

        if let Some(start) = start_date {
            params.push(Box::new(start.to_string()));
        }

        if let Some(end) = end_date {
            params.push(Box::new(end.to_string()));
        }

        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

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
             WHERE entities_fts MATCH ?1",
        );

        let mut conditions = Vec::new();
        let mut param_idx = 2;

        if let Some(types) = entity_types {
            if !types.is_empty() {
                let placeholders: Vec<String> =
                    types.iter().map(|_| format!("?{}", param_idx)).collect();
                conditions.push(format!("e.entity_type IN ({})", placeholders.join(",")));
                param_idx += types.len() as i32;
            }
        }

        if min_confidence.is_some() {
            conditions.push(format!("e.confidence >= ?{}", param_idx));
        }

        if !conditions.is_empty() {
            sql.push_str(" AND ");
            sql.push_str(&conditions.join(" AND "));
        }

        sql.push_str(" ORDER BY rank LIMIT ?");

        let mut stmt = conn.prepare(&sql)?;

        let mut params: Vec<Box<dyn rusqlite::ToSql>> =
            vec![Box::new(fts_query.clone()), Box::new(limit)];

        if let Some(types) = entity_types {
            for t in types {
                params.push(Box::new(t.clone()));
            }
        }

        if let Some(confidence) = min_confidence {
            params.push(Box::new(confidence));
        }

        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

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
        let mut result = String::with_capacity(input.len() + 64);
        let mut in_phrase = false;
        let mut pos = 0;
        let chars: Vec<char> = input.chars().collect();
        
        while pos < chars.len() {
            let c = chars[pos];
            
            if c == '"' {
                result.push('"');
                in_phrase = !in_phrase;
                pos += 1;
            } else if c == ' ' && !in_phrase {
                if !result.ends_with(' ') {
                    result.push(' ');
                }
                pos += 1;
            } else if c == '(' || c == ')' {
                result.push(c);
                pos += 1;
            } else if c.eq_ignore_ascii_case(&'A') && result.ends_with(' ') && pos + 2 < chars.len() {
                let next = chars[pos + 1];
                let next2 = chars[pos + 2];
                if next.eq_ignore_ascii_case(&'N') && next2.eq_ignore_ascii_case(&'D') {
                    result.push_str("AND ");
                    pos += 3;
                    continue;
                }
                result.push(c);
                pos += 1;
            } else if c.eq_ignore_ascii_case(&'O') && result.ends_with(' ') && pos + 1 < chars.len() {
                let next = chars[pos + 1];
                if next.eq_ignore_ascii_case(&'R') {
                    result.push_str("OR ");
                    pos += 2;
                    continue;
                }
                result.push(c);
                pos += 1;
            } else if c.eq_ignore_ascii_case(&'N') && result.ends_with(' ') && pos + 2 < chars.len() {
                let next = chars[pos + 1];
                let next2 = chars[pos + 2];
                if next.eq_ignore_ascii_case(&'O') && next2.eq_ignore_ascii_case(&'T') {
                    result.push_str("NOT ");
                    pos += 3;
                    continue;
                }
                result.push(c);
                pos += 1;
            } else {
                result.push(c);
                pos += 1;
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
                severity: Some(f.severity),
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

        combined.sort_by(|a, b| {
            a.rank
                .partial_cmp(&b.rank)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        combined.truncate(limit as usize);

        Ok(combined)
    }

    // Entity alias methods for entity resolution
    pub fn add_entity_alias(
        &self,
        canonical_id: i64,
        alias: &str,
        alias_type: &str,
        confidence: f64,
    ) -> Result<()> {
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
             ORDER BY a.confidence DESC",
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
    pub fn create_chain(
        &self,
        name: &str,
        chain_type: &str,
        description: &str,
        created_by: &str,
    ) -> Result<i64> {
        let conn = self.intelligence_conn.lock().unwrap();
        conn.execute(
            "INSERT INTO evidence_chains (chain_name, chain_type, description, created_by) VALUES (?1, ?2, ?3, ?4)",
            params![name, chain_type, description, created_by],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn add_to_chain(
        &self,
        chain_id: i64,
        intelligence_id: i64,
        relationship_type: &str,
        strength: f64,
        notes: &str,
        linked_by: &str,
    ) -> Result<()> {
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
             FROM evidence_chains WHERE id = ?1",
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

    pub fn get_all_chains(&self, limit: i64, offset: i64) -> Result<Vec<ChainSummary>> {
        let conn = self.intelligence_conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT c.id, c.chain_name, c.chain_type, c.description, c.created_by, c.created_at, c.updated_at,
                    COUNT(l.id) as item_count,
                    AVG(l.relationship_strength) as avg_strength
             FROM evidence_chains c
             LEFT JOIN evidence_chain_links l ON c.id = l.chain_id
             GROUP BY c.id
             ORDER BY c.updated_at DESC
             LIMIT ?1 OFFSET ?2"
        )?;

        let entries = stmt.query_map(params![limit, offset], |row| {
            Ok(ChainSummary {
                id: row.get(0)?,
                chain_name: row.get(1)?,
                chain_type: row.get(2)?,
                description: row.get(3)?,
                created_by: row.get(4)?,
                created_at: row.get(5)?,
                updated_at: row.get(6)?,
                item_count: row.get(7)?,
                avg_strength: row.get(8)?,
            })
        })?;

        entries.collect()
    }

    pub fn update_chain(
        &self,
        chain_id: i64,
        name: Option<&str>,
        description: Option<&str>,
    ) -> Result<()> {
        let conn = self.intelligence_conn.lock().unwrap();

        if let Some(n) = name {
            conn.execute(
                "UPDATE evidence_chains SET chain_name = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
                params![n, chain_id],
            )?;
        }

        if let Some(d) = description {
            conn.execute(
                "UPDATE evidence_chains SET description = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
                params![d, chain_id],
            )?;
        }

        Ok(())
    }

    pub fn remove_from_chain(&self, chain_id: i64, intelligence_id: i64) -> Result<()> {
        let conn = self.intelligence_conn.lock().unwrap();
        conn.execute(
            "DELETE FROM evidence_chain_links WHERE chain_id = ?1 AND intelligence_id = ?2",
            params![chain_id, intelligence_id],
        )?;
        Ok(())
    }

    pub fn delete_chain(&self, chain_id: i64) -> Result<()> {
        let conn = self.intelligence_conn.lock().unwrap();
        conn.execute(
            "DELETE FROM evidence_chain_links WHERE chain_id = ?1",
            params![chain_id],
        )?;
        conn.execute(
            "DELETE FROM evidence_chains WHERE id = ?1",
            params![chain_id],
        )?;
        Ok(())
    }

    pub fn get_chain_statistics(&self, chain_id: i64) -> Result<ChainStatistics> {
        let conn = self.intelligence_conn.lock().unwrap();

        let total: (i32, f64, i32, i32) = conn.query_row(
            "SELECT COUNT(*), AVG(severity_score), MAX(severity_score), MIN(severity_score)
             FROM evidence_chain_links l
             JOIN intelligence i ON l.intelligence_id = i.id
             WHERE l.chain_id = ?1",
            [chain_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )?;

        let categories: Vec<String> = {
            let mut stmt = conn.prepare(
                "SELECT DISTINCT i.category FROM evidence_chain_links l
                 JOIN intelligence i ON l.intelligence_id = i.id
                 WHERE l.chain_id = ?1 AND i.category IS NOT NULL",
            )?;
            let result: Vec<String> = stmt
                .query_map([chain_id], |row| row.get(0))?
                .filter_map(|r| r.ok())
                .collect();
            result
        };

        let relationship_types: Vec<String> = {
            let mut stmt = conn.prepare(
                "SELECT DISTINCT relationship_type FROM evidence_chain_links WHERE chain_id = ?1",
            )?;
            let result: Vec<String> = stmt
                .query_map([chain_id], |row| row.get(0))?
                .filter_map(|r| r.ok())
                .collect();
            result
        };

        Ok(ChainStatistics {
            total_items: total.0,
            avg_severity: total.1,
            max_severity: total.2,
            min_severity: total.3,
            categories,
            relationship_types,
        })
    }

    pub fn search_chain(&self, chain_id: i64, query: &str) -> Result<Vec<ChainItem>> {
        let conn = self.intelligence_conn.lock().unwrap();
        let search_pattern = format!("%{}%", query);

        let mut stmt = conn.prepare(
            "SELECT l.id, l.intelligence_id, l.relationship_type, l.relationship_strength, l.notes, l.linked_by, l.linked_at,
                    i.filename, i.fact_summary, i.category
             FROM evidence_chain_links l
             JOIN intelligence i ON l.intelligence_id = i.id
             WHERE l.chain_id = ?1 AND (i.fact_summary LIKE ?2 OR i.filename LIKE ?2 OR i.category LIKE ?2)
             ORDER BY l.linked_at DESC"
        )?;

        let entries = stmt.query_map(params![chain_id, search_pattern], |row| {
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

    // Temporal analysis methods
    pub fn get_timeline_events(
        &self,
        start_date: Option<&str>,
        end_date: Option<&str>,
        limit: i64,
    ) -> Result<Vec<TimelineEvent>> {
        let conn = self.intelligence_conn.lock().unwrap();

        let mut sql = String::from(
            "SELECT id, fingerprint, filename, fact_summary, category, associated_date, severity_score, confidence
             FROM intelligence
             WHERE is_deleted = FALSE AND associated_date IS NOT NULL"
        );

        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

        if let Some(start) = start_date {
            sql.push_str(" AND associated_date >= ?");
            params.push(Box::new(start.to_string()));
        }

        if let Some(end) = end_date {
            sql.push_str(" AND associated_date <= ?");
            params.push(Box::new(end.to_string()));
        }

        sql.push_str(" ORDER BY associated_date ASC LIMIT ?");
        params.push(Box::new(limit));

        let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

        let mut stmt = conn.prepare(&sql)?;
        let entries = stmt.query_map(rusqlite::params_from_iter(param_refs.iter()), |row| {
            Ok(TimelineEvent {
                id: row.get(0)?,
                fingerprint: row.get(1)?,
                filename: row.get(2)?,
                summary: row.get(3)?,
                category: row.get(4)?,
                date: row.get(5)?,
                severity: row.get(6)?,
                confidence: row.get(7)?,
            })
        })?;

        entries.collect()
    }

    pub fn get_date_distribution(&self) -> Result<Vec<DateDistribution>> {
        let conn = self.intelligence_conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT 
                strftime('%Y-%m', associated_date) as month,
                COUNT(*) as count,
                AVG(severity_score) as avg_severity
             FROM intelligence
             WHERE is_deleted = FALSE AND associated_date IS NOT NULL
             GROUP BY month
             ORDER BY month DESC
             LIMIT 24",
        )?;

        let entries = stmt.query_map([], |row| {
            Ok(DateDistribution {
                period: row.get(0)?,
                count: row.get(1)?,
                avg_severity: row.get(2)?,
            })
        })?;

        entries.collect()
    }

    pub fn get_temporal_clusters(&self, time_window_days: i32) -> Result<Vec<TemporalCluster>> {
        let conn = self.intelligence_conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT 
                id,
                fingerprint,
                filename,
                fact_summary,
                associated_date,
                severity_score,
                julianday(associated_date) as jd
             FROM intelligence
             WHERE is_deleted = FALSE AND associated_date IS NOT NULL
             ORDER BY jd ASC",
        )?;

        let all_events: Vec<(i64, String, String, String, String, i32, f64)> = stmt
            .query_map([], |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                ))
            })?
            .filter_map(|r| r.ok())
            .collect();

        let mut clusters: Vec<TemporalCluster> = Vec::new();
        let mut current_cluster: Vec<ClusterItem> = Vec::new();
        let mut cluster_start_jd: Option<f64> = None;
        let mut cluster_start_date: Option<String> = None;

        for event in all_events {
            if let Some(start_jd) = cluster_start_jd {
                let diff = event.6 - start_jd;

                if diff > time_window_days as f64 {
                    if !current_cluster.is_empty() {
                        clusters.push(TemporalCluster {
                            start_date: cluster_start_date.clone(),
                            end_date: current_cluster.last().map(|i| i.date.clone()),
                            event_count: current_cluster.len() as i32,
                            events: current_cluster.clone(),
                        });
                    }
                    current_cluster.clear();
                    cluster_start_jd = Some(event.6);
                    cluster_start_date = Some(event.4.clone());
                }
            } else {
                cluster_start_jd = Some(event.6);
                cluster_start_date = Some(event.4.clone());
            }

            current_cluster.push(ClusterItem {
                id: event.0,
                fingerprint: event.1,
                filename: event.2,
                summary: event.3,
                date: event.4,
                severity: event.5,
            });
        }

        if !current_cluster.is_empty() {
            clusters.push(TemporalCluster {
                start_date: cluster_start_date,
                end_date: current_cluster.last().map(|i| i.date.clone()),
                event_count: current_cluster.len() as i32,
                events: current_cluster,
            });
        }

        Ok(clusters)
    }

    // Network analysis methods
    pub fn get_entity_relationships(
        &self,
        entity_id: Option<i64>,
        min_confidence: f64,
    ) -> Result<Vec<EntityRelationship>> {
        let conn = self.intelligence_conn.lock().unwrap();

        let sql = if let Some(eid) = entity_id {
            format!(
                "SELECT e1.id, e1.entity_type, e1.value, e2.id, e2.entity_type, e2.value,
                        COUNT(*) as cooccurrence, AVG(i.confidence) as avg_confidence
                 FROM entities e1
                 JOIN entities e2 ON e1.fingerprint = e2.fingerprint AND e1.id < e2.id
                 JOIN intelligence i ON e1.fingerprint = i.fingerprint
                 WHERE e1.id = {} AND i.confidence >= {}
                 GROUP BY e1.id, e2.id
                 ORDER BY cooccurrence DESC
                 LIMIT 100",
                eid, min_confidence
            )
        } else {
            format!(
                "SELECT e1.id, e1.entity_type, e1.value, e2.id, e2.entity_type, e2.value,
                        COUNT(*) as cooccurrence, AVG(i.confidence) as avg_confidence
                 FROM entities e1
                 JOIN entities e2 ON e1.fingerprint = e2.fingerprint AND e1.id < e2.id
                 JOIN intelligence i ON e1.fingerprint = i.fingerprint
                 WHERE i.confidence >= {}
                 GROUP BY e1.id, e2.id
                 ORDER BY cooccurrence DESC
                 LIMIT 100",
                min_confidence
            )
        };

        let mut stmt = conn.prepare(&sql)?;
        let entries = stmt.query_map([], |row| {
            Ok(EntityRelationship {
                entity1_id: row.get(0)?,
                entity1_type: row.get(1)?,
                entity1_value: row.get(2)?,
                entity2_id: row.get(3)?,
                entity2_type: row.get(4)?,
                entity2_value: row.get(5)?,
                cooccurrence: row.get(6)?,
                avg_confidence: row.get(7)?,
            })
        })?;

        entries.collect()
    }

    pub fn get_entity_centrality(
        &self,
        entity_type: Option<&str>,
        min_confidence: f64,
    ) -> Result<Vec<EntityCentrality>> {
        let conn = self.intelligence_conn.lock().unwrap();

        let type_filter = if let Some(et) = entity_type {
            format!("AND e.entity_type = '{}'", et)
        } else {
            String::new()
        };

        let sql = format!(
            "SELECT e.id, e.entity_type, e.value, 
                    COUNT(DISTINCT e.fingerprint) as document_count,
                    COUNT(e.id) as occurrence_count,
                    AVG(e.confidence) as avg_confidence
             FROM entities e
             JOIN intelligence i ON e.fingerprint = i.fingerprint
             WHERE i.confidence >= {} {}
             GROUP BY e.id
             ORDER BY occurrence_count DESC
             LIMIT 50",
            min_confidence, type_filter
        );

        let mut stmt = conn.prepare(&sql)?;
        let entries = stmt.query_map([], |row| {
            Ok(EntityCentrality {
                entity_id: row.get(0)?,
                entity_type: row.get(1)?,
                value: row.get(2)?,
                document_count: row.get(3)?,
                occurrence_count: row.get(4)?,
                avg_confidence: row.get(5)?,
                centrality_score: 0.0,
            })
        })?;

        let mut results: Vec<EntityCentrality> = entries.filter_map(|r| r.ok()).collect();

        if let Some(max_occ) = results.iter().map(|e| e.occurrence_count).max() {
            if max_occ > 0 {
                for r in &mut results {
                    r.centrality_score = r.occurrence_count as f64 / max_occ as f64;
                }
            }
        }

        results.sort_by(|a, b| {
            b.centrality_score
                .partial_cmp(&a.centrality_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(results)
    }

    pub fn get_connected_entities(
        &self,
        entity_id: i64,
        _depth: i32,
        min_confidence: f64,
    ) -> Result<Vec<ConnectedEntity>> {
        let conn = self.intelligence_conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT DISTINCT e2.id, e2.entity_type, e2.value, e2.confidence, i.filename
             FROM entities e1
             JOIN entities e2 ON e1.fingerprint = e2.fingerprint AND e1.id != e2.id
             JOIN intelligence i ON e1.fingerprint = i.fingerprint
             WHERE e1.id = ?1 AND i.confidence >= ?2",
        )?;

        let entries = stmt.query_map(params![entity_id, min_confidence], |row| {
            Ok(ConnectedEntity {
                entity_id: row.get(0)?,
                entity_type: row.get(1)?,
                value: row.get(2)?,
                confidence: row.get(3)?,
                source_file: row.get(4)?,
                distance: 1,
            })
        })?;

        entries.collect()
    }

    // Anomaly detection methods
    pub fn detect_anomalies(&self, metric: &str, threshold_std: f64) -> Result<Vec<Anomaly>> {
        let conn = self.intelligence_conn.lock().unwrap();

        match metric {
            "severity" => {
                let mut stmt = conn.prepare(
                    "SELECT id, fingerprint, filename, fact_summary, severity_score, associated_date
                     FROM intelligence
                     WHERE is_deleted = FALSE"
                )?;

                let all: Vec<(i64, String, String, String, i32, Option<String>)> = stmt
                    .query_map([], |row| {
                        Ok((
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                            row.get(4)?,
                            row.get(5)?,
                        ))
                    })?
                    .filter_map(|r| r.ok())
                    .collect();

                let values: Vec<f64> = all.iter().map(|i| i.4 as f64).collect();
                let (mean, std) = Self::calculate_mean_std(&values);

                Ok(all
                    .iter()
                    .filter(|i| {
                        let z = (i.4 as f64 - mean) / std;
                        z.abs() > threshold_std
                    })
                    .map(|i| {
                        let z = (i.4 as f64 - mean) / std;
                        Anomaly {
                            id: i.0,
                            fingerprint: i.1.clone(),
                            filename: i.2.clone(),
                            summary: i.3.clone(),
                            metric: "severity".to_string(),
                            value: i.4 as f64,
                            expected_value: mean,
                            deviation: z,
                            associated_date: i.5.clone(),
                        }
                    })
                    .collect())
            }
            "confidence" => {
                let mut stmt = conn.prepare(
                    "SELECT id, fingerprint, filename, fact_summary, confidence, associated_date
                     FROM intelligence
                     WHERE is_deleted = FALSE AND confidence IS NOT NULL",
                )?;

                let all: Vec<(i64, String, String, String, f64, Option<String>)> = stmt
                    .query_map([], |row| {
                        Ok((
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                            row.get(4)?,
                            row.get(5)?,
                        ))
                    })?
                    .filter_map(|r| r.ok())
                    .collect();

                let values: Vec<f64> = all.iter().map(|i| i.4).collect();
                let (mean, std) = Self::calculate_mean_std(&values);

                Ok(all
                    .iter()
                    .filter(|i| {
                        let z = (i.4 - mean) / std;
                        z.abs() > threshold_std
                    })
                    .map(|i| {
                        let z = (i.4 - mean) / std;
                        Anomaly {
                            id: i.0,
                            fingerprint: i.1.clone(),
                            filename: i.2.clone(),
                            summary: i.3.clone(),
                            metric: "confidence".to_string(),
                            value: i.4,
                            expected_value: mean,
                            deviation: z,
                            associated_date: i.5.clone(),
                        }
                    })
                    .collect())
            }
            "quality" => {
                let mut stmt = conn.prepare(
                    "SELECT id, fingerprint, filename, fact_summary, quality_score, associated_date
                     FROM intelligence
                     WHERE is_deleted = FALSE AND quality_score IS NOT NULL",
                )?;

                let all: Vec<(i64, String, String, String, f64, Option<String>)> = stmt
                    .query_map([], |row| {
                        Ok((
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                            row.get(4)?,
                            row.get(5)?,
                        ))
                    })?
                    .filter_map(|r| r.ok())
                    .collect();

                let values: Vec<f64> = all.iter().map(|i| i.4).collect();
                let (mean, std) = Self::calculate_mean_std(&values);

                Ok(all
                    .iter()
                    .filter(|i| {
                        let z = (i.4 - mean) / std;
                        z.abs() > threshold_std
                    })
                    .map(|i| {
                        let z = (i.4 - mean) / std;
                        Anomaly {
                            id: i.0,
                            fingerprint: i.1.clone(),
                            filename: i.2.clone(),
                            summary: i.3.clone(),
                            metric: "quality".to_string(),
                            value: i.4,
                            expected_value: mean,
                            deviation: z,
                            associated_date: i.5.clone(),
                        }
                    })
                    .collect())
            }
            _ => Ok(Vec::new()),
        }
    }

    fn calculate_mean_std(values: &[f64]) -> (f64, f64) {
        if values.is_empty() {
            return (0.0, 0.0);
        }

        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / values.len() as f64;
        let std = variance.sqrt();

        (mean, std)
    }

    pub fn get_temporal_anomalies(
        &self,
        window_days: i32,
        severity_threshold: i32,
    ) -> Result<Vec<TemporalAnomaly>> {
        let conn = self.intelligence_conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT 
                strftime('%Y-%m-%d', associated_date) as date,
                COUNT(*) as count,
                AVG(severity_score) as avg_severity
             FROM intelligence
             WHERE is_deleted = FALSE AND associated_date IS NOT NULL
             GROUP BY date
             ORDER BY date ASC",
        )?;

        let all: Vec<(String, i32, f64)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?
            .filter_map(|r| r.ok())
            .collect();

        let counts: Vec<f64> = all.iter().map(|i| i.1 as f64).collect();
        let (mean, std) = Self::calculate_mean_std(&counts);

        let window = window_days as usize;

        let mut anomalies = Vec::new();
        for i in 0..all.len() {
            let mut local_severity = 0.0;
            let start = i.saturating_sub(window);
            let end = (i + window).min(all.len());

            for item in &all[start..end] {
                local_severity += item.2;
            }

            let count = end - start;
            let local_avg = if count > 0 {
                local_severity / count as f64
            } else {
                0.0
            };

            if local_avg > severity_threshold as f64 {
                let z = ((all[i].1 as f64) - mean) / std;
                anomalies.push(TemporalAnomaly {
                    date: all[i].0.clone(),
                    event_count: all[i].1,
                    avg_severity: all[i].2,
                    local_avg_severity: local_avg,
                    deviation: z,
                });
            }
        }

        Ok(anomalies)
    }

    // Evidence weighting methods
    pub fn calculate_evidence_weight(&self, intelligence_id: i64) -> Result<f64> {
        let conn = self.intelligence_conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT severity_score, confidence, quality_score, created_at
             FROM intelligence WHERE id = ?1",
        )?;

        let (severity, confidence, quality): (i32, Option<f64>, Option<f64>) = stmt
            .query_row([intelligence_id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?))
            })?;

        let severity_weight = (severity as f64 / 5.0).min(1.0) * 0.4;
        let confidence_weight = confidence.unwrap_or(0.5) * 0.35;
        let quality_weight = quality.unwrap_or(0.5) * 0.25;

        Ok(severity_weight + confidence_weight + quality_weight)
    }

    pub fn get_weighted_evidence(
        &self,
        min_weight: f64,
        limit: i64,
    ) -> Result<Vec<WeightedEvidence>> {
        let conn = self.intelligence_conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT i.id, i.fingerprint, i.filename, i.fact_summary, i.category, 
                    i.severity_score, i.confidence, i.quality_score, i.created_at,
                    (i.severity_score / 5.0 * 0.4 + COALESCE(i.confidence, 0.5) * 0.35 + COALESCE(i.quality_score, 0.5) * 0.25) as weight
             FROM intelligence i
             WHERE i.is_deleted = FALSE
             ORDER BY weight DESC
             LIMIT ?1"
        )?;

        let entries = stmt.query_map([limit], |row| {
            Ok(WeightedEvidence {
                id: row.get(0)?,
                fingerprint: row.get(1)?,
                filename: row.get(2)?,
                summary: row.get(3)?,
                category: row.get(4)?,
                severity: row.get(5)?,
                confidence: row.get(6)?,
                quality: row.get(7)?,
                created_at: row.get(8)?,
                weight: row.get(9)?,
            })
        })?;

        Ok(entries
            .filter_map(|r| r.ok())
            .filter(|e| e.weight >= min_weight)
            .collect())
    }

    // Automatic chain detection methods
    pub fn detect_chains(
        &self,
        min_weight: f64,
        min_related: i32,
    ) -> Result<Vec<AutoDetectedChain>> {
        let weighted = self.get_weighted_evidence(min_weight, 1000)?;
        
        if weighted.is_empty() {
            return Ok(Vec::new());
        }

        let conn = self.intelligence_conn.lock().unwrap();
        
        let fetch_start = std::time::Instant::now();
        
        // Pre-fetch all entity data in ONE query (avoids N+1 problem)
        let mut entities_stmt = conn.prepare(
            "SELECT fingerprint, value FROM entities WHERE fingerprint IN (
                SELECT fingerprint FROM intelligence WHERE is_deleted = FALSE
            )"
        )?;
        
        let mut entities_by_fp: HashMap<String, Vec<String>> = HashMap::new();
        entities_stmt.query_map([], |row| {
            let fp: String = row.get(0)?;
            let value: String = row.get(1)?;
            Ok((fp, value))
        })?.filter_map(|r| r.ok())
          .for_each(|(fp, value)| {
               entities_by_fp.entry(fp).or_insert_with(Vec::new).push(value);
           });
        
        let fetch_time = fetch_start.elapsed();
        let entity_count = entities_by_fp.len();
        info!(entities_fetched = entity_count, fetch_time_ms = fetch_time.as_millis() as u64, "Fetched entities for chain detection");
        
        let mut chains = Vec::new();
        let empty_vec: Vec<String> = Vec::new();

        // Compute overlaps in memory instead of database queries
        for (i, current) in weighted.iter().enumerate() {
            let current_entities = entities_by_fp.get(&current.fingerprint).unwrap_or(&empty_vec);
            let mut related: Vec<RelatedEvidence> = Vec::new();

            for (j, other) in weighted.iter().enumerate() {
                if i == j {
                    continue;
                }

                let other_entities = entities_by_fp.get(&other.fingerprint).unwrap_or(&empty_vec);
                let overlap = current_entities.iter()
                    .filter(|e| other_entities.contains(e))
                    .count() as i32;

                if overlap >= min_related {
                    related.push(RelatedEvidence {
                        id: other.id,
                        fingerprint: other.fingerprint.clone(),
                        filename: other.filename.clone(),
                        summary: other.summary.clone(),
                        weight: other.weight,
                        shared_entities: overlap,
                    });
                }
            }

            if related.len() >= 2 {
                chains.push(AutoDetectedChain {
                    root_id: current.id,
                    root_summary: current.summary.clone(),
                    root_weight: current.weight,
                    related_count: related.len() as i32,
                    related_evidence: related,
                });
            }
        }

        chains.sort_by(|a, b| b.related_count.cmp(&a.related_count));
        
        let total_time = fetch_start.elapsed();
        info!(chains_found = chains.len(), total_time_ms = total_time.as_millis() as u64, "Chain detection completed");

        Ok(chains)
    }

    pub fn detect_chains_by_entities(
        &self,
        entity_values: &[String],
        min_occurrences: i32,
    ) -> Result<Vec<EntityChain>> {
        let conn = self.intelligence_conn.lock().unwrap();

        let placeholders: Vec<String> = entity_values.iter().map(|_| "?".to_string()).collect();
        let sql = format!(
            "SELECT i.id, i.fingerprint, i.filename, i.fact_summary, i.severity_score, i.confidence,
                    GROUP_CONCAT(e.value) as entities
             FROM intelligence i
             JOIN entities e ON i.fingerprint = e.fingerprint
             WHERE e.value IN ({})
             GROUP BY i.id
             HAVING COUNT(DISTINCT e.value) >= ?1",
            placeholders.join(",")
        );

        let mut params: Vec<&dyn rusqlite::ToSql> = entity_values
            .iter()
            .map(|v| v as &dyn rusqlite::ToSql)
            .collect();
        params.push(&min_occurrences);

        let mut stmt = conn.prepare(&sql)?;
        let entries = stmt.query_map(rusqlite::params_from_iter(params.iter()), |row| {
            Ok(EntityChain {
                intelligence_id: row.get(0)?,
                fingerprint: row.get(1)?,
                filename: row.get(2)?,
                summary: row.get(3)?,
                severity: row.get(4)?,
                confidence: row.get(5)?,
                matching_entities: row
                    .get::<_, String>(6)?
                    .split(',')
                    .map(|s| s.to_string())
                    .collect(),
            })
        })?;

        entries.collect()
    }

    pub fn get_chain_suggestions(
        &self,
        intelligence_id: i64,
        similarity_threshold: f64,
    ) -> Result<Vec<ChainSuggestion>> {
        let conn = self.intelligence_conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT fingerprint, fact_summary, category, severity_score FROM intelligence WHERE id = ?1"
        )?;
        let (fingerprint, summary, category, severity): (String, String, Option<String>, i32) =
            stmt.query_row([intelligence_id], |row| {
                Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
            })?;

        let keywords: Vec<&str> = summary
            .split_whitespace()
            .filter(|w| w.len() > 4)
            .take(10)
            .collect();

        let keyword_where = keywords
            .iter()
            .map(|k| format!("fact_summary LIKE '%{}%'", k))
            .collect::<Vec<_>>()
            .join(" OR ");

        let sql = format!(
            "SELECT id, fact_summary, category, severity_score, confidence
             FROM intelligence
             WHERE is_deleted = FALSE AND fingerprint != '{}' AND ({})
             ORDER BY severity_score DESC
             LIMIT 20",
            fingerprint, keyword_where
        );

        let mut stmt = conn.prepare(&sql)?;
        let suggestions: Vec<ChainSuggestion> = stmt
            .query_map([], |row| {
                let id: i64 = row.get(0)?;
                let sum: String = row.get(1)?;
                let cat: Option<String> = row.get(2)?;
                let sev: i32 = row.get(3)?;
                let _conf: Option<f64> = row.get(4)?;

                let keyword_matches = keywords
                    .iter()
                    .filter(|k| sum.to_lowercase().contains(&k.to_lowercase()))
                    .count();
                let similarity = (keyword_matches as f64 / keywords.len() as f64) * 0.7
                    + if cat.as_ref() == category.as_ref() {
                        0.2
                    } else {
                        0.0
                    }
                    + if (sev - severity).abs() <= 1 {
                        0.1
                    } else {
                        0.0
                    };

                Ok(ChainSuggestion {
                    target_id: id,
                    summary: sum,
                    category: cat,
                    similarity,
                    match_reasons: format!("{} keyword matches", keyword_matches),
                })
            })?
            .filter_map(|r| r.ok())
            .collect();

        Ok(suggestions
            .into_iter()
            .filter(|s| s.similarity >= similarity_threshold)
            .collect())
    }

    // Tags and annotations methods
    pub fn add_tag(&self, intelligence_id: i64, tag: &str) -> Result<()> {
        let conn = self.intelligence_conn.lock().unwrap();

        let current_tags: Option<String> = conn.query_row(
            "SELECT tags FROM intelligence WHERE id = ?1",
            [intelligence_id],
            |row| row.get(0),
        )?;

        let mut tags: Vec<String> = current_tags
            .map(|t| {
                t.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        if !tags.contains(&tag.to_string()) {
            tags.push(tag.to_string());
        }

        let tags_str = tags.join(",");

        conn.execute(
            "UPDATE intelligence SET tags = ?1 WHERE id = ?2",
            params![tags_str, intelligence_id],
        )?;

        Ok(())
    }

    pub fn remove_tag(&self, intelligence_id: i64, tag: &str) -> Result<()> {
        let conn = self.intelligence_conn.lock().unwrap();

        let current_tags: Option<String> = conn.query_row(
            "SELECT tags FROM intelligence WHERE id = ?1",
            [intelligence_id],
            |row| row.get(0),
        )?;

        let mut tags: Vec<String> = current_tags
            .map(|t| {
                t.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        tags.retain(|t| t != tag);

        let tags_str = tags.join(",");

        conn.execute(
            "UPDATE intelligence SET tags = ?1 WHERE id = ?2",
            params![tags_str, intelligence_id],
        )?;

        Ok(())
    }

    pub fn get_all_tags(&self) -> Result<Vec<String>> {
        let conn = self.intelligence_conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT DISTINCT tags FROM intelligence WHERE tags IS NOT NULL AND tags != ''",
        )?;

        let all_tags: Vec<String> = stmt
            .query_map([], |row| {
                let tags_str: Option<String> = row.get(0)?;
                Ok(tags_str)
            })?
            .filter_map(|r| r.ok())
            .flatten()
            .flat_map(|t| {
                t.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
            })
            .collect();

        let mut unique_tags: Vec<String> = all_tags.into_iter().collect();
        unique_tags.sort();
        unique_tags.dedup();

        Ok(unique_tags)
    }

    pub fn add_annotation(
        &self,
        intelligence_id: i64,
        content: &str,
        annotation_type: &str,
    ) -> Result<i64> {
        let conn = self.intelligence_conn.lock().unwrap();

        conn.execute(
            "INSERT INTO annotations (intelligence_id, content, annotation_type) VALUES (?1, ?2, ?3)",
            params![intelligence_id, content, annotation_type],
        )?;

        Ok(conn.last_insert_rowid())
    }

    pub fn update_annotation(&self, annotation_id: i64, content: &str) -> Result<()> {
        let conn = self.intelligence_conn.lock().unwrap();

        conn.execute(
            "UPDATE annotations SET content = ?1, updated_at = CURRENT_TIMESTAMP WHERE id = ?2",
            params![content, annotation_id],
        )?;

        Ok(())
    }

    pub fn delete_annotation(&self, annotation_id: i64) -> Result<()> {
        let conn = self.intelligence_conn.lock().unwrap();

        conn.execute("DELETE FROM annotations WHERE id = ?1", [annotation_id])?;

        Ok(())
    }

    pub fn get_annotations(&self, intelligence_id: i64) -> Result<Vec<Annotation>> {
        let conn = self.intelligence_conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT id, content, annotation_type, created_at, updated_at
             FROM annotations WHERE intelligence_id = ?1 ORDER BY created_at DESC",
        )?;

        let entries = stmt.query_map([intelligence_id], |row| {
            Ok(Annotation {
                id: row.get(0)?,
                intelligence_id,
                content: row.get(1)?,
                annotation_type: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?;

        entries.collect()
    }

    pub fn search_by_tags(
        &self,
        tags: &[String],
        match_all: bool,
        limit: i64,
    ) -> Result<Vec<SearchResult>> {
        let conn = self.intelligence_conn.lock().unwrap();

        let conditions: Vec<String> = tags
            .iter()
            .map(|t| format!("tags LIKE '%{}%'", t))
            .collect();

        let where_clause = if match_all {
            conditions.join(" AND ")
        } else {
            conditions.join(" OR ")
        };

        let sql = format!(
            "SELECT id, fingerprint, filename, fact_summary, category, severity_score, confidence, created_at,
                    0.0 as rank
             FROM intelligence
             WHERE is_deleted = FALSE AND ({}) LIMIT ?",
            where_clause
        );

        let mut stmt = conn.prepare(&sql)?;

        let entries = stmt.query_map([limit], |row| {
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

    pub fn get_location_entities(&self, min_confidence: f64) -> Result<Vec<LocationEntity>> {
        let conn = self.intelligence_conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT e.id, e.value, e.normalized_value, e.confidence, i.fingerprint, i.filename, i.fact_summary, i.severity_score
             FROM entities e
             JOIN intelligence i ON e.fingerprint = i.fingerprint
             WHERE e.entity_type = 'LOCATION' AND e.confidence >= ?1
             ORDER BY e.confidence DESC
             LIMIT 100"
        )?;

        let entries = stmt.query_map([min_confidence], |row| {
            let value: String = row.get(1)?;
            let normalized: Option<String> = row.get(2)?;

            let (lat, lon) = Self::parse_location(&normalized.clone().unwrap_or(value.clone()));

            Ok(LocationEntity {
                id: row.get(0)?,
                name: value,
                normalized_name: normalized,
                latitude: lat,
                longitude: lon,
                confidence: row.get(3)?,
                fingerprint: row.get(4)?,
                source_file: row.get(5)?,
                fact_summary: row.get(6)?,
                severity: row.get(7)?,
            })
        })?;

        entries.collect()
    }

    fn parse_location(loc: &str) -> (Option<f64>, Option<f64>) {
        let coords_re = regex::Regex::new(r"(-?\d+\.?\d*)[,\s]+(-?\d+\.?\d*)").ok();

        if let Some(re) = coords_re {
            if let Some(caps) = re.captures(loc) {
                if let (Ok(lat), Ok(lon)) = (
                    caps.get(1).unwrap().as_str().parse::<f64>(),
                    caps.get(2).unwrap().as_str().parse::<f64>(),
                ) {
                    if (-90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lon) {
                        return (Some(lat), Some(lon));
                    }
                }
            }
        }

        (None, None)
    }

    // Export and reporting methods
    pub fn export_facts_json(&self, filters: &ExportFilters) -> Result<String> {
        let facts = self.get_weighted_evidence(filters.min_weight, filters.limit)?;

        let export: Vec<serde_json::Value> = facts
            .into_iter()
            .map(|f| {
                serde_json::json!({
                    "id": f.id,
                    "fingerprint": f.fingerprint,
                    "filename": f.filename,
                    "summary": f.summary,
                    "category": f.category,
                    "severity": f.severity,
                    "confidence": f.confidence,
                    "quality": f.quality,
                    "weight": f.weight,
                    "created_at": f.created_at,
                })
            })
            .collect();

        serde_json::to_string_pretty(&export)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
    }

    pub fn export_entities_csv(
        &self,
        entity_type: Option<&str>,
        min_confidence: f64,
    ) -> Result<String> {
        let centrality = self.get_entity_centrality(entity_type, min_confidence)?;

        let mut csv = String::from("entity_id,entity_type,value,document_count,occurrence_count,avg_confidence,centrality_score\n");

        for e in centrality {
            csv.push_str(&format!(
                "{},{},\"{}\",{},{},{},{:.3}\n",
                e.entity_id,
                e.entity_type,
                e.value.replace('"', "\"\""),
                e.document_count,
                e.occurrence_count,
                e.avg_confidence.unwrap_or(0.0),
                e.centrality_score
            ));
        }

        Ok(csv)
    }

    pub fn export_timeline_json(
        &self,
        start_date: Option<&str>,
        end_date: Option<&str>,
    ) -> Result<String> {
        let events = self.get_timeline_events(start_date, end_date, 10000)?;

        serde_json::to_string_pretty(&events)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))
    }

    // Analytics and statistics with caching
    pub fn get_category_distribution(&self) -> Result<Vec<CategoryStats>> {
        // Check cache first
        {
            let cache = self.category_cache.lock().unwrap();
            if let Some(entry) = cache.as_ref() {
                if entry.is_valid() {
                    return Ok(entry.data.clone());
                }
            }
        }

        let conn = self.intelligence_conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT category, COUNT(*) as count, AVG(severity_score) as avg_severity, AVG(confidence) as avg_confidence
             FROM intelligence
             WHERE is_deleted = FALSE AND category IS NOT NULL
             GROUP BY category
             ORDER BY count DESC"
        )?;

        let entries: Result<Vec<CategoryStats>> = stmt.query_map([], |row| {
            Ok(CategoryStats {
                category: row.get(0)?,
                count: row.get(1)?,
                avg_severity: row.get(2)?,
                avg_confidence: row.get(3)?,
            })
        })?.collect();

        let result = entries?;
        
        // Update cache with 60-second TTL
        let mut cache = self.category_cache.lock().unwrap();
        *cache = Some(CacheEntry::new(result.clone(), Duration::from_secs(60)));
        
        Ok(result)
    }

    pub fn get_severity_distribution(&self) -> Result<Vec<SeverityStats>> {
        let conn = self.intelligence_conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT severity_score, COUNT(*) as count
             FROM intelligence
             WHERE is_deleted = FALSE
             GROUP BY severity_score
             ORDER BY severity_score DESC",
        )?;

        let entries = stmt.query_map([], |row| {
            Ok(SeverityStats {
                severity: row.get(0)?,
                count: row.get(1)?,
            })
        })?;

        entries.collect()
    }

    pub fn get_entity_type_distribution(&self) -> Result<Vec<EntityTypeStats>> {
        let conn = self.intelligence_conn.lock().unwrap();

        let mut stmt = conn.prepare(
            "SELECT entity_type, COUNT(DISTINCT value) as unique_count, COUNT(*) as total_count
             FROM entities
             WHERE is_deleted = FALSE
             GROUP BY entity_type
             ORDER BY total_count DESC",
        )?;

        let entries = stmt.query_map([], |row| {
            Ok(EntityTypeStats {
                entity_type: row.get(0)?,
                unique_count: row.get(1)?,
                total_count: row.get(2)?,
            })
        })?;

        entries.collect()
    }

    pub fn get_overall_statistics(&self) -> Result<OverallStatistics> {
        // Check cache first
        {
            let cache = self.overall_stats_cache.lock().unwrap();
            if let Some(entry) = cache.as_ref() {
                if entry.is_valid() {
                    return Ok(entry.data.clone());
                }
            }
        }

        let conn = self.intelligence_conn.lock().unwrap();

        let (total_facts, avg_severity, avg_confidence, avg_quality): (i64, f64, f64, f64) = conn
            .query_row(
            "SELECT COUNT(*), AVG(severity_score), AVG(confidence), AVG(quality_score)
             FROM intelligence WHERE is_deleted = FALSE",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        )?;

        let (total_entities, unique_entities): (i64, i64) = conn.query_row(
            "SELECT COUNT(*), COUNT(DISTINCT value) FROM entities WHERE is_deleted = FALSE",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        let (total_chains, total_chain_links): (i64, i64) = conn.query_row(
            "SELECT COUNT(*), (SELECT COUNT(*) FROM evidence_chain_links) FROM evidence_chains",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;

        let result = OverallStatistics {
            total_facts,
            avg_severity,
            avg_confidence,
            avg_quality,
            total_entities,
            unique_entities,
            total_chains,
            total_chain_links,
        };

        // Update cache with 30-second TTL
        let mut cache = self.overall_stats_cache.lock().unwrap();
        *cache = Some(CacheEntry::new(result.clone(), Duration::from_secs(30)));
        
        Ok(result)
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEvent {
    pub id: i64,
    pub fingerprint: String,
    pub filename: String,
    pub summary: String,
    pub category: Option<String>,
    pub date: String,
    pub severity: i32,
    pub confidence: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DateDistribution {
    pub period: String,
    pub count: i32,
    pub avg_severity: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalCluster {
    pub start_date: Option<String>,
    pub end_date: Option<String>,
    pub event_count: i32,
    pub events: Vec<ClusterItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterItem {
    pub id: i64,
    pub fingerprint: String,
    pub filename: String,
    pub summary: String,
    pub date: String,
    pub severity: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityRelationship {
    pub entity1_id: i64,
    pub entity1_type: String,
    pub entity1_value: String,
    pub entity2_id: i64,
    pub entity2_type: String,
    pub entity2_value: String,
    pub cooccurrence: i32,
    pub avg_confidence: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityCentrality {
    pub entity_id: i64,
    pub entity_type: String,
    pub value: String,
    pub document_count: i32,
    pub occurrence_count: i32,
    pub avg_confidence: Option<f64>,
    pub centrality_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectedEntity {
    pub entity_id: i64,
    pub entity_type: String,
    pub value: String,
    pub confidence: Option<f64>,
    pub source_file: String,
    pub distance: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    pub id: i64,
    pub fingerprint: String,
    pub filename: String,
    pub summary: String,
    pub metric: String,
    pub value: f64,
    pub expected_value: f64,
    pub deviation: f64,
    pub associated_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemporalAnomaly {
    pub date: String,
    pub event_count: i32,
    pub avg_severity: f64,
    pub local_avg_severity: f64,
    pub deviation: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightedEvidence {
    pub id: i64,
    pub fingerprint: String,
    pub filename: String,
    pub summary: String,
    pub category: Option<String>,
    pub severity: i32,
    pub confidence: Option<f64>,
    pub quality: Option<f64>,
    pub created_at: Option<String>,
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoDetectedChain {
    pub root_id: i64,
    pub root_summary: String,
    pub root_weight: f64,
    pub related_count: i32,
    pub related_evidence: Vec<RelatedEvidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelatedEvidence {
    pub id: i64,
    pub fingerprint: String,
    pub filename: String,
    pub summary: String,
    pub weight: f64,
    pub shared_entities: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityChain {
    pub intelligence_id: i64,
    pub fingerprint: String,
    pub filename: String,
    pub summary: String,
    pub severity: i32,
    pub confidence: Option<f64>,
    pub matching_entities: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainSuggestion {
    pub target_id: i64,
    pub summary: String,
    pub category: Option<String>,
    pub similarity: f64,
    pub match_reasons: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainSummary {
    pub id: i64,
    pub chain_name: String,
    pub chain_type: String,
    pub description: Option<String>,
    pub created_by: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    pub item_count: i32,
    pub avg_strength: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainStatistics {
    pub total_items: i32,
    pub avg_severity: f64,
    pub max_severity: i32,
    pub min_severity: i32,
    pub categories: Vec<String>,
    pub relationship_types: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportFilters {
    pub min_weight: f64,
    pub limit: i64,
    pub categories: Option<Vec<String>>,
    pub start_date: Option<String>,
    pub end_date: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CategoryStats {
    pub category: String,
    pub count: i32,
    pub avg_severity: Option<f64>,
    pub avg_confidence: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeverityStats {
    pub severity: i32,
    pub count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityTypeStats {
    pub entity_type: String,
    pub unique_count: i32,
    pub total_count: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Annotation {
    pub id: i64,
    pub intelligence_id: i64,
    pub content: String,
    pub annotation_type: String,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationEntity {
    pub id: i64,
    pub name: String,
    pub normalized_name: Option<String>,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub confidence: Option<f64>,
    pub fingerprint: String,
    pub source_file: String,
    pub fact_summary: String,
    pub severity: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverallStatistics {
    pub total_facts: i64,
    pub avg_severity: f64,
    pub avg_confidence: f64,
    pub avg_quality: f64,
    pub total_entities: i64,
    pub unique_entities: i64,
    pub total_chains: i64,
    pub total_chain_links: i64,
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
