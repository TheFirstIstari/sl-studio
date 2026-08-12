// Shared SQLite connection pool.
use anyhow::{Context, Result};
use rusqlite::{params, Connection, Params, Row};
use std::sync::{Arc, Mutex};
use tracing::info;

/// Shared SQLite connection pool.
#[derive(Clone)]
pub struct Pool {
    conn: Arc<Mutex<Connection>>,
}

impl Pool {
    /// Create a new connection pool from the given database path.
    pub fn connect(db_path: &str) -> Result<Pool> {
        let db_dir = std::path::Path::new(db_path).parent().unwrap();
        std::fs::create_dir_all(db_dir).with_context(|| {
            format!("Failed to create database directory: {}", db_dir.display())
        })?;

        let conn = Connection::open(db_path)
            .with_context(|| format!("Failed to open database: {}", db_path))?;
        info!("Opened SQLite database: {}", db_path);

        run_migrations(&conn).context("Failed to run database migrations")?;

        Ok(Pool {
            conn: Arc::new(Mutex::new(conn)),
        })
    }

    /// Execute a single SQL statement.
    pub fn execute<P: Params>(&self, sql: &str, params: P) -> Result<()> {
        self.conn
            .lock()
            .map_err(|e| anyhow::anyhow!("Database mutex poisoned: {}", e))?
            .execute(sql, params)
            .map(|_| ())
            .map_err(Into::into)
    }

    /// Execute a query and return all rows mapped by the provided closure.
    pub fn query_map<T, F, P>(&self, sql: &str, params: P, f: F) -> Result<Vec<T>>
    where
        F: FnMut(&Row) -> rusqlite::Result<T>,
        P: Params,
    {
        let conn = self
            .conn
            .lock()
            .map_err(|e| anyhow::anyhow!("Database mutex poisoned: {}", e))?;
        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map(params, f)?;
        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    }

    /// Execute a query and return a single row.
    pub fn query_row<T, F, P>(&self, sql: &str, params: P, f: F) -> Result<T>
    where
        F: FnOnce(&Row) -> rusqlite::Result<T>,
        P: Params,
    {
        let conn = self
            .conn
            .lock()
            .map_err(|e| anyhow::anyhow!("Database mutex poisoned: {}", e))?;
        Ok(conn.query_row(sql, params, f)?)
    }

    /// Execute a query and return a single optional row.
    pub fn query_row_optional<T, F, P>(&self, sql: &str, params: P, f: F) -> Result<Option<T>>
    where
        F: FnOnce(&Row) -> rusqlite::Result<T>,
        P: Params,
        T: Default,
    {
        let conn = self
            .conn
            .lock()
            .map_err(|e| anyhow::anyhow!("Database mutex poisoned: {}", e))?;
        match conn.query_row(sql, params, f) {
            Ok(v) => Ok(Some(v)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(anyhow::anyhow!("Query failed: {}", e)),
        }
    }
}

/// Run all pending migrations on the database.
fn run_migrations(conn: &Connection) -> Result<()> {
    let migrations = get_migrations();
    for mig in migrations {
        info!("Running migration: {}", mig.name);
        conn.execute(mig.sql, params![])
            .with_context(|| format!("Migration {} failed", mig.name))?;
    }
    Ok(())
}

/// Returns the list of pending migrations.
fn get_migrations() -> Vec<Migration> {
    vec![
        // ── Intelligence (facts) ────────────────────────────────────────
        Migration {
            name: "intelligence",
            sql: "CREATE TABLE IF NOT EXISTS intelligence (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                fingerprint TEXT NOT NULL,
                filename TEXT NOT NULL,
                fact_summary TEXT NOT NULL,
                category TEXT,
                identified_crime TEXT,
                severity_score INTEGER NOT NULL DEFAULT 0,
                confidence REAL,
                quality_score REAL,
                source_quote TEXT,
                associated_date TEXT,
                is_deleted BOOLEAN DEFAULT FALSE,
                deleted_at DATETIME,
                verification_status TEXT DEFAULT 'unverified',
                review_notes TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        },
        Migration {
            name: "idx_intelligence",
            sql: "CREATE INDEX IF NOT EXISTS idx_intelligence_fingerprint ON intelligence(fingerprint);
                  CREATE INDEX IF NOT EXISTS idx_intelligence_category ON intelligence(category);
                  CREATE INDEX IF NOT EXISTS idx_intelligence_deleted ON intelligence(is_deleted) WHERE is_deleted = FALSE;",
        },

        // ── Evidence chains ─────────────────────────────────────────────
        Migration {
            name: "evidence_chains",
            sql: "CREATE TABLE IF NOT EXISTS evidence_chains (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                chain_name TEXT NOT NULL,
                chain_type TEXT NOT NULL,
                description TEXT,
                created_by TEXT,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        },
        Migration {
            name: "chain_items",
            sql: "CREATE TABLE IF NOT EXISTS chain_items (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                chain_id INTEGER NOT NULL,
                intelligence_id INTEGER NOT NULL,
                relationship_type TEXT NOT NULL,
                relationship_strength REAL DEFAULT 1.0,
                notes TEXT,
                linked_by TEXT,
                linked_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY(chain_id) REFERENCES evidence_chains(id) ON DELETE CASCADE
            )",
        },
        Migration {
            name: "idx_chain_items",
            sql: "CREATE INDEX IF NOT EXISTS idx_chain_items_chain ON chain_items(chain_id);
                  CREATE INDEX IF NOT EXISTS idx_chain_items_intel ON chain_items(intelligence_id);",
        },

        // ── Facet presets ───────────────────────────────────────────────
        Migration {
            name: "facet_presets",
            sql: "CREATE TABLE IF NOT EXISTS facet_presets (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                page TEXT NOT NULL,
                name TEXT NOT NULL,
                state_json TEXT NOT NULL,
                updated_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            )",
        },

        // ── Pipelines ──────────────────────────────────────────────────
        Migration {
            name: "pipelines",
            sql: "CREATE TABLE IF NOT EXISTS pipelines (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT,
                passes_json TEXT NOT NULL,
                is_builtin BOOLEAN DEFAULT FALSE,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                modified_at DATETIME
            )",
        },

        // ── Entities & resolution ──────────────────────────────────────
        Migration {
            name: "entities",
            sql: "CREATE TABLE IF NOT EXISTS entities (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                intelligence_id INTEGER,
                fingerprint TEXT NOT NULL,
                entity_type TEXT NOT NULL,
                value TEXT NOT NULL,
                normalized_value TEXT,
                confidence REAL,
                position_start INTEGER,
                position_end INTEGER,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        },
        Migration {
            name: "entity_aliases",
            sql: "CREATE TABLE IF NOT EXISTS entity_aliases (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                canonical_entity_id INTEGER NOT NULL,
                alias_value TEXT NOT NULL,
                confidence REAL DEFAULT 1.0,
                is_manual BOOLEAN DEFAULT FALSE,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        },

        // ── Registry & text cache ──────────────────────────────────────
        Migration {
            name: "registry",
            sql: "CREATE TABLE IF NOT EXISTS registry (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                fingerprint TEXT NOT NULL UNIQUE,
                path TEXT NOT NULL,
                file_size INTEGER,
                file_type TEXT,
                file_name TEXT,
                last_modified DATETIME,
                has_extracted_text BOOLEAN DEFAULT FALSE,
                extracted_at DATETIME,
                processed BOOLEAN DEFAULT FALSE,
                processing_priority INTEGER DEFAULT 0,
                retry_count INTEGER DEFAULT 0,
                extraction_quality REAL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        },
        Migration {
            name: "text_cache",
            sql: "CREATE TABLE IF NOT EXISTS text_cache (
                fingerprint TEXT PRIMARY KEY,
                file_name TEXT NOT NULL,
                extracted_text TEXT,
                text_hash TEXT,
                extraction_time_ms INTEGER,
                quality_score REAL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        },

        // ── File metadata cache ────────────────────────────────────────
        Migration {
            name: "file_metadata_cache",
            sql: "CREATE TABLE IF NOT EXISTS file_metadata_cache (
                fingerprint TEXT PRIMARY KEY,
                metadata_json TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        },

        // ── Fact validations ───────────────────────────────────────────
        Migration {
            name: "fact_validations",
            sql: "CREATE TABLE IF NOT EXISTS fact_validations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                intelligence_id INTEGER NOT NULL,
                validation_type TEXT NOT NULL,
                is_valid BOOLEAN,
                details TEXT,
                checked_at DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        },

        // ── Evidence weights ───────────────────────────────────────────
        Migration {
            name: "evidence_weights",
            sql: "CREATE TABLE IF NOT EXISTS evidence_weights (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                intelligence_id INTEGER NOT NULL,
                weight_type TEXT NOT NULL DEFAULT 'primary',
                reliability_score REAL DEFAULT 1.0,
                custom_weight REAL,
                weighted_confidence REAL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME
            )",
        },

        // ── Audit log ──────────────────────────────────────────────────
        Migration {
            name: "audit_log",
            sql: "CREATE TABLE IF NOT EXISTS audit_log (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
                user_name TEXT,
                action TEXT NOT NULL,
                target_type TEXT,
                target_id TEXT,
                details_json TEXT,
                ip_address TEXT,
                session_id TEXT
            )",
        },
    ]
}

struct Migration {
    name: &'static str,
    sql: &'static str,
}
