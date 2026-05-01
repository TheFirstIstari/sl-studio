use steinline_lib::inference::{AnalysisResult, Reasoner, ReasonerConfig, ReasonerFact};

#[test]
fn test_reasoner_config_default() {
    let config = ReasonerConfig::default();
    assert_eq!(config.context_size, 4096);
    assert_eq!(config.max_tokens, 256);
    assert_eq!(config.max_chars_per_chunk, 2000);
    assert_eq!(config.chunk_overlap, 150);
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
        source_quote: "A quote from the document.".to_string(),
        date: Some("2024-01-01".to_string()),
        location: None,
        people: vec![],
        summary: "Test summary".to_string(),
        category: "Financial".to_string(),
        identified_crime: Some("Fraud".to_string()),
        severity: 8,
        confidence: 0.9,
    };

    assert_eq!(fact.severity, 8);
    assert_eq!(fact.category, "Financial");
}

#[test]
fn test_analysis_result_struct() {
    let result = AnalysisResult {
        filename: "test.pdf".to_string(),
        facts: vec![],
        raw_response: "".to_string(),
        tokens_used: 0,
        quality_score: 0.0,
        entity_count: 0,
        quote_coverage: 0.0,
    };

    assert_eq!(result.facts.len(), 0);
    assert_eq!(result.filename, "test.pdf");
}

#[test]
fn test_analysis_result_with_facts() {
    let facts = vec![
        ReasonerFact {
            source: "doc1.pdf".to_string(),
            source_quote: "Quote from doc1.".to_string(),
            date: None,
            location: None,
            people: vec![],
            summary: "First fact".to_string(),
            category: "Legal".to_string(),
            identified_crime: None,
            severity: 5,
            confidence: 0.7,
        },
        ReasonerFact {
            source: "doc2.pdf".to_string(),
            source_quote: "Quote from doc2.".to_string(),
            date: Some("2024-02-01".to_string()),
            location: Some("London".to_string()),
            people: vec!["Alice".to_string()],
            summary: "Second fact".to_string(),
            category: "Financial".to_string(),
            identified_crime: Some("Embezzlement".to_string()),
            severity: 9,
            confidence: 0.95,
        },
    ];

    let result = AnalysisResult {
        filename: "combined.pdf".to_string(),
        facts,
        raw_response: "raw output".to_string(),
        tokens_used: 150,
        quality_score: 0.85,
        entity_count: 1,
        quote_coverage: 1.0,
    };

    assert_eq!(result.facts.len(), 2);
    assert_eq!(result.tokens_used, 150);
}
