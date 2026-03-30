use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::time::Duration;

// Mock structures for benchmarking
#[derive(Clone)]
struct MockFact {
    id: i64,
    fingerprint: String,
    filename: String,
    category: Option<String>,
    severity: i32,
    summary: String,
}

fn create_test_facts(n: usize) -> Vec<MockFact> {
    (0..n)
        .map(|i| MockFact {
            id: i as i64,
            fingerprint: format!("fp_{}", i),
            filename: format!("document_{}.pdf", i),
            category: Some(match i % 5 {
                0 => "Financial".to_string(),
                1 => "Legal".to_string(),
                2 => "Digital".to_string(),
                3 => "Physical".to_string(),
                _ => "Verbal".to_string(),
            }),
            severity: (i % 10) as i32 + 1,
            summary: format!("Test fact number {} with some longer text content", i),
        })
        .collect()
}

fn benchmark_parse_search_query(c: &mut Criterion) {
    let mut group = c.benchmark_group("parse_search_query");
    
    let queries = vec![
        "simple search",
        "financial AND fraud",
        "\"exact phrase\" OR keyword",
        "NOT suspicious AND (financial OR money)",
        "A very long search query with multiple AND OR NOT operators and special characters",
    ];
    
    for query in queries {
        group.bench_function(
            BenchmarkId::new("query", query.len()),
            |b| b.iter(|| parse_search_query_blackbox(black_box(query)))
        );
    }
    
    group.finish();
}

fn parse_search_query_blackbox(input: &str) -> String {
    let mut result = String::with_capacity(input.len() + 64);
    let mut in_phrase = false;
    let mut pos = 0;
    let chars: Vec<char> = input.chars().collect();

    while pos < chars.len() {
        let c = chars[pos];
        
        if c == '"' {
            result.push('"');
            in_phrase = !in_phrase;
            pos += 1;
        } else if c == ' ' && !in_phrase {
            if !result.ends_with(' ') {
                result.push(' ');
            }
            pos += 1;
        } else if c == '(' || c == ')' {
            result.push(c);
            pos += 1;
        } else if c.eq_ignore_ascii_case(&'A') && result.ends_with(' ') && pos + 2 < chars.len() {
            if chars[pos + 1].eq_ignore_ascii_case(&'N') && chars[pos + 2].eq_ignore_ascii_case(&'D') {
                result.push_str("AND ");
                pos += 3;
                continue;
            }
            result.push(c);
            pos += 1;
        } else if c.eq_ignore_ascii_case(&'O') && result.ends_with(' ') && pos + 1 < chars.len() {
            if chars[pos + 1].eq_ignore_ascii_case(&'R') {
                result.push_str("OR ");
                pos += 2;
                continue;
            }
            result.push(c);
            pos += 1;
        } else if c.eq_ignore_ascii_case(&'N') && result.ends_with(' ') && pos + 2 < chars.len() {
            if chars[pos + 1].eq_ignore_ascii_case(&'O') && chars[pos + 2].eq_ignore_ascii_case(&'T') {
                result.push_str("NOT ");
                pos += 3;
                continue;
            }
            result.push(c);
            pos += 1;
        } else {
            result.push(c);
            pos += 1;
        }
    }

    result.trim().to_string()
}

fn benchmark_entity_overlap(c: &mut Criterion) {
    use std::collections::{HashMap, HashSet};
    
    let mut group = c.benchmark_group("entity_overlap");
    group.measurement_time(Duration::from_secs(5));
    
    // Test with different sizes
    for size in [10, 50, 100, 200].iter() {
        group.bench_with_input(
            BenchmarkId::new("hashset_overlap", size),
            size,
            |b, &size| {
                let entities1: HashSet<String> = (0..size).map(|i| format!("entity_{}", i)).collect();
                let entities2: HashSet<String> = (size/2..size + size/2).map(|i| format!("entity_{}", i)).collect();
                
                b.iter(|| {
                    let overlap = entities1.intersection(&entities2).count();
                    black_box(overlap);
                });
            }
        );
        
        group.bench_with_input(
            BenchmarkId::new("vec_overlap", size),
            size,
            |b, &size| {
                let entities1: Vec<String> = (0..size).map(|i| format!("entity_{}", i)).collect();
                let entities2: Vec<String> = (size/2..size + size/2).map(|i| format!("entity_{}", i)).collect();
                
                b.iter(|| {
                    let overlap = entities1.iter().filter(|e| entities2.contains(e)).count();
                    black_box(overlap);
                });
            }
        );
    }
    
    group.finish();
}

fn benchmark_string_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("string_operations");
    
    // Benchmark String formatting
    group.bench_function("format_fingerprint", |b| {
        b.iter(|| {
            let s = format!("fp_{}", black_box(12345));
            black_box(s);
        });
    });
    
    // Benchmark string clone
    group.bench_function("string_clone", |b| {
        let original = String::from("This is a test string with some content");
        b.iter(|| {
            let cloned = original.clone();
            black_box(cloned);
        });
    });
    
    // Benchmark to_lowercase
    group.bench_function("to_lowercase", |b| {
        let text = "Financial Fraud Case DOCUMENT with Mixed Case TEXT";
        b.iter(|| {
            let lower = text.to_lowercase();
            black_box(lower);
        });
    });
    
    // Benchmark contains
    group.bench_function("contains_check", |b| {
        let haystack = "This is a long text that contains various keywords like fraud, money, financial transactions";
        b.iter(|| {
            let found = haystack.contains(black_box("fraud"));
            black_box(found);
        });
    });
    
    group.finish();
}

fn benchmark_hashing(c: &mut Criterion) {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    
    let mut group = c.benchmark_group("hashing");
    
    group.bench_function("hash_string", |b| {
        let s = "test_string_for_hashing_with_some_content";
        b.iter(|| {
            let mut hasher = DefaultHasher::new();
            s.hash(&mut hasher);
            let hash = hasher.finish();
            black_box(hash);
        });
    });
    
    group.bench_function("hash_multiple_strings", |b| {
        let strings: Vec<String> = (0..100).map(|i| format!("string_{}_with_some_content", i)).collect();
        b.iter(|| {
            let mut hasher = DefaultHasher::new();
            for s in &strings {
                s.hash(&mut hasher);
            }
            let hash = hasher.finish();
            black_box(hash);
        });
    });
    
    group.finish();
}

fn benchmark_collections(c: &mut Criterion) {
    use std::collections::HashMap;
    
    let mut group = c.benchmark_group("collections");
    
    group.bench_function("hashmap_insert_1000", |b| {
        b.iter(|| {
            let mut map = HashMap::new();
            for i in 0..1000 {
                map.insert(format!("key_{}", i), format!("value_{}", i));
            }
            black_box(map);
        });
    });
    
    group.bench_function("hashmap_lookup_1000", |b| {
        let mut map = HashMap::new();
        for i in 0..1000 {
            map.insert(format!("key_{}", i), format!("value_{}", i));
        }
        b.iter(|| {
            for i in 0..1000 {
                let _ = map.get(&format!("key_{}", i % 1000));
            }
        });
    });
    
    group.bench_function("vec_sort_severity", |b| {
        let mut facts = create_test_facts(1000);
        b.iter(|| {
            facts.sort_by(|a, b| b.severity.cmp(&a.severity));
            black_box(&facts);
        });
    });
    
    group.finish();
}

fn benchmark_category_icon(c: &mut Criterion) {
    let mut group = c.benchmark_group("category_icon");
    
    // Test with caching
    group.bench_function("icon_lookup_with_cache", |b| {
        use std::collections::HashMap;
        let mut cache: HashMap<Option<String>, String> = HashMap::new();
        cache.insert(Some("Financial".to_string()), "dollar".to_string());
        cache.insert(Some("Legal".to_string()), "scale".to_string());
        cache.insert(Some("Digital".to_string()), "laptop".to_string());
        cache.insert(None, "file".to_string());
        
        let categories = vec![
            Some("Financial".to_string()),
            Some("Legal".to_string()),
            Some("Digital".to_string()),
            None,
            Some("Unknown".to_string()),
        ];
        
        b.iter(|| {
            for cat in &categories {
                let icon = cache.get(cat).unwrap_or(&"file".to_string()).clone();
                black_box(icon);
            }
        });
    });
    
    // Test without caching (simulating multiple calls)
    group.bench_function("icon_lookup_no_cache", |b| {
        fn get_category_icon(category: &Option<String>) -> &str {
            match category.as_deref() {
                Some("Financial") => "dollar",
                Some("Legal") => "scale",
                Some("Digital") => "laptop",
                Some("Physical") => "map-pin",
                Some("Verbal") => "mic",
                _ => "file",
            }
        }
        
        let categories = vec![
            Some("Financial".to_string()),
            Some("Legal".to_string()),
            Some("Digital".to_string()),
            None,
            Some("Unknown".to_string()),
        ];
        
        b.iter(|| {
            for cat in &categories {
                let icon = get_category_icon(cat);
                black_box(icon);
            }
        });
    });
    
    group.finish();
}

criterion_group!(
    name = benches;
    config = Criterion::default()
        .sample_size(1000)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2));
    targets = benchmark_parse_search_query, benchmark_entity_overlap, benchmark_string_operations, benchmark_hashing, benchmark_collections, benchmark_category_icon
);

criterion_main!(benches);
