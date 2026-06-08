use serde::Deserialize;
use std::path::PathBuf;

/// Top-level configuration for the media-mcp server.
#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    /// Configuration for the Vision API (Anthropic-compatible endpoint).
    pub vision_api: VisionApiConfig,
    /// OCR engine configuration.
    pub ocr: OcrConfig,
    /// File extensions blocked by the PreToolUse hook.
    pub blocked_extensions: Vec<String>,
}

/// Vision API configuration (compatible with Anthropic messages API).
#[derive(Debug, Clone, Deserialize)]
pub struct VisionApiConfig {
    /// API base URL (e.g., "https://api.example.com/anthropic").
    pub base_url: String,
    /// API key for authentication.
    pub api_key: String,
    /// Model name (e.g., "mimo-v2.5", "claude-opus-4-8").
    pub model: String,
    /// Maximum tokens for the AI description response.
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    /// HTTP request timeout in seconds.
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    /// Prompt text sent to the Vision API for image description.
    #[serde(default = "default_prompt")]
    pub prompt: String,
}

/// OCR engine configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct OcrConfig {
    /// Default engine selection mode: "auto" | "tesseract" | "paddle".
    #[serde(default = "default_engine")]
    pub default_engine: String,
    /// Per-engine configurations.
    pub engines: OcrEngines,
    /// Confidence threshold (0–100) for Vision API fallback in "auto" mode.
    #[serde(default = "default_confidence_threshold")]
    pub confidence_threshold: u32,
}

/// Per-engine configuration container.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct OcrEngines {
    /// Tesseract OCR engine configuration.
    #[serde(default)]
    pub tesseract: TesseractConfig,
    /// PaddleOCR engine configuration.
    #[serde(default)]
    pub paddle: PaddleConfig,
}

/// PaddleOCR engine configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct PaddleConfig {
    /// Path to paddleocr executable (null = auto-detect from PATH).
    pub paddle_cmd: Option<String>,
    /// Language code (e.g., "ch" for Chinese, "en" for English).
    #[serde(default = "default_paddle_lang")]
    pub lang: Option<String>,
}

fn default_paddle_lang() -> Option<String> {
    Some("ch".to_string())
}

impl Default for PaddleConfig {
    fn default() -> Self {
        PaddleConfig {
            paddle_cmd: None,
            lang: Some("ch".to_string()),
        }
    }
}

/// Tesseract OCR engine configuration.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct TesseractConfig {
    /// Path to tesseract executable (null = auto-detect from PATH and common install locations).
    pub tesseract_cmd: Option<String>,
    /// OCR languages (e.g., ["chi_sim", "eng"]). Joined with "+" for tesseract.
    #[serde(default = "default_languages")]
    pub languages: Vec<String>,
}

fn default_engine() -> String {
    "auto".to_string()
}

fn default_confidence_threshold() -> u32 {
    60
}

fn default_max_tokens() -> u32 {
    10240
}

fn default_timeout() -> u64 {
    30
}

fn default_languages() -> Vec<String> {
    vec!["chi_sim".to_string(), "eng".to_string()]
}

fn default_prompt() -> String {
    "请详细描述这张图片的内容，包括文字、界面元素、布局等。用中文回答。".to_string()
}

impl Config {
    pub fn load(path: Option<&str>) -> anyhow::Result<Self> {
        let config_path = if let Some(p) = path {
            PathBuf::from(p)
        } else if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
            let global = PathBuf::from(home).join(".config/media-mcp/config.json");
            if global.exists() {
                global
            } else {
                let local = PathBuf::from(".media-mcp.json");
                if local.exists() {
                    local
                } else {
                    anyhow::bail!(
                        "No config file found. Provide --config <path> or create ~/.config/media-mcp/config.json"
                    );
                }
            }
        } else {
            anyhow::bail!("Cannot determine home directory and no --config provided");
        };

        let content = std::fs::read_to_string(&config_path)
            .map_err(|e| anyhow::anyhow!("Failed to read config {:?}: {}", config_path, e))?;
        let config: Config = serde_json::from_str(&content)?;
        Ok(config)
    }

    pub fn languages_string(&self) -> String {
        self.ocr.engines.tesseract.languages.join("+")
    }
}
