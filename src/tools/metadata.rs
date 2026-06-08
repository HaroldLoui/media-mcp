use serde::Serialize;
use std::path::Path;

/// Metadata about a multimedia file.
#[derive(Debug, Clone, Serialize)]
pub struct MediaMetadata {
    /// File name (last path component).
    pub name: String,
    /// Absolute path to the file.
    pub path: String,
    /// File size in bytes.
    pub size_bytes: u64,
    /// MIME type (e.g., "image/png", "application/pdf").
    pub mime_type: String,
    /// File format/extension in uppercase (e.g., "PNG", "JPG").
    pub format: String,
    /// Image dimensions as "WxH" (None for non-image files).
    pub dimensions: Option<String>,
}

pub fn extract_metadata(file_path: &str) -> anyhow::Result<MediaMetadata> {
    let path = Path::new(file_path);
    if !path.exists() {
        anyhow::bail!("File not found: {}", file_path);
    }

    let meta = std::fs::metadata(path)?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let mime = mime_guess::from_path(path)
        .first_or_octet_stream()
        .to_string();

    let format = path
        .extension()
        .map(|e| e.to_string_lossy().to_uppercase())
        .unwrap_or_else(|| "UNKNOWN".to_string());

    let dimensions = try_get_dimensions(path, &mime);

    Ok(MediaMetadata {
        name,
        path: path.to_string_lossy().to_string(),
        size_bytes: meta.len(),
        mime_type: mime,
        format,
        dimensions,
    })
}

fn try_get_dimensions(path: &Path, mime: &str) -> Option<String> {
    if !mime.starts_with("image/") {
        return None;
    }
    // SVG is text-based, image crate can't read it
    if mime == "image/svg+xml" {
        return None;
    }
    match image::image_dimensions(path) {
        Ok((w, h)) => Some(format!("{}x{}", w, h)),
        Err(_) => None,
    }
}
