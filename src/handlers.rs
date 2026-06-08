use crate::config::Config;
use crate::tools::{metadata, ocr, vision};
use rmcp::{
    handler::server::wrapper::Parameters,
    schemars,
    tool, tool_router,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ReadMediaRequest {
    #[schemars(description = "Absolute path to the multimedia file")]
    pub file_path: String,
    #[schemars(description = "OCR language hint, e.g. 'chi_sim+eng'")]
    pub language: Option<String>,
    #[schemars(description = "Vision API mode: 'auto' (default, OCR-based fallback), 'always' (force Vision), 'skip' (OCR only)")]
    pub vision: Option<String>,
}

#[derive(Debug, Serialize)]
struct ReadMediaResponse {
    file: metadata::MediaMetadata,
    #[serde(skip_serializing_if = "Option::is_none")]
    ocr_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ai_description: Option<String>,
    warnings: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct MediaServer {
    pub config: Config,
    pub client: reqwest::Client,
}

#[tool_router(server_handler)]
impl MediaServer {
    #[tool(description = "Read a multimedia file and return its metadata, OCR text, and AI-generated description. Supports images (PNG, JPG, GIF, BMP, WEBP, SVG, TIFF), documents (PDF), and media files (MP4, AVI, MP3, WAV, etc.). Use this instead of the Read tool for multimedia files.")]
    async fn read_media(
        &self,
        Parameters(ReadMediaRequest { file_path, language, vision }): Parameters<ReadMediaRequest>,
    ) -> String {
        let lang = language.unwrap_or_else(|| self.config.languages_string());

        // 0. Validate path
        #[cfg(unix)]
        {
            if !file_path.starts_with('/') {
                return serde_json::json!({
                    "error": format!("Path must be absolute: {}", file_path)
                })
                .to_string();
            }
        }
        #[cfg(windows)]
        {
            let bytes = file_path.as_bytes();
            if bytes.len() < 2
                || !bytes[0].is_ascii_alphabetic()
                || bytes[1] != b':'
            {
                return serde_json::json!({
                    "error": format!("Path must be absolute (e.g. C:\\...): {}", file_path)
                })
                .to_string();
            }
        }

        // Resolve symlinks and normalize path
        let resolved_path = match std::fs::canonicalize(&file_path) {
            Ok(p) => p,
            Err(e) => {
                // file_not_found is OK (may not exist yet), but invalid paths are not
                if e.kind() == std::io::ErrorKind::NotFound {
                    std::path::PathBuf::from(&file_path)
                } else {
                    return serde_json::json!({
                        "error": format!("Invalid path '{}': {}", file_path, e)
                    })
                    .to_string();
                }
            }
        };
        let file_path = resolved_path.to_string_lossy().to_string();

        // 1. Metadata
        let meta = match metadata::extract_metadata(&file_path) {
            Ok(m) => m,
            Err(e) => {
                return serde_json::json!({
                    "error": e.to_string()
                })
                .to_string();
            }
        };

        let mut warnings: Vec<String> = Vec::new();

        // Determine vision mode
        let vision_mode = vision.as_deref().unwrap_or("auto");

        // 2. OCR (for images) — always run OCR for images
        let mut ocr_text: Option<String> = None;
        let mut need_vision = vision_mode == "always";

        if meta.mime_type.starts_with("image/") {
            if vision_mode == "skip" || vision_mode == "always" {
                // Plain OCR, no confidence check
                let result = ocr::run_ocr(&file_path, &lang);
                if let Some(w) = result.warning {
                    warnings.push(w);
                }
                ocr_text = result.text;
                need_vision = vision_mode == "always";
            } else {
                // "auto" mode: OCR with confidence check
                let result = ocr::run_ocr_with_confidence(&file_path, &lang);
                if let Some(w) = result.warning {
                    warnings.push(w);
                }
                ocr_text = result.text;

                if result.average_confidence < self.config.ocr.confidence_threshold as f64 {
                    need_vision = true;
                    warnings.push(format!(
                        "OCR confidence ({:.0}/100) below threshold ({}); falling back to Vision API",
                        result.average_confidence,
                        self.config.ocr.confidence_threshold,
                    ));
                }
            }
        } else {
            warnings.push(format!(
                "AI description not yet supported for {} files. Only images are supported in v0.1.",
                meta.mime_type
            ));
        }

        // 3. Vision API (when needed)
        let vision_result = if need_vision && meta.mime_type.starts_with("image/") {
            let result = vision::describe_image(&self.client, &file_path, &meta.mime_type, &self.config.vision_api).await;
            if let Some(w) = result.warning {
                warnings.push(w);
            }
            result.description
        } else {
            None
        };

        let response = ReadMediaResponse {
            file: meta,
            ocr_text,
            ai_description: vision_result,
            warnings,
        };

        serde_json::to_string_pretty(&response).unwrap_or_else(|e| {
            format!("{{\"error\": \"Serialization failed: {}\"}}", e)
        })
    }

    #[tool(description = "List all supported multimedia file types and check the status of OCR and Vision API availability.")]
    async fn list_supported_types(&self) -> String {
        let mut status = serde_json::json!({
            "supported_image_types": ["png", "jpg", "jpeg", "gif", "bmp", "webp", "svg", "ico", "tiff"],
            "supported_document_types": ["pdf"],
            "supported_video_types": ["mp4", "avi", "mkv", "mov", "webm"],
            "supported_audio_types": ["mp3", "wav", "flac", "aac", "ogg"],
            "ocr_available": false,
            "vision_api_configured": !self.config.vision_api.api_key.is_empty(),
            "vision_api_model": self.config.vision_api.model,
        });

        // Check OCR availability
        status["ocr_available"] = serde_json::json!(ocr::is_available());

        status.to_string()
    }
}
