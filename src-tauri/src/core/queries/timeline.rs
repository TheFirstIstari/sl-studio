use rusqlite::Result;

use super::super::database::Database;
use super::super::database::*;

impl Database {
    pub fn get_timeline_events(
        &self,
        start_date: Option<&str>,
        end_date: Option<&str>,
        limit: i64,
    ) -> Result<Vec<TimelineEvent>> {
        let conn = self.intel_conn()?;

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
        let conn = self.intel_conn()?;

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
        let conn = self.intel_conn()?;

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

    pub fn get_location_entities(&self, min_confidence: f64) -> Result<Vec<LocationEntity>> {
        let conn = self.intel_conn()?;

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
}
