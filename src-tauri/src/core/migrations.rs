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
    vec![Migration {
        version: 1,
        name: "baseline_intelligence_schema",
        up_sql: "SELECT 1",
    }]
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
        run_migrations(&mut conn, &intelligence_migrations()).unwrap();
        run_migrations(&mut conn, &registry_migrations()).unwrap();
        let max: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(version), 0) FROM schema_migrations",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(max, 1);
    }
}
