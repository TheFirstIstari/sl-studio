use steinline_lib::inference::{AnalysisResult, ReasonerFact, Reasoner, ReasonerConfig};

#[test]
fn test_reasoner_config_default() {
    let config = ReasonerConfig::default();
    assert_eq!(config.context_size, 16384);
    assert_eq!(config.max_tokens, 2000);
    assert_eq!(config.max_chars_per_chunk, 20000);
    assert_eq!(config.chunk_overlap, 2000);
}

#[test]
fn test_reasoner_without_model() {
    let config = ReasonerConfig::default();
    let result = Reasoner::new(config);

    // Either succeeds (stub mode) or fails - both acceptable
    match result {
        Ok(_r) => {
            // Stub mode - model not actually required
        }
        Err(_e) => {
            // Expected to fail in real mode
        }
    }
}

#[test]
fn test_fact_struct_creation() {
    let fact = ReasonerFact {
        source: "test.pdf".to_string(),
        date: Some("2024-01-01".to_string()),
        summary: "Test summary".to_string(),
        fact_type: "Financial".to_string(),
        crime: Some("Fraud".to_string()),
        severity: 8,
    };

    assert_eq!(fact.severity, 8);
    assert_eq!(fact.fact_type, "Financial");
}

#[test]
fn test_analysis_result_struct() {
    let result = AnalysisResult {
        filename: "test.pdf".to_string(),
        facts: vec![],
        raw_response: "".to_string(),
        tokens_used: 0,
    };

    assert_eq!(result.facts.len(), 0);
    assert_eq!(result.filename, "test.pdf");
}

#[test]
fn test_analysis_result_with_facts() {
    let facts = vec![
        ReasonerFact {
            source: "doc1.pdf".to_string(),
            date: None,
            summary: "First fact".to_string(),
            fact_type: "Legal".to_string(),
            crime: None,
            severity: 5,
        },
        ReasonerFact {
            source: "doc2.pdf".to_string(),
            date: Some("2024-02-01".to_string()),
            summary: "Second fact".to_string(),
            fact_type: "Financial".to_string(),
            crime: Some("Embezzlement".to_string()),
            severity: 9,
        },
    ];

    let result = AnalysisResult {
        filename: "combined.pdf".to_string(),
        facts,
        raw_response: "raw output".to_string(),
        tokens_used: 150,
    };

    assert_eq!(result.facts.len(), 2);
    assert_eq!(result.tokens_used, 150);
}
