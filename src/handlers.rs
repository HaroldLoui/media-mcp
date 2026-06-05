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
}

#[tool_router(server_handler)]
impl MediaServer {
    #[tool(description = "Read a multimedia file and return its metadata, OCR text, and AI-generated description. Supports images (PNG, JPG, GIF, BMP, WEBP, SVG, TIFF), documents (PDF), and media files (MP4, AVI, MP3, WAV, etc.). Use this instead of the Read tool for multimedia files.")]
    async fn read_media(
        &self,
        Parameters(ReadMediaRequest { file_path, language }): Parameters<ReadMediaRequest>,
    ) -> String {
        let lang = language.unwrap_or_else(|| self.config.languages_string());

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

        // 2. OCR (for images)
        let ocr_result = if meta.mime_type.starts_with("image/") {
            let result = ocr::run_ocr(&file_path, &lang);
            if let Some(w) = result.warning {
                warnings.push(w);
            }
            result.text
        } else {
            None
        };

        // 3. Vision API (for images)
        let vision_result = if meta.mime_type.starts_with("image/") {
            let result = vision::describe_image(&file_path, &meta.mime_type, &self.config.vision_api).await;
            if let Some(w) = result.warning {
                warnings.push(w);
            }
            result.description
        } else {
            warnings.push(format!(
                "AI description not yet supported for {} files. Only images are supported in v0.1.",
                meta.mime_type
            ));
            None
        };

        let response = ReadMediaResponse {
            file: meta,
            ocr_text: ocr_result,
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
