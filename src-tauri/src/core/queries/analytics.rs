use rusqlite::Result;

use super::super::database::Database;
use super::super::database::*;

use super::super::database::CacheEntry;
use crate::WorkflowState;
use std::time::Duration;

impl Database {
    pub fn get_registry_count(&self) -> Result<i64> {
        let conn = self.reg_conn()?;
        conn.query_row("SELECT COUNT(*) FROM registry", [], |row| row.get(0))
    }

    pub fn get_processed_count(&self) -> Result<i64> {
        let conn = self.reg_conn()?;
        conn.query_row(
            "SELECT COUNT(*) FROM registry WHERE processed = 1",
            [],
            |row| row.get(0),
        )
    }

    pub fn get_intelligence_count(&self) -> Result<i64> {
        let conn = self.intel_conn()?;
        conn.query_row("SELECT COUNT(*) FROM intelligence", [], |row| row.get(0))
    }

    pub fn get_all_counts(&self) -> Result<AllCounts> {
        let reg_conn = self.reg_conn()?;
        let intel_conn = self.intel_conn()?;

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

    pub fn get_workflow_state(&self) -> Result<WorkflowState> {
        let conn = self.reg_conn()?;

        let total: i64 = conn.query_row("SELECT COUNT(*) FROM registry", [], |r| r.get(0))?;
        let extracted: i64 = conn.query_row(
            "SELECT COUNT(*) FROM registry WHERE has_extracted_text = 1",
            [],
            |r| r.get(0),
        )?;
        let analyzed: i64 = conn.query_row(
            "SELECT COUNT(*) FROM registry WHERE processed = 1",
            [],
            |r| r.get(0),
        )?;

        let last_scan_time: Option<String> = conn
            .query_row("SELECT MAX(created_at) FROM registry", [], |r| r.get(0))
            .ok();

        let last_extraction_time: Option<String> = conn
            .query_row(
                "SELECT MAX(extracted_at) FROM registry WHERE has_extracted_text = 1",
                [],
                |r| r.get(0),
            )
            .ok();

        let last_analysis_time: Option<String> = conn
            .query_row(
                "SELECT MAX(processed_at) FROM registry WHERE processed = 1",
                [],
                |r| r.get(0),
            )
            .ok();

        let current_stage = if analyzed > 0 {
            "analyzed"
        } else if extracted > 0 {
            "extracted"
        } else if total > 0 {
            "scanned"
        } else {
            "none"
        };

        Ok(WorkflowState {
            files_scanned: total,
            files_extracted: extracted,
            files_analyzed: analyzed,
            last_scan_time,
            last_extraction_time,
            last_analysis_time,
            current_stage: current_stage.to_string(),
            is_scanning: false,
            is_extracting: false,
            is_analyzing: false,
            scan_progress: 0.0,
            extract_progress: 0.0,
            analyze_progress: 0.0,
            current_file: String::new(),
            processed_count: 0,
            total_count: 0,
        })
    }

    pub fn get_extraction_statistics(&self) -> Result<ExtractionStatistics> {
        let conn = self.reg_conn()?;

        let total_files: i64 =
            conn.query_row("SELECT COUNT(*) FROM text_cache", [], |row| row.get(0))?;

        let total_characters: i64 = conn.query_row(
            "SELECT COALESCE(SUM(LENGTH(extracted_text)), 0) FROM text_cache",
            [],
            |row| row.get(0),
        )?;

        let average_characters = if total_files > 0 {
            total_characters as f64 / total_files as f64
        } else {
            0.0
        };

        let average_quality: f64 = conn.query_row(
            "SELECT COALESCE(AVG(quality_score), 0) FROM text_cache",
            [],
            |row| row.get(0),
        )?;

        let partial_count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM text_cache WHERE quality_score < 0.7",
            [],
            |row| row.get(0),
        )?;

        let mut stmt = conn.prepare(
            "SELECT r.file_type, COUNT(*) as count FROM text_cache t
             JOIN registry r ON t.fingerprint = r.fingerprint
             GROUP BY r.file_type",
        )?;

        let mut files_by_type = std::collections::HashMap::new();
        let rows = stmt.query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        })?;
        for row in rows.flatten() {
            files_by_type.insert(row.0, row.1);
        }

        Ok(ExtractionStatistics {
            total_files,
            total_characters,
            average_characters,
            average_quality,
            partial_count,
            files_by_type,
        })
    }

    // Metadata cache operations

    pub fn detect_anomalies(&self, metric: &str, threshold_std: f64) -> Result<Vec<Anomaly>> {
        let conn = self.intel_conn()?;

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
        let conn = self.intel_conn()?;

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

    pub fn get_category_distribution(&self) -> Result<Vec<CategoryStats>> {
        // Check cache first
        {
            if let Ok(cache) = self.category_cache.lock() {
                if let Some(entry) = cache.as_ref() {
                    if entry.is_valid() {
                        return Ok(entry.data.clone());
                    }
                }
            }
        }

        let conn = self.intel_conn()?;

        let mut stmt = conn.prepare(
            "SELECT category, COUNT(*) as count, AVG(severity_score) as avg_severity, AVG(confidence) as avg_confidence
             FROM intelligence
             WHERE is_deleted = FALSE AND category IS NOT NULL
             GROUP BY category
             ORDER BY count DESC"
        )?;

        let entries: Result<Vec<CategoryStats>> = stmt
            .query_map([], |row| {
                Ok(CategoryStats {
                    category: row.get(0)?,
                    count: row.get(1)?,
                    avg_severity: row.get(2)?,
                    avg_confidence: row.get(3)?,
                })
            })?
            .collect();

        let result = entries?;

        // Update cache with 60-second TTL (skip update if mutex is poisoned)
        if let Ok(mut cache) = self.category_cache.lock() {
            *cache = Some(CacheEntry::new(result.clone(), Duration::from_secs(60)));
        }

        Ok(result)
    }

    pub fn get_severity_distribution(&self) -> Result<Vec<SeverityStats>> {
        let conn = self.intel_conn()?;

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
        let conn = self.intel_conn()?;

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
            if let Ok(cache) = self.overall_stats_cache.lock() {
                if let Some(entry) = cache.as_ref() {
                    if entry.is_valid() {
                        return Ok(entry.data.clone());
                    }
                }
            }
        }

        let conn = self.intel_conn()?;

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

        // Update cache with 30-second TTL (skip update if mutex is poisoned)
        if let Ok(mut cache) = self.overall_stats_cache.lock() {
            *cache = Some(CacheEntry::new(result.clone(), Duration::from_secs(30)));
        }

        Ok(result)
    }
}
