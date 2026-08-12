// Inference modules for SL Studio

pub mod mlx_pipeline;
pub mod reasoner;

/// Return the set of built-in pipelines available out of the box.
pub fn get_builtin_pipelines() -> Vec<crate::Pipeline> {
    vec![
        crate::Pipeline {
            id: "default".to_string(),
            name: "Default Analysis Pipeline".to_string(),
            description: "Standard forensic document analysis pipeline".to_string(),
            is_builtin: true,
            passes: vec![
                crate::PipelinePass {
                    name: "text_extraction".to_string(),
                    description: "Extract text from document".to_string(),
                    prompt_template: "Extract all text from the following document:".to_string(),
                    output_schema: None,
                    max_tokens: 4096,
                    temperature: 0.0,
                    sample_size: None,
                },
                crate::PipelinePass {
                    name: "fact_extraction".to_string(),
                    description: "Identify facts and claims".to_string(),
                    prompt_template: "Extract all factual claims and statements from:".to_string(),
                    output_schema: None,
                    max_tokens: 2048,
                    temperature: 0.1,
                    sample_size: None,
                },
                crate::PipelinePass {
                    name: "entity_recognition".to_string(),
                    description: "Identify named entities".to_string(),
                    prompt_template: "Identify all named entities (people, organizations, locations, dates) from:".to_string(),
                    output_schema: None,
                    max_tokens: 2048,
                    temperature: 0.0,
                    sample_size: None,
                },
            ],
        },
        crate::Pipeline {
            id: "deep_analysis".to_string(),
            name: "Deep Forensic Analysis".to_string(),
            description: "Comprehensive multi-pass forensic analysis".to_string(),
            is_builtin: true,
            passes: vec![
                crate::PipelinePass {
                    name: "ocr_extraction".to_string(),
                    description: "OCR scan for embedded images".to_string(),
                    prompt_template: "Perform OCR on any images in:".to_string(),
                    output_schema: None,
                    max_tokens: 4096,
                    temperature: 0.0,
                    sample_size: None,
                },
                crate::PipelinePass {
                    name: "fact_validation".to_string(),
                    description: "Validate extracted facts".to_string(),
                    prompt_template: "Cross-validate each fact against the document:".to_string(),
                    output_schema: None,
                    max_tokens: 2048,
                    temperature: 0.2,
                    sample_size: None,
                },
                crate::PipelinePass {
                    name: "timeline_construction".to_string(),
                    description: "Build chronological timeline".to_string(),
                    prompt_template: "Construct a timeline of events from:".to_string(),
                    output_schema: None,
                    max_tokens: 2048,
                    temperature: 0.1,
                    sample_size: None,
                },
            ],
        },
    ]
}
