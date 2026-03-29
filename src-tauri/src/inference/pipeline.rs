use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PipelineError {
    #[error("LLM error: {0}")]
    LlmError(String),
    #[error("JSON parse error: {0}")]
    JsonError(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("No model loaded")]
    NoModel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelinePass {
    pub name: String,
    pub description: String,
    pub prompt_template: String,
    pub output_schema: Option<String>,
    pub max_tokens: u32,
    pub temperature: f32,
    pub sample_size: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pipeline {
    pub id: String,
    pub name: String,
    pub description: String,
    pub passes: Vec<PipelinePass>,
    pub is_builtin: bool,
}

impl Pipeline {
    pub fn basic_facts() -> Self {
        Pipeline {
            id: "basic-facts".to_string(),
            name: "Basic Facts".to_string(),
            description: "Extract factual statements from text".to_string(),
            is_builtin: true,
            passes: vec![PipelinePass {
                name: "extract_facts".to_string(),
                description: "Extract key factual statements".to_string(),
                prompt_template: include_str!("prompts/basic_facts.txt").to_string(),
                output_schema: Some(include_str!("schemas/facts.json").to_string()),
                max_tokens: 4000,
                temperature: 0.1,
                sample_size: Some(100),
            }],
        }
    }

    pub fn financial_crimes() -> Self {
        Pipeline {
            id: "financial-crimes".to_string(),
            name: "Financial Crimes".to_string(),
            description: "Analyze financial documents for crimes".to_string(),
            is_builtin: true,
            passes: vec![
                PipelinePass {
                    name: "extract_entities".to_string(),
                    description: "Extract financial entities".to_string(),
                    prompt_template: include_str!("prompts/financial_entities.txt").to_string(),
                    output_schema: Some(include_str!("schemas/entities.json").to_string()),
                    max_tokens: 4000,
                    temperature: 0.1,
                    sample_size: Some(100),
                },
                PipelinePass {
                    name: "identify_patterns".to_string(),
                    description: "Identify suspicious patterns".to_string(),
                    prompt_template: include_str!("prompts/financial_patterns.txt").to_string(),
                    output_schema: Some(include_str!("schemas/patterns.json").to_string()),
                    max_tokens: 4000,
                    temperature: 0.2,
                    sample_size: Some(50),
                },
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineResult {
    pub pass_name: String,
    pub output: String,
    pub facts_extracted: usize,
    pub entities_extracted: usize,
    pub processing_time_ms: u64,
    pub success: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineRunResult {
    pub pipeline_id: String,
    pub pipeline_name: String,
    pub results: Vec<PipelineResult>,
    pub total_time_ms: u64,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fact {
    pub summary: String,
    pub category: Option<String>,
    pub severity: Option<i32>,
    pub confidence: f32,
    pub source_quote: String,
}

impl Fact {
    pub fn from_json(json_str: &str) -> Result<Vec<Fact>, PipelineError> {
        let parsed: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| PipelineError::JsonError(e.to_string()))?;

        let facts_array = if parsed.is_array() {
            parsed.as_array().unwrap().clone()
        } else if let Some(obj) = parsed.get("facts").and_then(|f| f.as_array()) {
            obj.clone()
        } else {
            return Ok(vec![]);
        };

        let facts: Vec<Fact> = facts_array
            .iter()
            .filter_map(|v| {
                Some(Fact {
                    summary: v.get("summary")?.as_str()?.to_string(),
                    category: v.get("category").and_then(|c| c.as_str()).map(String::from),
                    severity: v.get("severity").and_then(|s| s.as_i64()).map(|s| s as i32),
                    confidence: v.get("confidence").and_then(|c| c.as_f64()).unwrap_or(0.5) as f32,
                    source_quote: v.get("source_quote").and_then(|s| s.as_str()).map(String::from).unwrap_or_default(),
                })
            })
            .collect();

        Ok(facts)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_builtin() {
        let pipeline = Pipeline::basic_facts();
        assert_eq!(pipeline.id, "basic-facts");
        assert!(pipeline.is_builtin);
        assert!(!pipeline.passes.is_empty());
    }

    #[test]
    fn test_fact_parsing() {
        let json = r#"[
            {"summary": "Test fact", "category": "test", "confidence": 0.9, "source_quote": "quote"}
        ]"#;
        let facts = Fact::from_json(json).unwrap();
        assert_eq!(facts.len(), 1);
        assert_eq!(facts[0].summary, "Test fact");
    }
}
