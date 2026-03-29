use sha2::{Sha256, Digest};
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use walkdir::WalkDir;
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

/// Supported file types for evidence processing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileType {
    Pdf,
    Image,
    Audio,
    Video,
    Docx,
    Text,
    Unknown,
}

impl FileType {
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "pdf" => FileType::Pdf,
            "jpg" | "jpeg" | "png" | "bmp" | "tiff" | "tif" | "gif" | "webp" => FileType::Image,
            "mp3" | "wav" | "m4a" | "ogg" | "flac" => FileType::Audio,
            "mp4" | "mov" | "avi" | "mkv" | "webm" => FileType::Video,
            "docx" | "doc" => FileType::Docx,
            "txt" | "md" | "rtf" => FileType::Text,
            _ => FileType::Unknown,
        }
    }

    pub fn is_supported(&self) -> bool {
        !matches!(self, FileType::Unknown)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            FileType::Pdf => "pdf",
            FileType::Image => "image",
            FileType::Audio => "audio",
            FileType::Video => "video",
            FileType::Docx => "docx",
            FileType::Text => "text",
            FileType::Unknown => "unknown",
        }
    }
}

/// File metadata for registry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub path: String,
    pub file_name: String,
    pub extension: String,
    pub file_type: FileType,
    pub size: u64,
    pub fingerprint: String,
    pub modified: Option<u64>,
}

impl FileMetadata {
    pub fn from_path(path: &Path) -> Result<Self, Box<dyn std::error::Error>> {
        let path_str = path.to_string_lossy().to_string();
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        
        let extension = path
            .extension()
            .map(|e| e.to_string_lossy().to_string())
            .unwrap_or_default();
        
        let file_type = FileType::from_extension(&extension);
        
        let metadata = std::fs::metadata(path)?;
        let size = metadata.len();
        
        let modified = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs());
        
        let fingerprint = compute_fingerprint(path)?;
        
        Ok(FileMetadata {
            path: path_str,
            file_name,
            extension,
            file_type,
            size,
            fingerprint,
            modified,
        })
    }
}

/// Compute SHA-256 fingerprint of a file
pub fn compute_fingerprint(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    
    // For large files, only hash first and last 64KB
    const CHUNK_SIZE: usize = 65536;
    let mut buffer = [0u8; CHUNK_SIZE];
    
    // Read first chunk
    let n = reader.read(&mut buffer)?;
    if n > 0 {
        hasher.update(&buffer[..n]);
    }
    
    // If file is large, also read last chunk
    let metadata = std::fs::metadata(path)?;
    if metadata.len() > (CHUNK_SIZE * 2) as u64 {
        // Seek to near end
        use std::io::Seek;
        let seek_pos = metadata.len() - CHUNK_SIZE as u64;
        reader.seek(std::io::SeekFrom::Start(seek_pos))?;
        
        let n = reader.read(&mut buffer)?;
        if n > 0 {
            hasher.update(&buffer[..n]);
        }
    }
    
    Ok(hex::encode(hasher.finalize()))
}

/// Walk a directory and collect all supported files
pub fn walk_directory(root: &Path, max_depth: Option<usize>) -> Vec<FileMetadata> {
    let mut files = Vec::new();
    
    let walker = match max_depth {
        Some(depth) => WalkDir::new(root).max_depth(depth),
        None => WalkDir::new(root),
    };
    
    for entry in walker.into_iter().filter_map(|e| e.ok()) {
        if entry.file_type().is_file() {
            if let Ok(metadata) = FileMetadata::from_path(entry.path()) {
                if metadata.file_type.is_supported() {
                    files.push(metadata);
                } else {
                    warn!("Skipping unsupported file type: {}", entry.path().display());
                }
            }
        }
    }
    
    info!("Found {} supported files in {}", files.len(), root.display());
    files
}

/// Get file hash for content-based deduplication (full file)
pub fn compute_full_hash(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    
    let mut buffer = [0u8; 8192];
    loop {
        let n = reader.read(&mut buffer)?;
        if n == 0 {
            break;
        }
        hasher.update(&buffer[..n]);
    }
    
    Ok(hex::encode(hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_file_type_detection() {
        assert_eq!(FileType::from_extension("pdf"), FileType::Pdf);
        assert_eq!(FileType::from_extension("PDF"), FileType::Pdf);
        assert_eq!(FileType::from_extension("jpg"), FileType::Image);
        assert_eq!(FileType::from_extension("MP3"), FileType::Audio);
        assert_eq!(FileType::from_extension("xyz"), FileType::Unknown);
    }

    #[test]
    fn test_fingerprint() {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(b"test content").unwrap();
        
        let hash = compute_fingerprint(file.path()).unwrap();
        assert_eq!(hash.len(), 64);
    }
}
