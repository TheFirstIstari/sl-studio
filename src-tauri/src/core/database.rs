use r2d2::Pool;
use r2d2_sqlite::SqliteConnectionManager;
use rusqlite::Result;

use crate::core::migrations::{intelligence_migrations, registry_migrations, run_migrations};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tracing::info;

pub type DbPool = Pool<SqliteConnectionManager>;
pub type PooledConn = r2d2::PooledConnection<SqliteConnectionManager>;

fn pool_err(e: r2d2::Error) -> rusqlite::Error {
    rusqlite::Error::SqliteFailure(
        rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CANTOPEN),
        Some(format!("connection pool exhausted: {e}")),
    )
}

fn build_pool(path: &str) -> Result<DbPool> {
    let manager = SqliteConnectionManager::file(path).with_init(|c| {
        c.execute_batch(
            "PRAGMA journal_mode = WAL;\n\
             PRAGMA synchronous = NORMAL;\n\
             PRAGMA busy_timeout = 5000;\n\
             PRAGMA foreign_keys = ON;",
        )
    });
    Pool::builder().max_size(8).build(manager).map_err(|e| {
        rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CANTOPEN),
            Some(format!("failed to build connection pool: {e}")),
        )
    })
}

pub(crate) struct CacheEntry<T: Clone> {
    pub(crate) data: T,
    expires_at: Instant,
}

impl<T: Clone> CacheEntry<T> {
    pub(crate) fn new(data: T, ttl: Duration) -> Self {
        Self {
            data,
            expires_at: Instant::now() + ttl,
        }
    }

    pub(crate) fn is_valid(&self) -> bool {
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
    pub location: Option<String>,
    pub people: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractionStatistics {
    pub total_files: i64,
    pub total_characters: i64,
    pub average_characters: f64,
    pub average_quality: f64,
    pub partial_count: i64,
    pub files_by_type: std::collections::HashMap<String, i64>,
}

pub struct Database {
    registry_pool: DbPool,
    intelligence_pool: DbPool,
    pub(crate) category_cache: Mutex<Option<CacheEntry<Vec<CategoryStats>>>>,
    pub(crate) severity_cache: Mutex<Option<CacheEntry<Vec<SeverityStats>>>>,
    pub(crate) overall_stats_cache: Mutex<Option<CacheEntry<OverallStatistics>>>,
}

impl Database {
    pub fn new(registry_path: &str, intelligence_path: &str) -> Result<Self> {
        let registry_pool = build_pool(registry_path)?;
        let intelligence_pool = build_pool(intelligence_path)?;

        let db = Database {
            registry_pool,
            intelligence_pool,
            category_cache: Mutex::new(None),
            severity_cache: Mutex::new(None),
            overall_stats_cache: Mutex::new(None),
        };

        db.init_schema()?;
        db.run_migrations()?;
        Ok(db)
    }

    pub(crate) fn reg_conn(&self) -> Result<PooledConn> {
        self.registry_pool.get().map_err(pool_err)
    }

    pub(crate) fn intel_conn(&self) -> Result<PooledConn> {
        self.intelligence_pool.get().map_err(pool_err)
    }

    fn run_migrations(&self) -> Result<()> {
        {
            let mut conn = self.reg_conn()?;
            run_migrations(&mut *conn, &registry_migrations())?;
        }
        {
            let mut conn = self.intel_conn()?;
            run_migrations(&mut *conn, &intelligence_migrations())?;
        }
        Ok(())
    }

    /// Returns the highest applied migration version across the registry
    /// database's `schema_migrations` table. Returns 0 if no migrations have
    /// been applied (or the table does not exist yet).
    pub fn schema_version(&self) -> Result<i64> {
        let conn = self.reg_conn()?;
        let exists: i64 = conn.query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='schema_migrations'",
            [],
            |row| row.get(0),
        )?;
        if exists == 0 {
            return Ok(0);
        }
        conn.query_row(
            "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
            [],
            |row| row.get(0),
        )
    }

    fn init_schema(&self) -> Result<()> {
        let reg_conn = self.reg_conn()?;
        let intel_conn = self.intel_conn()?;

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
                location TEXT,
                people TEXT,
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
                verification_status TEXT DEFAULT 'unverified',
                review_notes TEXT,
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

    pub fn invalidate_cache(&self) {
        *self.category_cache.lock().unwrap() = None;
        *self.severity_cache.lock().unwrap() = None;
        *self.overall_stats_cache.lock().unwrap() = None;
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
