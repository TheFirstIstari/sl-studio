use crate::inference::llama::{LlamaConfig, LlamaModel};
use rand::seq::SliceRandom;
use rand::SeedableRng;
use serde::{Deserialize, Serialize};
use std::time::Instant;
use thiserror::Error;
use tracing::{info, warn};

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
        let parsed: serde_json::Value =
            serde_json::from_str(json_str).map_err(|e| PipelineError::JsonError(e.to_string()))?;

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
                    source_quote: v
                        .get("source_quote")
                        .and_then(|s| s.as_str())
                        .map(String::from)
                        .unwrap_or_default(),
                })
            })
            .collect();

        Ok(facts)
    }
}

pub struct PipelineRunner {
    model: Option<LlamaModel>,
    model_config: LlamaConfig,
}

impl PipelineRunner {
    pub fn new(model_path: Option<String>) -> Self {
        let model_config = LlamaConfig {
            model_path: model_path.unwrap_or_default(),
            context_size: 16384,
            gpu_layers: 0,
            temperature: 0.1,
            max_tokens: 4000,
            repeat_penalty: 1.1,
            use_kv_cache: true,
            prompt_cache: None,
            n_threads: num_cpus::get() as u32,
        };

        PipelineRunner {
            model: None,
            model_config,
        }
    }

    pub fn load_model(&mut self, model_path: &str) -> Result<(), PipelineError> {
        let mut config = self.model_config.clone();
        config.model_path = model_path.to_string();

        let mut model = LlamaModel::new(config);
        model
            .load()
            .map_err(|e| PipelineError::LlmError(e.to_string()))?;

        self.model = Some(model);
        info!("PipelineRunner: Model loaded");
        Ok(())
    }

    pub fn is_model_loaded(&self) -> bool {
        self.model.as_ref().map(|m| m.is_loaded()).unwrap_or(false)
    }

    pub fn run_pipeline(
        &self,
        pipeline: &Pipeline,
        text: &str,
        source_name: &str,
    ) -> PipelineRunResult {
        let start_time = Instant::now();
        let mut results = Vec::new();

        let sampled_text =
            self.sample_text(text, pipeline.passes.first().and_then(|p| p.sample_size));

        for pass in &pipeline.passes {
            let pass_result = self.run_pass(pass, &sampled_text, source_name);
            results.push(pass_result);
        }

        let total_time_ms = start_time.elapsed().as_millis() as u64;
        let success = results.iter().all(|r| r.success);

        PipelineRunResult {
            pipeline_id: pipeline.id.clone(),
            pipeline_name: pipeline.name.clone(),
            results,
            total_time_ms,
            success,
        }
    }

    fn run_pass(&self, pass: &PipelinePass, text: &str, source_name: &str) -> PipelineResult {
        let start_time = Instant::now();

        let prompt = Self::build_prompt(&pass.prompt_template, source_name, text);

        let output = match &self.model {
            Some(model) => match model.generate_structured(&prompt) {
                Ok(response) => response,
                Err(e) => {
                    warn!("LLM generation failed: {}", e);
                    return PipelineResult {
                        pass_name: pass.name.clone(),
                        output: String::new(),
                        facts_extracted: 0,
                        entities_extracted: 0,
                        processing_time_ms: start_time.elapsed().as_millis() as u64,
                        success: false,
                        error: Some(e.to_string()),
                    };
                }
            },
            None => {
                return PipelineResult {
                    pass_name: pass.name.clone(),
                    output: String::new(),
                    facts_extracted: 0,
                    entities_extracted: 0,
                    processing_time_ms: start_time.elapsed().as_millis() as u64,
                    success: false,
                    error: Some("No model loaded".to_string()),
                };
            }
        };

        let facts = Fact::from_json(&output.text).unwrap_or_default();
        let entities_extracted = Self::count_entities_in_output(&output.text);

        PipelineResult {
            pass_name: pass.name.clone(),
            output: output.text,
            facts_extracted: facts.len(),
            entities_extracted,
            processing_time_ms: start_time.elapsed().as_millis() as u64,
            success: true,
            error: None,
        }
    }

    fn sample_text(&self, text: &str, sample_size: Option<usize>) -> String {
        let size = match sample_size {
            Some(s) => s,
            None => return text.to_string(),
        };

        if text.len() <= size {
            return text.to_string();
        }

        let paragraphs: Vec<&str> = text
            .split("\n\n")
            .filter(|p| !p.trim().is_empty())
            .collect();

        if paragraphs.is_empty() {
            return text[..size].to_string();
        }

        let mut rng = rand::rngs::StdRng::from_entropy();
        let sample_count = size.min(paragraphs.len());
        let selected: Vec<_> = paragraphs
            .choose_multiple(&mut rng, sample_count)
            .cloned()
            .collect();

        selected.join("\n\n")
    }

    fn build_prompt(template: &str, source_name: &str, text: &str) -> String {
        format!(
            "{}<|im_start|>user\nSOURCE: {}\nTEXT: {}<|im_end|>\n<|im_start|>assistant\n",
            template, source_name, text
        )
    }

    fn count_entities_in_output(output: &str) -> usize {
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(output) {
            if let Some(arr) = parsed.get("entities").and_then(|e| e.as_array()) {
                return arr.len();
            }
            if let Some(arr) = parsed.get("patterns").and_then(|p| p.as_array()) {
                return arr.len();
            }
            if let Some(arr) = parsed.as_array() {
                return arr.len();
            }
        }
        0
    }
}

pub fn get_builtin_pipelines() -> Vec<Pipeline> {
    vec![Pipeline::basic_facts(), Pipeline::financial_crimes()]
}

pub fn get_pipeline_by_id(id: &str) -> Option<Pipeline> {
    match id {
        "basic-facts" => Some(Pipeline::basic_facts()),
        "financial-crimes" => Some(Pipeline::financial_crimes()),
        _ => None,
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
