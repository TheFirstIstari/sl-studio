use crate::extractors::{Deconstructor, ExtractionResult, ExtractorConfig};
use crate::inference::llama::{LlamaConfig, LlamaModel};
use serde::{Deserialize, Serialize};
use std::path::Path;
use thiserror::Error;
use tracing::{error, info, warn};

/// Model family for prompt format selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ModelFamily {
    /// Llama 2 style: [INST] <<SYS>>...<[/SYS>>...[/INST]
    #[default]
    Llama2,
    /// Gemma 3 style: <start_of_turn>user...<end_of_turn>
    Gemma3,
    /// Mistral style (same as Llama 2)
    Mistral,
    /// Generic chatml style: <|im_start|>user...<|im_end|>
    ChatML,
}

impl ModelFamily {
    /// Detect model family from model filename
    pub fn from_filename(filename: &str) -> Self {
        let lower = filename.to_lowercase();
        if lower.contains("gemma-3") || lower.contains("gemma3") {
            ModelFamily::Gemma3
        } else if lower.contains("mistral") {
            ModelFamily::Mistral
        } else if lower.contains("llama-2") || lower.contains("llama2") {
            ModelFamily::Llama2
        } else {
            // Default to ChatML for unknown models
            ModelFamily::ChatML
        }
    }
}

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

/// Extracted fact from document analysis
/// Matches SPEC.md intelligence table schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fact {
    pub source: String,              // Source filename (required)
    pub source_quote: String,       // Exact supporting quote from document (required per FR-EVD-001)
    pub date: Option<String>,        // Associated date/time (optional)
    pub location: Option<String>,    // Location mentioned in fact (optional)
    pub people: Vec<String>,         // Related people/entities (optional)
    pub summary: String,             // Fact summary (required)
    pub category: String,            // Category: legal, financial, temporal, relationship, etc.
    pub identified_crime: Option<String>,  // Potential crime type if applicable
    pub severity: i32,               // Severity 1-5 (1=low, 5=critical)
    pub confidence: f32,             // Extraction confidence 0.0-1.0 (per FR-QUAL-001)
}

/// Result from LLM analysis
/// Includes quality metrics per FR-QUAL-001
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalysisResult {
    pub filename: String,
    pub facts: Vec<Fact>,
    pub raw_response: String,
    pub tokens_used: i32,
    pub quality_score: f32,          // Overall extraction quality 0.0-1.0
    pub entity_count: i32,           // Number of entities detected
    pub quote_coverage: f32,        // Percentage of facts with source quotes
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
    pub n_threads: u32,
    pub n_threads_batch: Option<u32>,  // For batch processing parallelism
    pub model_family: ModelFamily,      // For prompt format selection
}

impl Default for ReasonerConfig {
    fn default() -> Self {
        // Context MUST be 4096 - model's native context size
        ReasonerConfig {
            model_path: String::new(),
            context_size: 4096,
            gpu_layers: 32,
            temperature: 0.7,    // Recommended by model author
            max_tokens: 256,       // Keep output short to avoid quality issues
            // Max chars = ~700 tokens (4096 - 2000 for prompt - 1000 for output buffer)
            max_chars_per_chunk: 2000,  
            chunk_overlap: 150,
            batch_size: 4,
            n_threads: 4,
            n_threads_batch: Some(8),
            model_family: ModelFamily::default(),
        }
    }
}

pub struct Reasoner {
    deconstructor: Deconstructor,
    model: Option<LlamaModel>,
    config: ReasonerConfig,
    system_prompt: String,
    model_family: ModelFamily,
}

impl Reasoner {
    pub fn new(config: ReasonerConfig) -> Result<Self, ReasonerError> {
        let extractor_config = ExtractorConfig::default();
        let deconstructor = Deconstructor::new(extractor_config)
            .map_err(|e| ReasonerError::ExtractionError(e.to_string()))?;

        // Detect model family from config
        let model_family = ModelFamily::from_filename(&config.model_path);
        let system_prompt = Self::system_prompt_for_family(model_family);

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
                n_threads: config.n_threads,
                n_threads_batch: config.n_threads_batch.unwrap_or(config.n_threads * 2),
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

        let model_family = ModelFamily::from_filename(&config.model_path);
        
        Ok(Reasoner {
            deconstructor,
            model,
            config,
            system_prompt,
            model_family,
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
            n_threads: self.config.n_threads,
            n_threads_batch: self.config.n_threads_batch.unwrap_or(self.config.n_threads * 2),
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
                quality_score: 0.0,
                entity_count: 0,
                quote_coverage: 0.0,
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

        // Calculate quality metrics
        let quality_score = if unique_facts.is_empty() {
            0.0
        } else {
            unique_facts.iter().map(|f| f.confidence).sum::<f32>() / unique_facts.len() as f32
        };
        let entity_count: i32 = unique_facts.iter().map(|f| f.people.len() as i32).sum();
        let quote_coverage = if unique_facts.is_empty() {
            0.0
        } else {
            unique_facts.iter().filter(|f| !f.source_quote.is_empty()).count() as f32 / unique_facts.len() as f32
        };

        Ok(AnalysisResult {
            filename,
            facts: unique_facts,
            raw_response: raw_responses
                .iter()
                .map(|r| r.text.as_str())
                .collect::<Vec<_>>()
                .join("\n---\n"),
            tokens_used: 0,
            quality_score,
            entity_count,
            quote_coverage,
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
        match self.model_family {
            ModelFamily::Gemma3 => {
                // Gemma 3 uses <start_of_turn>user...<end_of_turn> format
                format!(
                    "<start_of_turn>user\n{}Extract key facts from FILE: {}\n\nDOCUMENT DATA:\n{}\n\nOutput a JSON array of facts with fields: source, date, summary, type, crime, severity.\n<end_of_turn>\n<start_of_turn>model\n",
                    self.system_prompt, filename, text
                )
            },
            ModelFamily::Llama2 | ModelFamily::Mistral => {
                // Llama 2 / Mistral use [INST] <<SYS>>...<[/SYS>> format
                format!(
                    "[INST] <<SYS>>\n{}<</SYS>>\n\nFILE: {}\nDATA: {}\n\nOutput JSON only: [/INST]",
                    self.system_prompt, filename, text
                )
            },
            ModelFamily::ChatML => {
                // Default ChatML format
                format!(
                    "{}<|im_start|>user\nFILE: {}\nDATA: {}<|im_end|>\n<|im_start|>assistant\n",
                    self.system_prompt, filename, text
                )
            }
        }
    }

    fn parse_facts(&self, response: &str) -> Vec<Fact> {
        let mut facts = Vec::new();

        let items = Self::extract_json_objects(response);

        for item in items {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&item) {
                // Parse people array
                let people: Vec<String> = json
                    .get("people")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|p| p.as_str())
                            .map(|s| s.to_string())
                            .collect()
                    })
                    .unwrap_or_default();

                let fact = Fact {
                    source: json
                        .get("source")
                        .and_then(|v| v.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                    source_quote: json
                        .get("source_quote")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    date: json
                        .get("date")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    location: json
                        .get("location")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    people,
                    summary: json
                        .get("summary")
                        .and_then(|v| v.as_str())
                        .unwrap_or("No summary")
                        .to_string(),
                    category: json
                        .get("category")
                        .and_then(|v| v.as_str())
                        .unwrap_or("other")
                        .to_string(),
                    identified_crime: json
                        .get("identified_crime")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string()),
                    severity: json.get("severity").and_then(|v| v.as_i64()).unwrap_or(1) as i32,
                    confidence: json.get("confidence").and_then(|v| v.as_f64()).unwrap_or(0.5) as f32,
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

    fn get_system_prompt(&self) -> String {
        Self::system_prompt_for_family(self.model_family)
    }

    fn system_prompt_for_family(family: ModelFamily) -> String {
        match family {
            ModelFamily::Gemma3 => {
                r#"You are a forensic document analyst for law enforcement. Extract structured facts from documents.
For each fact, extract these EXACT fields:
- source: filename
- source_quote: EXACT text from document that supports this fact (REQUIRED per forensic standards)
- date: date/time mentioned (YYYY-MM-DD format, or null if not found)
- location: location mentioned (city, address, or null)
- people: array of names mentioned in context of this fact
- summary: brief description of the fact
- category: one of [legal, financial, temporal, relationship, communication, activity, other]
- identified_crime: crime type if applicable (fraud, theft, assault, corruption, etc.) or null
- severity: 1-5 (1=minor, 2=low, 3=medium, 4=high, 5=critical)
- confidence: 0.0-1.0 based on how well the source quote supports the fact

Output ONLY valid JSON array. No text before or after JSON.
Example: [{"source":"doc.pdf","source_quote":"signed on Jan 15 2024","date":"2024-01-15","location":null,"people":["John Smith"],"summary":"Contract signed","category":"legal","identified_crime":null,"severity":2,"confidence":0.9}]"#.to_string()
            },
            ModelFamily::Llama2 | ModelFamily::Mistral => {
                r#"You are a forensic document analyst for law enforcement. Extract structured facts from documents.
For each fact, extract these EXACT fields:
- source: filename
- source_quote: EXACT text from document that supports this fact (REQUIRED per forensic standards)
- date: date/time mentioned (YYYY-MM-DD format, or null if not found)
- location: location mentioned (city, address, or null)
- people: array of names mentioned in context of this fact
- summary: brief description of the fact
- category: one of [legal, financial, temporal, relationship, communication, activity, other]
- identified_crime: crime type if applicable (fraud, theft, assault, corruption, etc.) or null
- severity: 1-5 (1=minor, 2=low, 3=medium, 4=high, 5=critical)
- confidence: 0.0-1.0 based on how well the source quote supports the fact

Output ONLY valid JSON array. No text before or after JSON.
Example: [{"source":"doc.pdf","source_quote":"signed on Jan 15 2024","date":"2024-01-15","location":null,"people":["John Smith"],"summary":"Contract signed","category":"legal","identified_crime":null,"severity":2,"confidence":0.9}]"#.to_string()
            },
            ModelFamily::ChatML => {
                r#"You are a forensic document analyst for law enforcement. Extract structured facts from documents.
For each fact, extract these EXACT fields:
- source: filename
- source_quote: EXACT text from document that supports this fact (REQUIRED per forensic standards)
- date: date/time mentioned (YYYY-MM-DD format, or null if not found)
- location: location mentioned (city, address, or null)
- people: array of names mentioned in context of this fact
- summary: brief description of the fact
- category: one of [legal, financial, temporal, relationship, communication, activity, other]
- identified_crime: crime type if applicable (fraud, theft, assault, corruption, etc.) or null
- severity: 1-5 (1=minor, 2=low, 3=medium, 4=high, 5=critical)
- confidence: 0.0-1.0 based on how well the source quote supports the fact

Output ONLY valid JSON array. No text before or after JSON.
Example: [{"source":"doc.pdf","source_quote":"signed on Jan 15 2024","date":"2024-01-15","location":null,"people":["John Smith"],"summary":"Contract signed","category":"legal","identified_crime":null,"severity":2,"confidence":0.9}]"#.to_string()
            }
        }
    }

    fn default_system_prompt() -> String {
        // Legacy Llama 2 format (not used anymore)
        r#"[INST] <<SYS>>
You are a helpful forensic document analyst. Extract key facts from documents.
Output ONLY valid JSON array. Example: [{"source":"filename.pdf","summary":"key fact","type":"legal","severity":5}]
Do NOT include any text before or after the JSON.
<</SYS>>

Extract facts from this document. Output JSON only: [/INST]"#.to_string()
    }

    fn extract_json_objects(text: &str) -> Vec<String> {
        let text = text.trim();
        let mut results = Vec::new();
        
        // First, try to parse as a JSON array
        if let Ok(arr) = serde_json::from_str::<serde_json::Value>(text) {
            if let Some(items) = arr.as_array() {
                for item in items {
                    if let Some(obj_str) = item.as_object() {
                        if let Ok(obj_json) = serde_json::to_string(obj_str) {
                            results.push(obj_json);
                        }
                    } else if let Some(obj_str) = item.as_str() {
                        // Try parsing string as JSON
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(obj_str) {
                            if let Some(obj) = v.as_object() {
                                if let Ok(obj_json) = serde_json::to_string(obj) {
                                    results.push(obj_json);
                                }
                            }
                        } else {
                            // It's a plain string, wrap it
                            results.push(obj_str.to_string());
                        }
                    }
                }
                if !results.is_empty() {
                    return results;
                }
            }
        }
        
        // Fall back to extracting individual objects using depth tracking
        let mut depth = 0;
        let mut start = None;
        let mut in_array = false;

        for (i, c) in text.char_indices() {
            match c {
                '[' => {
                    if depth == 0 {
                        in_array = true;
                    }
                    depth += 1;
                }
                '{' => {
                    if depth == 0 {
                        start = Some(i);
                    }
                    depth += 1;
                }
                ']' => {
                    depth -= 1;
                    if depth == 0 {
                        in_array = false;
                    }
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
                quality_score: 0.0,
                entity_count: 0,
                quote_coverage: 0.0,
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
                    // Debug: show full response
                    let response_preview = &response.text[..response.text.len().min(1000)];
                    info!("LLM response (first 1000 chars): {:?}", response_preview);
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

        // Calculate quality metrics
        let quality_score = if unique_facts.is_empty() {
            0.0
        } else {
            unique_facts.iter().map(|f| f.confidence).sum::<f32>() / unique_facts.len() as f32
        };
        let entity_count: i32 = unique_facts.iter().map(|f| f.people.len() as i32).sum();
        let quote_coverage = if unique_facts.is_empty() {
            0.0
        } else {
            unique_facts.iter().filter(|f| !f.source_quote.is_empty()).count() as f32 / unique_facts.len() as f32
        };

        Ok(AnalysisResult {
            filename: filename.to_string(),
            facts: unique_facts,
            raw_response: raw_responses
                .iter()
                .map(|r| r.text.as_str())
                .collect::<Vec<_>>()
                .join("\n---\n"),
            tokens_used: 0,
            quality_score,
            entity_count,
            quote_coverage,
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
