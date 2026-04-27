use rusqlite::{params, Result};

use super::super::database::Database;

impl Database {
    pub fn get_entity_graph(
        &self,
        min_cooccurrence: i32,
    ) -> Result<(Vec<i64>, Vec<crate::inference::network::GraphEdge>)> {
        let conn = self.intel_conn()?;

        let sql = "SELECT e1.id, e2.id, COUNT(*) as cooccurrence
                   FROM entities e1
                   JOIN entities e2 ON e1.fingerprint = e2.fingerprint AND e1.id < e2.id
                   JOIN intelligence i ON e1.fingerprint = i.fingerprint
                   WHERE i.is_deleted = FALSE
                   GROUP BY e1.id, e2.id
                   HAVING cooccurrence >= ?1";

        let mut stmt = conn.prepare(sql)?;
        let rows = stmt.query_map(params![min_cooccurrence], |row| {
            let a: i64 = row.get(0)?;
            let b: i64 = row.get(1)?;
            let c: i64 = row.get(2)?;
            Ok((a, b, c))
        })?;

        let mut edges: Vec<crate::inference::network::GraphEdge> = Vec::new();
        let mut node_set: std::collections::BTreeSet<i64> = std::collections::BTreeSet::new();
        for r in rows {
            let (a, b, c) = r?;
            node_set.insert(a);
            node_set.insert(b);
            edges.push(crate::inference::network::GraphEdge {
                a,
                b,
                weight: c as f64,
            });
        }
        let nodes: Vec<i64> = node_set.into_iter().collect();
        Ok((nodes, edges))
    }
}
