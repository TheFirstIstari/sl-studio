//! FR-NET-005: Network analysis primitives
//!
//! Pure-Rust implementations of community detection (connected components via
//! union-find), Brandes' betweenness centrality, and local clustering
//! coefficient. No external graph crates are used.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub a: i64,
    pub b: i64,
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityCommunity {
    pub community_id: i64,
    pub entity_ids: Vec<i64>,
    pub size: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityBetweenness {
    pub entity_id: i64,
    pub betweenness: f64,
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build an index map (entity_id -> contiguous index) and an adjacency list
/// (vec of vec of neighbor indices). Self-loops and duplicate edges are
/// collapsed deterministically.
fn build_adjacency(
    edges: &[GraphEdge],
    node_ids: &[i64],
) -> (HashMap<i64, usize>, Vec<Vec<usize>>) {
    let mut idx: HashMap<i64, usize> = HashMap::with_capacity(node_ids.len());
    for (i, &id) in node_ids.iter().enumerate() {
        idx.entry(id).or_insert(i);
    }
    let n = node_ids.len();
    let mut adj: Vec<HashSet<usize>> = vec![HashSet::new(); n];
    for e in edges {
        let (Some(&u), Some(&v)) = (idx.get(&e.a), idx.get(&e.b)) else {
            continue;
        };
        if u == v {
            continue;
        }
        adj[u].insert(v);
        adj[v].insert(u);
    }
    let adj: Vec<Vec<usize>> = adj
        .into_iter()
        .map(|s| {
            let mut v: Vec<usize> = s.into_iter().collect();
            v.sort_unstable();
            v
        })
        .collect();
    (idx, adj)
}

// ---------------------------------------------------------------------------
// Community detection (connected components / union-find)
// ---------------------------------------------------------------------------

struct DSU {
    parent: Vec<usize>,
    rank: Vec<u32>,
}

impl DSU {
    fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
            rank: vec![0; n],
        }
    }
    fn find(&mut self, x: usize) -> usize {
        let mut r = x;
        while self.parent[r] != r {
            r = self.parent[r];
        }
        // path compression
        let mut cur = x;
        while self.parent[cur] != r {
            let next = self.parent[cur];
            self.parent[cur] = r;
            cur = next;
        }
        r
    }
    fn union(&mut self, a: usize, b: usize) {
        let ra = self.find(a);
        let rb = self.find(b);
        if ra == rb {
            return;
        }
        if self.rank[ra] < self.rank[rb] {
            self.parent[ra] = rb;
        } else if self.rank[ra] > self.rank[rb] {
            self.parent[rb] = ra;
        } else {
            self.parent[rb] = ra;
            self.rank[ra] += 1;
        }
    }
}

pub fn detect_communities(
    edges: &[GraphEdge],
    num_nodes: usize,
    node_ids: &[i64],
) -> Vec<EntityCommunity> {
    let _ = num_nodes; // kept for API compatibility; n derived from node_ids
    let n = node_ids.len();
    if n == 0 {
        return Vec::new();
    }
    let mut idx: HashMap<i64, usize> = HashMap::with_capacity(n);
    for (i, &id) in node_ids.iter().enumerate() {
        idx.entry(id).or_insert(i);
    }
    let mut dsu = DSU::new(n);
    for e in edges {
        if let (Some(&u), Some(&v)) = (idx.get(&e.a), idx.get(&e.b)) {
            if u != v {
                dsu.union(u, v);
            }
        }
    }
    let mut groups: HashMap<usize, Vec<i64>> = HashMap::new();
    for (i, &id) in node_ids.iter().enumerate() {
        let r = dsu.find(i);
        groups.entry(r).or_default().push(id);
    }
    let mut comms: Vec<Vec<i64>> = groups.into_values().collect();
    // Drop singletons.
    comms.retain(|c| c.len() >= 2);
    // Sort each community's ids for determinism.
    for c in comms.iter_mut() {
        c.sort_unstable();
    }
    // Largest first; tie-break by smallest id for determinism.
    comms.sort_by(|a, b| {
        b.len()
            .cmp(&a.len())
            .then_with(|| a.first().cmp(&b.first()))
    });
    comms
        .into_iter()
        .enumerate()
        .map(|(i, ids)| EntityCommunity {
            community_id: (i as i64) + 1,
            size: ids.len(),
            entity_ids: ids,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Brandes' betweenness centrality (unweighted)
// ---------------------------------------------------------------------------

pub fn betweenness_centrality(edges: &[GraphEdge], node_ids: &[i64]) -> Vec<EntityBetweenness> {
    let n = node_ids.len();
    if n < 3 {
        return Vec::new();
    }
    let (_idx, adj) = build_adjacency(edges, node_ids);
    let mut cb = vec![0f64; n];

    for s in 0..n {
        // Single-source shortest paths via BFS
        let mut stack: Vec<usize> = Vec::with_capacity(n);
        let mut preds: Vec<Vec<usize>> = vec![Vec::new(); n];
        let mut sigma = vec![0f64; n];
        sigma[s] = 1.0;
        let mut dist: Vec<i64> = vec![-1; n];
        dist[s] = 0;
        let mut queue: VecDeque<usize> = VecDeque::new();
        queue.push_back(s);

        while let Some(v) = queue.pop_front() {
            stack.push(v);
            for &w in &adj[v] {
                if dist[w] < 0 {
                    dist[w] = dist[v] + 1;
                    queue.push_back(w);
                }
                if dist[w] == dist[v] + 1 {
                    sigma[w] += sigma[v];
                    preds[w].push(v);
                }
            }
        }

        // Accumulation
        let mut delta = vec![0f64; n];
        while let Some(w) = stack.pop() {
            for &v in &preds[w] {
                if sigma[w] > 0.0 {
                    delta[v] += (sigma[v] / sigma[w]) * (1.0 + delta[w]);
                }
            }
            if w != s {
                cb[w] += delta[w];
            }
        }
    }

    // Undirected graphs: each pair counted twice -> divide by 2.
    node_ids
        .iter()
        .enumerate()
        .map(|(i, &id)| EntityBetweenness {
            entity_id: id,
            betweenness: cb[i] / 2.0,
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Local clustering coefficient
// ---------------------------------------------------------------------------

pub fn clustering_coefficient(edges: &[GraphEdge], node_ids: &[i64]) -> Vec<(i64, f64)> {
    let n = node_ids.len();
    if n == 0 {
        return Vec::new();
    }
    let (_idx, adj) = build_adjacency(edges, node_ids);
    let neighbor_sets: Vec<HashSet<usize>> =
        adj.iter().map(|v| v.iter().copied().collect()).collect();

    let mut out = Vec::with_capacity(n);
    for (i, &id) in node_ids.iter().enumerate() {
        let k = adj[i].len();
        if k < 2 {
            out.push((id, 0.0));
            continue;
        }
        let neighbors = &adj[i];
        let mut links = 0usize;
        for a in 0..neighbors.len() {
            let na = neighbors[a];
            for &nb in neighbors.iter().skip(a + 1) {
                if neighbor_sets[na].contains(&nb) {
                    links += 1;
                }
            }
        }
        let possible = k * (k - 1) / 2;
        out.push((id, links as f64 / possible as f64));
    }
    out
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn e(a: i64, b: i64) -> GraphEdge {
        GraphEdge { a, b, weight: 1.0 }
    }

    #[test]
    fn triangle_graph() {
        let nodes = vec![1, 2, 3];
        let edges = vec![e(1, 2), e(2, 3), e(1, 3)];

        let comms = detect_communities(&edges, nodes.len(), &nodes);
        assert_eq!(comms.len(), 1);
        assert_eq!(comms[0].size, 3);
        assert_eq!(comms[0].community_id, 1);

        let bc = betweenness_centrality(&edges, &nodes);
        assert_eq!(bc.len(), 3);
        for b in &bc {
            assert!(
                b.betweenness.abs() < 1e-9,
                "expected 0 betweenness, got {}",
                b.betweenness
            );
        }

        let cc = clustering_coefficient(&edges, &nodes);
        for (_id, c) in &cc {
            assert!((c - 1.0).abs() < 1e-9, "expected 1.0 clustering, got {}", c);
        }
    }

    #[test]
    fn path_graph() {
        let nodes = vec![1, 2, 3, 4, 5];
        let edges = vec![e(1, 2), e(2, 3), e(3, 4), e(4, 5)];

        let comms = detect_communities(&edges, nodes.len(), &nodes);
        assert_eq!(comms.len(), 1);
        assert_eq!(comms[0].size, 5);

        let bc = betweenness_centrality(&edges, &nodes);
        let map: HashMap<i64, f64> = bc.iter().map(|b| (b.entity_id, b.betweenness)).collect();
        // Middle node 3 should have the highest betweenness.
        let middle = map[&3];
        assert!(middle > map[&2]);
        assert!(middle > map[&4]);
        assert!(middle > map[&1]);
        assert!(middle > map[&5]);
        assert!((map[&1]).abs() < 1e-9);
        assert!((map[&5]).abs() < 1e-9);

        let cc = clustering_coefficient(&edges, &nodes);
        for (_id, c) in &cc {
            assert!(c.abs() < 1e-9);
        }
    }

    #[test]
    fn two_disconnected_triangles() {
        let nodes = vec![1, 2, 3, 4, 5, 6];
        let edges = vec![e(1, 2), e(2, 3), e(1, 3), e(4, 5), e(5, 6), e(4, 6)];
        let comms = detect_communities(&edges, nodes.len(), &nodes);
        assert_eq!(comms.len(), 2);
        assert_eq!(comms[0].size, 3);
        assert_eq!(comms[1].size, 3);
        assert_eq!(comms[0].community_id, 1);
        assert_eq!(comms[1].community_id, 2);
    }

    #[test]
    fn star_graph() {
        // Center=1, leaves=2,3,4,5
        let nodes = vec![1, 2, 3, 4, 5];
        let edges = vec![e(1, 2), e(1, 3), e(1, 4), e(1, 5)];
        let bc = betweenness_centrality(&edges, &nodes);
        let map: HashMap<i64, f64> = bc.iter().map(|b| (b.entity_id, b.betweenness)).collect();
        let center = map[&1];
        for leaf in [2, 3, 4, 5] {
            assert!(map[&leaf].abs() < 1e-9, "leaf {} should be 0", leaf);
        }
        assert!(center > 0.0);

        let cc = clustering_coefficient(&edges, &nodes);
        let cc_map: HashMap<i64, f64> = cc.into_iter().collect();
        assert!(cc_map[&1].abs() < 1e-9);
    }
}
