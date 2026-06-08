use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub vision_api: VisionApiConfig,
    pub ocr: OcrConfig,
    pub blocked_extensions: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VisionApiConfig {
    pub base_url: String,
    pub api_key: String,
    pub model: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64,
    #[serde(default = "default_prompt")]
    pub prompt: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OcrConfig {
    #[serde(default = "default_engine")]
    pub default_engine: String,
    pub engines: OcrEngines,
    #[serde(default = "default_confidence_threshold")]
    pub confidence_threshold: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OcrEngines {
    #[serde(default)]
    pub tesseract: TesseractConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TesseractConfig {
    pub tesseract_cmd: Option<String>,
    #[serde(default = "default_languages")]
    pub languages: Vec<String>,
}

fn default_engine() -> String {
    "auto".to_string()
}

impl Default for TesseractConfig {
    fn default() -> Self {
        TesseractConfig {
            tesseract_cmd: None,
            languages: vec!["chi_sim".to_string(), "eng".to_string()],
        }
    }
}

impl Default for OcrEngines {
    fn default() -> Self {
        OcrEngines {
            tesseract: TesseractConfig::default(),
        }
    }
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
