use rusqlite::{params, Connection, Result};

pub struct Migration {
    pub version: i64,
    pub name: &'static str,
    pub up_sql: &'static str,
}

/// Migrations for the intelligence database.
///
/// Version 1 is the baseline that simply records the schema produced by
/// `Database::init_schema()` as already applied. Future migrations should be
/// appended with strictly increasing versions.
pub fn intelligence_migrations() -> Vec<Migration> {
    vec![
        Migration {
            version: 1,
            name: "baseline_intelligence_schema",
            up_sql: "SELECT 1",
        },
        // FR-PLP: persist user-defined LLM pipelines.
        Migration {
            version: 2,
            name: "create_pipelines_table",
            up_sql: "CREATE TABLE IF NOT EXISTS pipelines (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                description TEXT NOT NULL DEFAULT '',
                passes_json TEXT NOT NULL,
                is_builtin BOOLEAN NOT NULL DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );
            CREATE INDEX IF NOT EXISTS idx_pipelines_builtin ON pipelines(is_builtin);",
        },
        // FR-FACET-004: persist saved filter/facet presets per page.
        Migration {
            version: 3,
            name: "create_facet_presets_table",
            up_sql: "CREATE TABLE IF NOT EXISTS facet_presets (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                page TEXT NOT NULL,
                name TEXT NOT NULL,
                state_json TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
                UNIQUE (page, name)
            );
            CREATE INDEX IF NOT EXISTS idx_facet_presets_page ON facet_presets(page);",
        },
        // Performance: indexes for common analytic filters.
        // associated_date is used by timeline queries; confidence is used by
        // weighted-evidence, centrality, and anomaly filters.
        Migration {
            version: 4,
            name: "add_intelligence_analytic_indexes",
            up_sql: "CREATE INDEX IF NOT EXISTS idx_intelligence_associated_date
                        ON intelligence(associated_date)
                        WHERE is_deleted = FALSE AND associated_date IS NOT NULL;
                     CREATE INDEX IF NOT EXISTS idx_intelligence_confidence
                        ON intelligence(confidence)
                        WHERE is_deleted = FALSE AND confidence IS NOT NULL;",
        },
    ]
}

/// Migrations for the registry database.
pub fn registry_migrations() -> Vec<Migration> {
    vec![Migration {
        version: 1,
        name: "baseline_registry_schema",
        up_sql: "SELECT 1",
    }]
}

/// Apply pending migrations to the given connection.
///
/// Creates the `schema_migrations` tracking table if necessary, then executes
/// each migration whose version is greater than the highest applied version.
/// Each migration runs inside its own transaction. Re-running this function
/// when no migrations are pending is a no-op.
pub fn run_migrations(conn: &mut Connection, migrations: &[Migration]) -> Result<()> {
    conn.execute(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            name TEXT,
            applied_at DATETIME DEFAULT CURRENT_TIMESTAMP
        )",
        [],
    )?;

    let applied: std::collections::HashSet<i64> = {
        let mut stmt = conn.prepare("SELECT version FROM schema_migrations")?;
        let rows = stmt.query_map([], |row| row.get::<_, i64>(0))?;
        let mut set = std::collections::HashSet::new();
        for v in rows {
            set.insert(v?);
        }
        set
    };

    for migration in migrations {
        if applied.contains(&migration.version) {
            continue;
        }
        let tx = conn.transaction()?;
        tx.execute_batch(migration.up_sql)?;
        tx.execute(
            "INSERT INTO schema_migrations (version, name) VALUES (?1, ?2)",
            params![migration.version, migration.name],
        )?;
        tx.commit()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_migrations() -> Vec<Migration> {
        vec![
            Migration {
                version: 1,
                name: "create_foo",
                up_sql: "CREATE TABLE foo (id INTEGER)",
            },
            Migration {
                version: 2,
                name: "add_bar_column",
                up_sql: "ALTER TABLE foo ADD COLUMN bar TEXT",
            },
        ]
    }

    #[test]
    fn applies_migrations_and_tracks_versions() {
        let mut conn = Connection::open_in_memory().unwrap();
        let migs = fake_migrations();
        run_migrations(&mut conn, &migs).unwrap();

        let versions: Vec<i64> = conn
            .prepare("SELECT version FROM schema_migrations ORDER BY version")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .map(|r| r.unwrap())
            .collect();
        assert_eq!(versions, vec![1, 2]);

        // foo table exists with both columns
        conn.execute("INSERT INTO foo (id, bar) VALUES (1, 'hi')", [])
            .unwrap();
    }

    #[test]
    fn rerun_is_idempotent() {
        let mut conn = Connection::open_in_memory().unwrap();
        let migs = fake_migrations();
        run_migrations(&mut conn, &migs).unwrap();
        run_migrations(&mut conn, &migs).unwrap();

        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn baseline_migrations_run_clean() {
        let mut conn = Connection::open_in_memory().unwrap();
        // Migrations assume Database::init_schema() already created the core
        // tables (CREATE TABLE IF NOT EXISTS). Simulate the minimal subset
        // that later migrations index against.
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS intelligence (
                id INTEGER PRIMARY KEY,
                associated_date TEXT,
                confidence REAL,
                is_deleted BOOLEAN DEFAULT FALSE
            );",
        )
        .unwrap();
        run_migrations(&mut conn, &intelligence_migrations()).unwrap();
        let max: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
                [],
                |r| r.get(0),
            )
            .unwrap();
        // Highest intelligence migration version (currently 4: analytic indexes).
        assert!(max >= 4);

        let mut reg_conn = Connection::open_in_memory().unwrap();
        run_migrations(&mut reg_conn, &registry_migrations()).unwrap();
    }

    #[test]
    fn pipelines_migration_creates_table() {
        let mut conn = Connection::open_in_memory().unwrap();
        // v4 indexes the `intelligence` table; pre-create the minimum schema.
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS intelligence (
                id INTEGER PRIMARY KEY,
                associated_date TEXT,
                confidence REAL,
                is_deleted BOOLEAN DEFAULT FALSE
            );",
        )
        .unwrap();
        run_migrations(&mut conn, &intelligence_migrations()).unwrap();
        // Insert a row to confirm the table is usable.
        conn.execute(
            "INSERT INTO pipelines (id, name, passes_json, is_builtin) VALUES (?1, ?2, ?3, ?4)",
            params!["test", "Test Pipeline", "[]", 0],
        )
        .unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM pipelines", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);
    }
}
