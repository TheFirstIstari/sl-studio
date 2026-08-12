// MLX pipeline for SL Studio — wraps the `rapid-mlx` CLI subprocess.
//
// The pipeline spawns `rapid-mlx serve <model_name>` as a background process
// and communicates with it via the OpenAI-compatible HTTP API on localhost:8000.

#![allow(dead_code)]

use anyhow::Result;
use std::process::Child;
use std::time::Duration;
use tracing::info;

/// MLX inference pipeline backed by the `rapid-mlx` serve subprocess.
pub struct MlxPipeline {
    pub model_name: String,
    pub context_length: usize,
    pub server_url: String,
    pub child: Option<Child>,
}

impl MlxPipeline {
    pub fn new(model_name: String, context_length: usize) -> Self {
        Self {
            model_name,
            context_length,
            server_url: "http://127.0.0.1:8000".to_string(),
            child: None,
        }
    }

    pub fn load(&mut self) -> Result<()> {
        info!("Starting rapid-mlx serve for model: {}", self.model_name);

        let child = std::process::Command::new("rapid-mlx")
            .args(["serve", &self.model_name])
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()?;

        self.child = Some(child);

        // Poll the health endpoint until the server is ready (up to 30s).
        let client = reqwest::blocking::Client::new();
        let health_url = format!("{}/health", self.server_url);
        for _ in 0..60 {
            if let Ok(resp) = client.get(&health_url).send() {
                if resp.status().is_success() {
                    info!("rapid-mlx serve is ready at {}", self.server_url);
                    return Ok(());
                }
            }
            std::thread::sleep(Duration::from_millis(500));
        }

        Err(anyhow::anyhow!(
            "rapid-mlx serve did not become ready within 30s for model: {}",
            self.model_name
        ))
    }

    pub fn infer(&self, prompt: &str, max_tokens: usize) -> Result<String> {
        info!(
            "Running MLX inference: {} chars, max_tokens={}",
            prompt.len(),
            max_tokens
        );

        let client = reqwest::blocking::Client::new();
        let body = serde_json::json!({
            "messages": [{"role": "user", "content": prompt}],
            "max_tokens": max_tokens,
        });

        let resp = client
            .post(format!("{}/v1/chat/completions", self.server_url))
            .json(&body)
            .send()?;

        let response: serde_json::Value = resp.json()?;

        let content = response["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("")
            .to_string();

        Ok(content)
    }
}

impl Drop for MlxPipeline {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
        }
    }
}
