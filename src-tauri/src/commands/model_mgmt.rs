use crate::utils;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DownloadProgress {
    pub bytes_downloaded: u64,
    pub total_bytes: u64,
    pub filename: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelInfo {
    pub id: String,
    pub filename: String,
    pub size: u64,
    pub path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HuggingFaceFile {
    #[serde(alias = "rfilename")]
    pub path: String,
    pub size: Option<u64>,
    #[serde(alias = "downloadUrl")]
    pub download_url: Option<String>,
}

#[allow(dead_code)]
async fn get_huggingface_tree(repo_id: &str) -> Result<String, String> {
    let url = format!("https://huggingface.co/api/models/{}", repo_id);

    let client = reqwest::Client::builder()
        .user_agent("SL-Studio/0.2.0")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch model info: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    #[derive(Deserialize)]
    struct ModelInfoResp {
        sha: Option<String>,
        #[allow(dead_code)]
        siblings: Option<Vec<HuggingFaceFile>>,
    }

    let info: ModelInfoResp = serde_json::from_str(&text).map_err(|e| {
        format!(
            "Failed to parse response: {}. Response preview: {}",
            e,
            &text[..text.len().min(300)]
        )
    })?;

    let sha = info.sha.unwrap_or_else(|| "main".to_string());
    Ok(sha)
}

async fn get_huggingface_files_with_size(repo_id: &str) -> Result<Vec<HuggingFaceFile>, String> {
    let files = get_huggingface_files(repo_id).await?;

    let gguf_files: Vec<HuggingFaceFile> = files
        .into_iter()
        .filter(|f| f.path.to_lowercase().ends_with(".gguf"))
        .collect();

    Ok(gguf_files)
}

async fn get_huggingface_files(repo_id: &str) -> Result<Vec<HuggingFaceFile>, String> {
    let url = format!("https://huggingface.co/api/models/{}", repo_id);

    let client = reqwest::Client::builder()
        .user_agent("SL-Studio/0.2.0")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let response = client
        .get(&url)
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| format!("Failed to fetch model info: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("HTTP error: {}", response.status()));
    }

    let text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response: {}", e))?;

    #[derive(Deserialize)]
    struct ModelInfoResp {
        siblings: Option<Vec<HuggingFaceFile>>,
    }

    let info: ModelInfoResp = serde_json::from_str(&text).map_err(|e| {
        format!(
            "Failed to parse response: {}. Response preview: {}",
            e,
            &text[..text.len().min(300)]
        )
    })?;

    info.siblings
        .ok_or_else(|| "No files found in model repository".to_string())
}

#[allow(dead_code)]
fn find_gguf_file(files: &[HuggingFaceFile]) -> Option<(String, u64)> {
    for file in files {
        if file.path.to_lowercase().ends_with(".gguf") {
            let url = file.download_url.as_ref()?;
            return Some((url.clone(), file.size.unwrap_or(0)));
        }
    }
    None
}

#[tauri::command]
pub async fn get_huggingface_models(repo_id: String) -> Result<Vec<String>, String> {
    let files = get_huggingface_files_with_size(&repo_id).await?;
    let gguf_files: Vec<String> = files
        .into_iter()
        .filter(|f| f.path.to_lowercase().ends_with(".gguf"))
        .map(|f| f.path)
        .collect();
    Ok(gguf_files)
}

#[tauri::command]
pub async fn download_model(
    app: AppHandle,
    repo_id: String,
    filename: String,
) -> Result<ModelInfo, String> {
    let files = get_huggingface_files_with_size(&repo_id).await?;

    let file = if filename.contains(".gguf") {
        files
            .iter()
            .find(|f| f.path == filename)
            .ok_or_else(|| "File not found in repository".to_string())?
    } else {
        files
            .iter()
            .find(|f| f.path.to_lowercase().ends_with(".gguf"))
            .ok_or_else(|| "No GGUF files found".to_string())?
    };

    let filename_for_url = file.path.clone();
    let actual_filename = file.path.clone();

    let download_url = format!(
        "https://huggingface.co/{}/resolve/main/{}",
        repo_id, filename_for_url
    );
    let total_size = file.size.unwrap_or(0);

    let models_dir = utils::models_dir();

    std::fs::create_dir_all(&models_dir).map_err(|e| {
        format!(
            "Failed to create models directory: {}. Check permissions.",
            e
        )
    })?;

    let output_path = models_dir.join(&actual_filename);

    info!("Starting download from: {}", download_url);

    app.emit(
        "download_status",
        DownloadProgress {
            bytes_downloaded: 0,
            total_bytes: 0,
            filename: actual_filename.to_string(),
            status: "starting".to_string(),
        },
    )
    .ok();

    let client = reqwest::Client::builder()
        .user_agent("SL-Studio/0.2.0")
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let mut response = client
        .get(&download_url)
        .header("Accept", "application/octet-stream")
        .header("User-Agent", "SL-Studio/0.2.0")
        .send()
        .await
        .map_err(|e| format!("Failed to connect to HuggingFace: {}. Make sure you have accepted the model terms on the website.", e))?;

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        return Err(format!("HTTP error: {}. Response: {}", status, error_text));
    }

    let total_size = response.content_length().unwrap_or(total_size);

    let mut file =
        std::fs::File::create(&output_path).map_err(|e| format!("Failed to create file: {}", e))?;

    use std::io::Write;
    let mut bytes_downloaded = 0u64;

    while let Some(chunk) = response.chunk().await.map_err(|e| format!("Download error: {}", e))? {
        file.write_all(&chunk)
            .map_err(|e| format!("Failed to write: {}", e))?;

        bytes_downloaded += chunk.len() as u64;

        app.emit(
            "download_status",
            DownloadProgress {
                bytes_downloaded,
                total_bytes: total_size,
                filename: actual_filename.to_string(),
                status: "downloading".to_string(),
            },
        )
        .ok();
    }

    file.flush().map_err(|e| e.to_string())?;

    app.emit(
        "download_status",
        DownloadProgress {
            bytes_downloaded,
            total_bytes: total_size,
            filename: actual_filename.to_string(),
            status: "complete".to_string(),
        },
    )
    .ok();

    info!("Download complete: {:?}", output_path);

    Ok(ModelInfo {
        id: repo_id,
        filename,
        size: bytes_downloaded,
        path: output_path.to_string_lossy().to_string(),
    })
}

#[tauri::command]
pub fn list_downloaded_models() -> Vec<ModelInfo> {
    let models_dir = if crate::IS_DEV {
        utils::dev_models_dir()
    } else {
        utils::models_dir()
    };

    let mut models = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&models_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map(|e| e == "gguf").unwrap_or(false) {
                if let Ok(metadata) = std::fs::metadata(&path) {
                    models.push(ModelInfo {
                        id: "local".to_string(),
                        filename: path
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default(),
                        size: metadata.len(),
                        path: path.to_string_lossy().to_string(),
                    });
                }
            }
        }
    }

    models
}
