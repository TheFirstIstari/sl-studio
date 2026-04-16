use crate::extractors::{Deconstructor, ExtractionResult, ExtractorConfig};
use crate::inference::llama::{LlamaConfig, LlamaModel};
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;
use tracing::{error, info, warn};

#[derive(Error, Debug)]
pub enum ReasonerError {
    #[error("Extraction error: {0}")]
    ExtractionError(String),
    #[error("LLM error: {0}")]
    LlmError(String),
    #[error("No model loaded")]
    NoModel,
    #[error("Model not configured")]
    ModelNotConfigured,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fact {
    pub source: String,
    pub date: Option<String>,
    pub summary: String,
    pub fact_type: String,
    pub crime: Option<String>,
    pub severity: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub filename: String,
    pub facts: Vec<Fact>,
    pub raw_response: String,
    pub tokens_used: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasonerConfig {
    pub model_path: String,
    pub context_size: u32,
    pub gpu_layers: i32,
    pub temperature: f32,
    pub max_tokens: u32,
    pub max_chars_per_chunk: usize,
    pub chunk_overlap: usize,
    pub batch_size: usize,
}

impl Default for ReasonerConfig {
    fn default() -> Self {
        ReasonerConfig {
            model_path: String::new(),
            context_size: 16384,
            gpu_layers: 0,
            temperature: 0.0,
            max_tokens: 2000,
            max_chars_per_chunk: 20000,
            chunk_overlap: 2000,
            batch_size: 24,
        }
    }
}

pub struct Reasoner {
    deconstructor: Deconstructor,
    model: Option<LlamaModel>,
    config: ReasonerConfig,
    system_prompt: String,
}

impl Reasoner {
    pub fn new(config: ReasonerConfig) -> Result<Self, ReasonerError> {
        let extractor_config = ExtractorConfig::default();
        let deconstructor = Deconstructor::new(extractor_config)
            .map_err(|e| ReasonerError::ExtractionError(e.to_string()))?;

        let system_prompt = Self::default_system_prompt();

        let model = if !config.model_path.is_empty() {
            let llama_config = LlamaConfig {
                model_path: config.model_path.clone(),
                context_size: config.context_size,
                gpu_layers: config.gpu_layers,
                temperature: config.temperature,
                max_tokens: config.max_tokens,
                repeat_penalty: 1.1,
                use_kv_cache: true,
                prompt_cache: None,
            };

            let mut model = LlamaModel::new(llama_config);
            model
                .load()
                .map_err(|e| ReasonerError::LlmError(e.to_string()))?;

            Some(model)
        } else {
            None
        };

        info!(
            "Reasoner initialized with model: {}",
            !config.model_path.is_empty()
        );

        Ok(Reasoner {
            deconstructor,
            model,
            config,
            system_prompt,
        })
    }

    pub fn load_model(&mut self, model_path: &str) -> Result<(), ReasonerError> {
        if let Some(ref mut model) = self.model {
            model.unload();
        }

        let llama_config = LlamaConfig {
            model_path: model_path.to_string(),
            context_size: self.config.context_size,
            gpu_layers: self.config.gpu_layers,
            temperature: self.config.temperature,
            max_tokens: self.config.max_tokens,
            repeat_penalty: 1.1,
            use_kv_cache: true,
            prompt_cache: None,
        };

        let mut model = LlamaModel::new(llama_config);
        model
            .load()
            .map_err(|e| ReasonerError::LlmError(e.to_string()))?;

        self.model = Some(model);
        self.config.model_path = model_path.to_string();

        info!("Model loaded: {}", model_path);
        Ok(())
    }

    pub fn is_model_loaded(&self) -> bool {
        self.model.as_ref().map(|m| m.is_loaded()).unwrap_or(false)
    }

    pub fn get_config(&self) -> ReasonerConfig {
        self.config.clone()
    }

    pub fn analyze_file(&self, path: &Path) -> Result<AnalysisResult, ReasonerError> {
        let model = self.model.as_ref().ok_or(ReasonerError::NoModel)?;

        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();

        info!("Analyzing file: {}", filename);

        let extraction = match self.deconstructor.extract(path) {
            Ok(extraction) => extraction,
            Err(e) => {
                error!("Extraction failed for {}: {}", filename, e);
                return Err(ReasonerError::ExtractionError(e.to_string()));
            }
        };

        if extraction.char_count == 0 {
            warn!("No text extracted from: {}", filename);
            return Ok(AnalysisResult {
                filename,
                facts: vec![],
                raw_response: "No text content found".to_string(),
                tokens_used: 0,
            });
        }

        let chunks = self.chunk_text(&extraction.text, extraction.source_file.clone());

        let mut all_facts = Vec::new();
        let mut raw_responses = Vec::new();

        // Process chunks sequentially to stay within context limits
        // Parallel processing would require separate model instances
        info!("Processing {} chunks for {}", chunks.len(), filename);
        for (i, chunk) in chunks.iter().enumerate() {
            let prompt = self.build_prompt(&chunk.source_file, &chunk.text);
            match model.generate_structured(&prompt) {
                Ok(response) => {
                    raw_responses.push(response.clone());
                    // Debug: log the raw LLM output
                    info!(
                        "LLM raw response (first 500 chars): {:?}",
                        &response.text[..response.text.len().min(500)]
                    );
                    let facts = self.parse_facts(&response.text);
                    info!("Parsed {} facts from response", facts.len());
                    all_facts.extend(facts);
                    info!(
                        "Processed chunk {}/{} for {}",
                        i + 1,
                        chunks.len(),
                        filename
                    );
                }
                Err(e) => {
                    error!(
                        "Failed to process chunk {}/{} for {}: {}",
                        i + 1,
                        chunks.len(),
                        filename,
                        e
                    );
                }
            }
        }

        let unique_facts = self.deduplicate_facts(all_facts);

        info!(
            "Extracted {} unique facts from {}",
            unique_facts.len(),
            filename
        );

        info!("Analysis complete for {}", filename);

        Ok(AnalysisResult {
            filename,
            facts: unique_facts,
            raw_response: raw_responses
                .iter()
                .map(|r| r.text.as_str())
                .collect::<Vec<_>>()
                .join("\n---\n"),
            tokens_used: 0,
        })
    }

    fn chunk_text(&self, text: &str, source_file: String) -> Vec<ExtractionResult> {
        if text.len() <= self.config.max_chars_per_chunk {
            return vec![ExtractionResult {
                text: text.to_string(),
                source_file,
                file_type: "chunk".to_string(),
                char_count: text.len(),
                is_partial: false,
            }];
        }

        let mut chunks = Vec::new();
        let mut start = 0;

        while start < text.len() {
            let end = std::cmp::min(start + self.config.max_chars_per_chunk, text.len());
            let chunk_text = text[start..end].to_string();

            let is_last = end >= text.len();

            chunks.push(ExtractionResult {
                text: chunk_text.clone(),
                source_file: if start == 0 {
                    source_file.clone()
                } else {
                    format!(
                        "{} (Part {})",
                        source_file,
                        start / self.config.max_chars_per_chunk + 1
                    )
                },
                file_type: "chunk".to_string(),
                char_count: chunk_text.len(),
                is_partial: !is_last,
            });

            if is_last {
                break;
            }

            start += self.config.max_chars_per_chunk - self.config.chunk_overlap;
        }

        chunks
    }

    fn build_prompt(&self, filename: &str, text: &str) -> String {
        format!(
            "{}<|im_start|>user\nFILE: {}\nDATA: {}<|im_end|>\n<|im_start|>assistant\n",
            self.system_prompt, filename, text
        )
    }

    fn parse_facts(&self, response: &str) -> Vec<Fact> {
        let mut facts = Vec::new();

        let items = Self::extract_json_objects(response);

        for item in items {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&item) {
                let fact = Fact {
                    source: json
                        .get("source")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    date: json
                        .get("date")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    summary: json
                        .get("summary")
                        .and_then(|v| v.as_str())
                        .unwrap_or("No summary")
                        .to_string(),
                    fact_type: json
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("General")
                        .to_string(),
                    crime: json
                        .get("crime")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    severity: json.get("severity").and_then(|v| v.as_i64()).unwrap_or(1) as i32,
                };
                facts.push(fact);
            }
        }

        facts
    }

    fn deduplicate_facts(&self, mut facts: Vec<Fact>) -> Vec<Fact> {
        use std::collections::HashSet;

        let mut seen = HashSet::new();
        facts.retain(|f| {
            let key = format!("{}:{}", f.source, f.summary);
            seen.insert(key)
        });

        facts
    }

    fn default_system_prompt() -> String {
        r#"<|im_start|>system
You are a forensic analyst. Extract facts from the provided text into JSON objects with the following keys:
- source: The document or file name
- date: Any dates mentioned (YYYY-MM-DD or description)
- summary: A brief summary of the fact
- type: Category (e.g., "Financial", "Legal", "Physical", "Digital", "Verbal")
- crime: Any crime type mentioned
- severity: 1-10 severity score

Output ONLY valid JSON array objects like: {"source": "...", "date": "...", "summary": "...", "type": "...", "crime": "...", "severity": 5}
Do not include any explanations or text before/after the JSON.<|im_end|>
"#.to_string()
    }

    fn extract_json_objects(text: &str) -> Vec<String> {
        let mut results = Vec::new();
        let mut depth = 0;
        let mut start = None;

        for (i, c) in text.char_indices() {
            match c {
                '{' => {
                    if depth == 0 {
                        start = Some(i);
                    }
                    depth += 1;
                }
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        if let Some(s) = start {
                            let obj = &text[s..=i];
                            if serde_json::from_str::<serde_json::Value>(obj).is_ok() {
                                results.push(obj.to_string());
                            }
                            start = None;
                        }
                    }
                }
                _ => {}
            }
        }

        results
    }

    /// Analyze pre-extracted text (for two-stage pipeline)
    pub fn analyze_text(
        &self,
        fingerprint: &str,
        filename: &str,
        text: &str,
    ) -> Result<AnalysisResult, ReasonerError> {
        let model = self.model.as_ref().ok_or(ReasonerError::NoModel)?;

        info!("Analyzing pre-extracted text for: {}", filename);

        if text.is_empty() {
            warn!("No text provided for: {}", filename);
            return Ok(AnalysisResult {
                filename: filename.to_string(),
                facts: vec![],
                raw_response: "No text content provided".to_string(),
                tokens_used: 0,
            });
        }

        let chunks = self.chunk_text(text, fingerprint.to_string());

        let mut all_facts = Vec::new();
        let mut raw_responses: Vec<crate::inference::llama::GenerationResult> = Vec::new();

        info!("Processing {} chunks for {}", chunks.len(), filename);
        for (i, chunk) in chunks.iter().enumerate() {
            let prompt = self.build_prompt(&chunk.source_file, &chunk.text);
            match model.generate_structured(&prompt) {
                Ok(response) => {
                    raw_responses.push(response.clone());
                    let facts = self.parse_facts(&response.text);
                    info!("Parsed {} facts from response", facts.len());
                    all_facts.extend(facts);
                    info!(
                        "Processed chunk {}/{} for {}",
                        i + 1,
                        chunks.len(),
                        filename
                    );
                }
                Err(e) => {
                    error!(
                        "Failed to process chunk {}/{} for {}: {}",
                        i + 1,
                        chunks.len(),
                        filename,
                        e
                    );
                }
            }
        }

        let unique_facts = self.deduplicate_facts(all_facts);

        info!(
            "Extracted {} unique facts from {}",
            unique_facts.len(),
            filename
        );

        Ok(AnalysisResult {
            filename: filename.to_string(),
            facts: unique_facts,
            raw_response: raw_responses
                .iter()
                .map(|r| r.text.as_str())
                .collect::<Vec<_>>()
                .join("\n---\n"),
            tokens_used: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reasoner_config_default() {
        let config = ReasonerConfig::default();
        assert_eq!(config.max_chars_per_chunk, 20000);
        assert_eq!(config.chunk_overlap, 2000);
    }

    #[test]
    fn test_extract_json_objects() {
        let text = r#"{"source": "file.txt", "summary": "test"}"#;
        let results = Reasoner::extract_json_objects(text);
        assert_eq!(results.len(), 1);
    }
}
