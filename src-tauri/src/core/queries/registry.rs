use rusqlite::{params, Result};

use super::super::database::Database;
use super::super::database::*;

use std::path::Path;

impl Database {
    pub fn insert_fingerprint(
        &self,
        fingerprint: &str,
        path: &str,
        file_type: &str,
        file_size: i64,
        file_name: &str,
    ) -> Result<i64> {
        let conn = self.reg_conn()?;
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
        let mut conn = self.reg_conn()?;
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
        let conn = self.reg_conn()?;
        let mut stmt = conn.prepare("SELECT fingerprint FROM registry")?;
        let fingerprints = stmt.query_map([], |row| row.get(0))?;

        let mut set = std::collections::HashSet::new();
        for f in fingerprints.flatten() {
            set.insert(f);
        }
        Ok(set)
    }

    pub fn mark_processed(&self, fingerprint: &str) -> Result<()> {
        let conn = self.reg_conn()?;
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
        let conn = self.reg_conn()?;
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
        let conn = self.reg_conn()?;
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

        let conn = self.reg_conn()?;
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

        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(metadata.len().to_le_bytes());
        if let Ok(mtime) = metadata.modified() {
            hasher.update(
                mtime
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
                    .to_le_bytes(),
            );
        }
        hasher.update(&buffer);
        let hash = hasher.finalize();

        Ok(format!("{:x}-{}", hash, metadata.len()))
    }

    pub fn get_unprocessed_files(&self, limit: i64) -> Result<Vec<RegistryEntry>> {
        let conn = self.reg_conn()?;
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

    /// Return all registry entries, ordered by file_name, up to `limit` rows.
    /// Used by the FR-META metadata viewer page.
    pub fn get_all_registry_files(&self, limit: i64) -> Result<Vec<RegistryEntry>> {
        let conn = self.reg_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, fingerprint, path, file_size, file_type, file_name,
                    last_modified, last_hash_check, has_extracted_text,
                    extracted_at, processed_at, processed,
                    processing_priority, retry_count, extraction_quality, created_at
             FROM registry
             ORDER BY file_name ASC
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

    pub fn save_text_cache(
        &self,
        fingerprint: &str,
        file_name: &str,
        text: &str,
        text_hash: &str,
        extraction_time_ms: i64,
        quality_score: f64,
    ) -> Result<()> {
        let conn = self.reg_conn()?;
        conn.execute(
            "INSERT OR REPLACE INTO text_cache (fingerprint, file_name, extracted_text, text_hash, extraction_time_ms, quality_score, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, CURRENT_TIMESTAMP)",
            params![fingerprint, file_name, text, text_hash, extraction_time_ms, quality_score],
        )?;
        Ok(())
    }

    pub fn get_text_cache(&self, fingerprint: &str) -> Result<Option<TextCacheEntry>> {
        let conn = self.reg_conn()?;
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
        let conn = self.reg_conn()?;
        conn.query_row("SELECT COUNT(*) FROM text_cache", [], |row| row.get(0))
    }

    pub fn save_metadata_cache(
        &self,
        fingerprint: &str,
        metadata_type: &str,
        metadata_json: &str,
    ) -> Result<()> {
        let conn = self.reg_conn()?;
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
        let conn = self.reg_conn()?;
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

    pub fn get_extraction_queue(&self, limit: i64) -> Result<Vec<RegistryEntry>> {
        let conn = self.reg_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, fingerprint, path, file_size, file_type, file_name,
                    last_modified, last_hash_check, has_extracted_text,
                    extracted_at, processed_at, processed,
                    processing_priority, retry_count, extraction_quality, created_at
             FROM registry
             WHERE has_extracted_text = 0
             ORDER BY processing_priority ASC, last_modified DESC
             LIMIT ?1",
        )?;

        let entries = stmt.query_map([limit], |row: &rusqlite::Row| {
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

    /// Get files that have extracted text but haven't been analyzed
    pub fn get_analysis_queue(&self, limit: i64) -> Result<Vec<RegistryEntry>> {
        let conn = self.reg_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, fingerprint, path, file_size, file_type, file_name,
                    last_modified, last_hash_check, has_extracted_text,
                    extracted_at, processed_at, processed,
                    processing_priority, retry_count, extraction_quality, created_at
             FROM registry
             WHERE has_extracted_text = 1 AND processed = 0
             ORDER BY processing_priority ASC, last_modified DESC
             LIMIT ?1",
        )?;

        let entries = stmt.query_map([limit], |row: &rusqlite::Row| {
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

    /// Mark a file as having extracted text
    pub fn mark_extracted(&self, fingerprint: &str, is_partial: bool) -> Result<()> {
        let conn = self.reg_conn()?;
        conn.execute(
            "UPDATE registry SET 
                has_extracted_text = 1,
                extracted_at = CURRENT_TIMESTAMP,
                extraction_quality = CASE WHEN ?2 = 1 THEN 0.5 ELSE 1.0 END
             WHERE fingerprint = ?1",
            params![fingerprint, is_partial as i32],
        )?;
        Ok(())
    }

    /// Get extracted text from text_cache
    pub fn get_extracted_text(&self, fingerprint: &str) -> Result<Option<String>> {
        let conn = self.reg_conn()?;
        let mut stmt =
            conn.prepare("SELECT extracted_text FROM text_cache WHERE fingerprint = ?1")?;

        let mut rows = stmt.query(params![fingerprint])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    /// Get registry entry by fingerprint
    pub fn get_registry_entry(&self, fingerprint: &str) -> Result<RegistryEntry> {
        let conn = self.reg_conn()?;
        let mut stmt = conn.prepare(
            "SELECT id, fingerprint, path, file_size, file_type, file_name,
                    last_modified, last_hash_check, has_extracted_text,
                    extracted_at, processed_at, processed,
                    processing_priority, retry_count, extraction_quality, created_at
             FROM registry WHERE fingerprint = ?1",
        )?;

        stmt.query_row([fingerprint], |row: &rusqlite::Row| {
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
        })
    }
}
