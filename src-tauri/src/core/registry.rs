use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use std::sync::mpsc;
use tracing::{info, warn};
use walkdir::WalkDir;

use super::Database;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryProgress {
    pub total: usize,
    pub processed: usize,
    pub current_file: String,
    pub phase: String,
}

pub struct RegistryWorker {
    db: Option<Database>,
    evidence_root: String,
    batch_size: usize,
}

impl RegistryWorker {
    pub fn new(
        evidence_root: &str,
        registry_db_path: &str,
        intelligence_db_path: &str,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let db = Database::new(registry_db_path, intelligence_db_path)?;
        Ok(RegistryWorker {
            db: Some(db),
            evidence_root: evidence_root.to_string(),
            batch_size: 100,
        })
    }

    pub fn scan(
        &mut self,
        progress_tx: mpsc::Sender<RegistryProgress>,
    ) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
        let db = self.db.as_ref().ok_or("Database not initialized")?;

        info!("Starting registry scan of: {}", self.evidence_root);

        progress_tx
            .send(RegistryProgress {
                total: 0,
                processed: 0,
                current_file: "Discovering files...".to_string(),
                phase: "discovery".to_string(),
            })
            .ok();

        // Phase 1: Discover files
        let files: Vec<_> = WalkDir::new(&self.evidence_root)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| e.path().to_path_buf())
            .collect();

        let total_files = files.len();
        info!("Discovered {} files", total_files);

        progress_tx
            .send(RegistryProgress {
                total: total_files,
                processed: 0,
                current_file: format!("Found {} files...", total_files),
                phase: "discovery".to_string(),
            })
            .ok();

        // Phase 2: Load existing fingerprints into memory for fast skip
        progress_tx
            .send(RegistryProgress {
                total: total_files,
                processed: 0,
                current_file: "Loading cache...".to_string(),
                phase: "cache".to_string(),
            })
            .ok();

        let existing_fingerprints = db.get_all_fingerprints().unwrap_or_default();
        let existing_count = existing_fingerprints.len();
        info!("Loaded {} existing fingerprints from cache", existing_count);

        progress_tx
            .send(RegistryProgress {
                total: total_files,
                processed: 0,
                current_file: "Hashing files...".to_string(),
                phase: "hashing".to_string(),
            })
            .ok();

        // Phase 3: Parallel hashing with batch inserts
        let batch_size = self.batch_size;
        let mut new_count = 0;

        // Use rayon for parallel processing
        let results: Vec<(String, String, String, i64, String)> = files
            .par_iter()
            .filter_map(|path| {
                let file_type = path
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("unknown")
                    .to_lowercase();

                let file_name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string();

                match hash_file(path) {
                    Ok(fingerprint) => {
                        // Skip if already exists
                        if existing_fingerprints.contains(&fingerprint) {
                            return None;
                        }

                        let file_size =
                            std::fs::metadata(path).map(|m| m.len() as i64).unwrap_or(0);
                        Some((
                            fingerprint,
                            path.to_string_lossy().to_string(),
                            file_type,
                            file_size,
                            file_name,
                        ))
                    }
                    Err(e) => {
                        warn!("Failed to hash {}: {}", path.display(), e);
                        None
                    }
                }
            })
            .collect();

        // Batch insert with incremental progress
        let mut processed = 0;
        for chunk in results.chunks(batch_size) {
            let entries: Vec<_> = chunk
                .iter()
                .map(|(fp, p, ft, fs, fnm)| (fp.clone(), p.clone(), ft.clone(), *fs, fnm.clone()))
                .collect();
            if let Some(db) = &self.db {
                match db.insert_fingerprints_batch(&entries) {
                    Ok(count) => new_count += count,
                    Err(e) => warn!("Batch insert failed: {}", e),
                }
            }

            processed += chunk.len();
            progress_tx
                .send(RegistryProgress {
                    total: total_files,
                    processed: processed,
                    current_file: format!("Hashing {} / {}...", processed, total_files),
                    phase: "hashing".to_string(),
                })
                .ok();
        }

        // Send final completion
        progress_tx
            .send(RegistryProgress {
                total: total_files,
                processed: total_files,
                current_file: format!(
                    "Scan complete: {} new, {} existing",
                    new_count,
                    total_files.saturating_sub(new_count)
                ),
                phase: "complete".to_string(),
            })
            .ok();

        info!(
            "Registry scan complete. Discovered {} files: {} new, {} already existed",
            total_files,
            new_count,
            total_files.saturating_sub(new_count)
        );

        // Log audit
        if let Some(db) = &self.db {
            db.log_audit(
                "registry_scan",
                &format!("Added {} new files, {} existing", new_count, existing_count),
                None,
            )
            .ok();
        }

        Ok(new_count)
    }

    pub fn get_stats(&self) -> Result<(i64, i64), Box<dyn std::error::Error + Send + Sync>> {
        let db = self.db.as_ref().ok_or("Database not initialized")?;
        let registry_count = db.get_registry_count()?;
        let intelligence_count = db.get_intelligence_count()?;
        Ok((registry_count, intelligence_count))
    }
}

fn hash_file(path: &Path) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();

    // 64KB buffer for faster I/O
    let mut buffer = [0u8; 65536];
    loop {
        let bytes_read = reader.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(hex::encode(hasher.finalize()))
}

pub fn hash_file_sync(path: &Path) -> Result<String, String> {
    hash_file(path).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_hash_file() {
        let mut tmp = NamedTempFile::new().unwrap();
        tmp.write_all(b"test content").unwrap();

        let hash = hash_file(tmp.path()).unwrap();
        assert_eq!(hash.len(), 64); // SHA-256 hex is 64 chars
    }
}
