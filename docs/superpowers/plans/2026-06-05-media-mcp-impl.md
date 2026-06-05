# Media MCP Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Rust MCP server that reads multimedia files (images, documents, audio, video) and returns text-only results (metadata + OCR + AI description), paired with a bash hook to prevent Claude Code's Read tool from sending images to non-multimodal models.

**Architecture:** MCP server using `rmcp` crate with stdio transport, exposing `read_media` and `list_supported_types` tools. PreToolUse bash hook intercepts Read calls on multimedia file extensions. Vision API integration uses Anthropic-compatible format with configurable endpoint.

**Tech Stack:** Rust (edition 2024), rmcp 1.7, image 0.25, tesseract 0.15, reqwest 0.12, tokio, serde, clap 4

---

## File Map

| File | Responsibility |
|------|---------------|
| `Cargo.toml` | Dependencies and features |
| `src/main.rs` | Entry point, CLI args, MCP server startup |
| `src/config.rs` | Config struct, loading from file, defaults |
| `src/tools/mod.rs` | Module declarations |
| `src/tools/metadata.rs` | Image/file metadata extraction (format, dimensions, size) |
| `src/tools/ocr.rs` | Tesseract OCR wrapper with graceful degradation |
| `src/tools/vision.rs` | Vision API HTTP client (Anthropic-compatible) |
| `src/handlers.rs` | MCP tool handler struct, `read_media` and `list_supported_types` implementations |
| `hooks/guard-multimedia.sh` | PreToolUse hook script |
| `config.json.example` | Example configuration |
| `test_assets/` | Test images for integration testing |

---

### Task 1: Project Scaffolding

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`

- [ ] **Step 1: Initialize Cargo project**

```bash
cd D:/workspace/RustProjects
cargo new media-mcp
cd media-mcp
```

Expected: `Created binary (application) media-mcp package`

- [ ] **Step 2: Write Cargo.toml with all dependencies**

```toml
[package]
name = "media-mcp"
version = "0.1.0"
edition = "2024"
description = "MCP server for reading multimedia files as text (metadata + OCR + AI description)"

[dependencies]
rmcp = { version = "1.7", features = ["transport-io", "macros", "server", "base64", "schemars"] }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
clap = { version = "4", features = ["derive"] }
image = "0.25"
mime_guess = "2"
base64 = "0.22"
reqwest = { version = "0.12", features = ["json"] }

[dependencies.tesseract]
version = "0.15"
features = []
optional = true

[features]
default = ["ocr"]
ocr = ["dep:tesseract"]
```

- [ ] **Step 3: Write minimal main.rs**

```rust
use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "media-mcp", about = "MCP server for multimedia file reading")]
struct Args {
    /// Path to config file
    #[arg(long)]
    config: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let _args = Args::parse();
    tracing::info!("media-mcp starting");

    Ok(())
}
```

- [ ] **Step 4: Verify it compiles**

```bash
cargo build 2>&1
```

Expected: `Finished dev profile [unoptimized + debuginfo]` (may take a while on first build)

- [ ] **Step 5: Commit**

```bash
git init
git add -A
git commit -m "feat: scaffold media-mcp project with dependencies"
```

---

### Task 2: Config Module

**Files:**
- Create: `src/config.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Write config.rs**

```rust
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
}

#[derive(Debug, Clone, Deserialize)]
pub struct OcrConfig {
    #[serde(default = "default_languages")]
    pub languages: Vec<String>,
    pub tesseract_cmd: Option<String>,
}

fn default_max_tokens() -> u32 {
    1024
}

fn default_timeout() -> u64 {
    30
}

fn default_languages() -> Vec<String> {
    vec!["chi_sim".to_string(), "eng".to_string()]
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
        self.ocr.languages.join("+")
    }
}
```

- [ ] **Step 2: Wire into main.rs**

Add to `src/main.rs`:

```rust
mod config;

use config::Config;

// In main(), after args parsing:
let config = Config::load(args.config.as_deref())?;
tracing::info!("Config loaded: model={}, ocr_langs={}", config.vision_api.model, config.languages_string());
```

- [ ] **Step 3: Create config.json.example**

```json
{
  "vision_api": {
    "base_url": "https://token-plan-cn.xiaomimimo.com/anthropic",
    "api_key": "tp-your-api-key-here",
    "model": "mimo-v2-omni",
    "max_tokens": 1024,
    "timeout_seconds": 30
  },
  "ocr": {
    "languages": ["chi_sim", "eng"],
    "tesseract_cmd": null
  },
  "blocked_extensions": [
    ".png", ".jpg", ".jpeg", ".gif", ".bmp", ".webp", ".svg", ".ico", ".tiff",
    ".mp4", ".avi", ".mkv", ".mov", ".webm",
    ".mp3", ".wav", ".flac", ".aac", ".ogg",
    ".pdf"
  ]
}
```

- [ ] **Step 4: Verify it compiles**

```bash
cargo build 2>&1
```

Expected: Compiles successfully (config file not needed at compile time)

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat: add config module with loading and defaults"
```

---

### Task 3: Metadata Extraction Module

**Files:**
- Create: `src/tools/mod.rs`
- Create: `src/tools/metadata.rs`

- [ ] **Step 1: Write tools/mod.rs**

```rust
pub mod metadata;
pub mod ocr;
pub mod vision;
```

- [ ] **Step 2: Write tools/metadata.rs**

```rust
use serde::Serialize;
use std::path::Path;

#[derive(Debug, Clone, Serialize)]
pub struct MediaMetadata {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub mime_type: String,
    pub format: String,
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
```

- [ ] **Step 3: Add `mod tools;` to main.rs**

Add after `mod config;`:

```rust
mod tools;
```

- [ ] **Step 4: Verify it compiles**

```bash
cargo build 2>&1
```

Expected: Compiles successfully

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "feat: add metadata extraction module"
```

---

### Task 4: OCR Module

**Files:**
- Create: `src/tools/ocr.rs`

- [ ] **Step 1: Write tools/ocr.rs**

```rust
use std::path::Path;

/// Run OCR on an image file. Returns extracted text or None if OCR is unavailable.
/// Gracefully degrades: if tesseract is not installed or the feature is disabled,
/// returns None with a warning message.
pub fn run_ocr(file_path: &str, languages: &str) -> OcrResult {
    #[cfg(feature = "ocr")]
    {
        run_ocr_impl(file_path, languages)
    }
    #[cfg(not(feature = "ocr"))]
    {
        let _ = (file_path, languages);
        OcrResult {
            text: None,
            warning: Some("OCR feature not enabled. Rebuild with --features ocr".to_string()),
        }
    }
}

#[cfg(feature = "ocr")]
fn run_ocr_impl(file_path: &str, languages: &str) -> OcrResult {
    let path = Path::new(file_path);
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    // Only OCR image types that tesseract can handle
    let ocrable = ["png", "jpg", "jpeg", "bmp", "tiff", "tif", "webp"];
    if !ocrable.contains(&ext.as_str()) {
        return OcrResult {
            text: None,
            warning: Some(format!("OCR not supported for .{} files", ext)),
        };
    }

    // Tesseract uses TESSDATA_PREFIX env var for data path (set in main.rs from config)
    match tesseract::Tesseract::new(None, Some(languages)) {
        Ok(tess) => {
            let tess = match tess.set_image(file_path) {
                Ok(t) => t,
                Err(e) => {
                    return OcrResult {
                        text: None,
                        warning: Some(format!("OCR set_image failed: {:?}", e)),
                    };
                }
            };
            match tess.get_text() {
                Ok(text) => {
                    let trimmed = text.trim();
                    if trimmed.is_empty() {
                        OcrResult {
                            text: None,
                            warning: Some("OCR returned empty text".to_string()),
                        }
                    } else {
                        OcrResult {
                            text: Some(trimmed.to_string()),
                            warning: None,
                        }
                    }
                }
                Err(e) => OcrResult {
                    text: None,
                    warning: Some(format!("OCR get_text failed: {:?}", e)),
                },
            }
        }
        Err(e) => OcrResult {
            text: None,
            warning: Some(format!(
                "Tesseract not available: {:?}. Install tesseract-ocr package.",
                e
            )),
        },
    }
}


pub struct OcrResult {
    pub text: Option<String>,
    pub warning: Option<String>,
}

/// Check if OCR (tesseract) is available on this system.
pub fn is_available() -> bool {
    #[cfg(feature = "ocr")]
    {
        tesseract::Tesseract::new(None, Some("eng")).is_ok()
    }
    #[cfg(not(feature = "ocr"))]
    {
        false
    }
}
```

- [ ] **Step 2: Verify it compiles**

```bash
cargo build 2>&1
```

Expected: Compiles (tesseract may fail to link if system lib not installed — that's OK, the feature flag handles it)

If tesseract linking fails, build without OCR:

```bash
cargo build --no-default-features 2>&1
```

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat: add OCR module with graceful degradation"
```

---

### Task 5: Vision API Module

**Files:**
- Create: `src/tools/vision.rs`

- [ ] **Step 1: Write tools/vision.rs**

```rust
use crate::config::VisionApiConfig;
use base64::Engine;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<Message>,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: Vec<ContentBlock>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum ContentBlock {
    #[serde(rename = "image")]
    Image { source: ImageSource },
    #[serde(rename = "text")]
    Text { text: String },
}

#[derive(Debug, Serialize)]
struct ImageSource {
    #[serde(rename = "type")]
    source_type: String,
    media_type: String,
    data: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<ResponseContent>,
}

#[derive(Debug, Deserialize)]
struct ResponseContent {
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

/// Call vision API to describe an image. Returns description or warning.
pub async fn describe_image(
    file_path: &str,
    mime_type: &str,
    config: &VisionApiConfig,
) -> VisionResult {
    // Read and encode image
    let image_data = match std::fs::read(file_path) {
        Ok(data) => data,
        Err(e) => {
            return VisionResult {
                description: None,
                warning: Some(format!("Failed to read image file: {}", e)),
            };
        }
    };

    // Resize if too large (>10MB)
    let data = if image_data.len() > 10 * 1024 * 1024 {
        match resize_image(&image_data, file_path) {
            Ok(resized) => resized,
            Err(e) => {
                return VisionResult {
                    description: None,
                    warning: Some(format!("Failed to resize large image: {}", e)),
                };
            }
        }
    } else {
        image_data
    };

    let b64 = base64::engine::general_purpose::STANDARD.encode(&data);

    let request = AnthropicRequest {
        model: config.model.clone(),
        max_tokens: config.max_tokens,
        messages: vec![Message {
            role: "user".to_string(),
            content: vec![
                ContentBlock::Image {
                    source: ImageSource {
                        source_type: "base64".to_string(),
                        media_type: mime_type.to_string(),
                        data: b64,
                    },
                },
                ContentBlock::Text {
                    text: "请详细描述这张图片的内容，包括文字、界面元素、布局等。用中文回答。".to_string(),
                },
            ],
        }],
    };

    let url = format!("{}/v1/messages", config.base_url.trim_end_matches('/'));

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(config.timeout_seconds))
        .build()
        .unwrap_or_default();

    // Retry once on failure
    for attempt in 0..2 {
        match client
            .post(&url)
            .header("x-api-key", &config.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
        {
            Ok(resp) => {
                if !resp.status().is_success() {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    if attempt == 0 {
                        tracing::warn!("Vision API returned {}, retrying...", status);
                        continue;
                    }
                    return VisionResult {
                        description: None,
                        warning: Some(format!("Vision API error {}: {}", status, body)),
                    };
                }
                match resp.json::<AnthropicResponse>().await {
                    Ok(api_resp) => {
                        let text: String = api_resp
                            .content
                            .iter()
                            .filter_map(|c| c.text.as_deref())
                            .collect::<Vec<_>>()
                            .join("");
                        if text.is_empty() {
                            return VisionResult {
                                description: None,
                                warning: Some("Vision API returned empty response".to_string()),
                            };
                        }
                        return VisionResult {
                            description: Some(text),
                            warning: None,
                        };
                    }
                    Err(e) => {
                        if attempt == 0 {
                            tracing::warn!("Failed to parse vision API response: {}, retrying...", e);
                            continue;
                        }
                        return VisionResult {
                            description: None,
                            warning: Some(format!("Failed to parse vision API response: {}", e)),
                        };
                    }
                }
            }
            Err(e) => {
                if attempt == 0 {
                    tracing::warn!("Vision API request failed: {}, retrying...", e);
                    continue;
                }
                return VisionResult {
                    description: None,
                    warning: Some(format!("Vision API request failed: {}", e)),
                };
            }
        }
    }

    VisionResult {
        description: None,
        warning: Some("Vision API failed after retry".to_string()),
    }
}

fn resize_image(data: &[u8], _path: &str) -> anyhow::Result<Vec<u8>> {
    let img = image::load_from_memory(data)?;
    // Scale down to max 2048px on longest side
    let longest = img.width().max(img.height());
    if longest <= 2048 {
        return Ok(data.to_vec());
    }
    let scale = 2048.0 / longest as f32;
    let new_w = (img.width() as f32 * scale) as u32;
    let new_h = (img.height() as f32 * scale) as u32;
    let resized = img.resize_exact(new_w, new_h, image::imageops::FilterType::Lanczos3);
    let mut buf = std::io::Cursor::new(Vec::new());
    resized.write_to(&mut buf, image::ImageFormat::Png)?;
    Ok(buf.into_inner())
}

pub struct VisionResult {
    pub description: Option<String>,
    pub warning: Option<String>,
}
```

- [ ] **Step 2: Verify it compiles**

```bash
cargo build 2>&1
```

Expected: Compiles successfully

- [ ] **Step 3: Commit**

```bash
git add -A
git commit -m "feat: add vision API client with retry and image resizing"
```

---

### Task 6: MCP Tool Handlers

**Files:**
- Create: `src/handlers.rs`
- Modify: `src/main.rs`

- [ ] **Step 1: Write handlers.rs**

```rust
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
```

- [ ] **Step 2: Wire handlers into main.rs**

Replace `src/main.rs` with:

```rust
mod config;
mod handlers;
mod tools;

use anyhow::Result;
use clap::Parser;
use config::Config;
use handlers::MediaServer;
use rmcp::{ServiceExt, transport::stdio};
use tracing_subscriber::EnvFilter;

#[derive(Parser)]
#[command(name = "media-mcp", about = "MCP server for multimedia file reading")]
struct Args {
    /// Path to config file
    #[arg(long)]
    config: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    let args = Args::parse();
    let config = Config::load(args.config.as_deref())?;

    // Set TESSDATA_PREFIX for tesseract if configured
    if let Some(ref cmd) = config.ocr.tesseract_cmd {
        std::env::set_var("TESSDATA_PREFIX", cmd);
    }

    tracing::info!(
        "media-mcp starting: model={}, ocr_langs={}",
        config.vision_api.model,
        config.languages_string()
    );

    let server = MediaServer { config };
    let service = server.serve(stdio()).await.inspect_err(|e| {
        tracing::error!("MCP serve error: {:?}", e);
    })?;

    service.waiting().await?;
    Ok(())
}
```

- [ ] **Step 3: Verify it compiles**

```bash
cargo build 2>&1
```

Expected: Compiles successfully

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat: add MCP tool handlers (read_media, list_supported_types)"
```

---

### Task 7: Bash Hook Script

**Files:**
- Create: `hooks/guard-multimedia.sh`

- [ ] **Step 1: Create hooks directory and script**

```bash
mkdir -p hooks
```

Write `hooks/guard-multimedia.sh`:

```bash
#!/bin/bash
# PreToolUse hook: block Read tool on multimedia files
# Reads JSON from stdin, outputs decision JSON to stdout

INPUT=$(cat)

# Only intercept Read tool
TOOL_NAME=$(echo "$INPUT" | jq -r '.tool_name // empty' 2>/dev/null)
if [ "$TOOL_NAME" != "Read" ]; then
    exit 0
fi

# Extract file path
FILE_PATH=$(echo "$INPUT" | jq -r '.tool_input.file_path // empty' 2>/dev/null)
if [ -z "$FILE_PATH" ]; then
    exit 0
fi

# Get extension (lowercase)
EXT="${FILE_PATH##*.}"
EXT_LOWER=$(echo "$EXT" | tr '[:upper:]' '[:lower:]')

# Blocked extensions list (must match config.json)
BLOCKED_EXTENSIONS=(
    png jpg jpeg gif bmp webp svg ico tiff tif
    mp4 avi mkv mov webm
    mp3 wav flac aac ogg
    pdf
)

for ext in "${BLOCKED_EXTENSIONS[@]}"; do
    if [ ".$EXT_LOWER" = ".$ext" ]; then
        echo "{\"decision\":\"block\",\"reason\":\"Multimedia file detected. Use read_media(file_path=\\\"$FILE_PATH\\\") from media-mcp tool instead of Read. The Read tool cannot process this file type and will cause an API error.\"}"
        exit 0
    fi
done

exit 0
```

- [ ] **Step 2: Make it executable**

```bash
chmod +x hooks/guard-multimedia.sh
```

- [ ] **Step 3: Test the hook manually**

```bash
# Should block
echo '{"tool_name":"Read","tool_input":{"file_path":"/tmp/test.png"}}' | bash hooks/guard-multimedia.sh

# Should pass through (empty output)
echo '{"tool_name":"Read","tool_input":{"file_path":"/tmp/test.rs"}}' | bash hooks/guard-multimedia.sh

# Should pass through (not Read tool)
echo '{"tool_name":"Grep","tool_input":{}}' | bash hooks/guard-multimedia.sh
```

Expected for PNG: `{"decision":"block","reason":"Multimedia file detected. Use read_media(file_path=\"/tmp/test.png\") from media-mcp tool instead of Read. The Read tool cannot process this file type and will cause an API error."}`
Expected for .rs: empty output
Expected for Grep: empty output

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "feat: add PreToolUse bash hook for multimedia file blocking"
```

---

### Task 8: Integration Test & Claude Code Setup

**Files:**
- Create: `test_assets/` (test images)
- Modify: `~/.claude/settings.json`

- [ ] **Step 1: Create a test image**

```bash
mkdir -p test_assets
# Create a simple PNG test image using Python (if available)
python3 -c "
from PIL import Image, ImageDraw, ImageFont
img = Image.new('RGB', (400, 200), color='white')
d = ImageDraw.Draw(img)
d.text((50, 80), 'Hello Media MCP!', fill='black')
img.save('test_assets/test.png')
" 2>/dev/null || echo "test image placeholder" > test_assets/test.txt
```

If Python/PIL not available, skip to step 2 (manual test with any existing image).

- [ ] **Step 2: Do a full build**

```bash
cargo build --release 2>&1
```

Expected: `Finished release [optimized + debuginfo]`

- [ ] **Step 3: Create a test config**

```bash
mkdir -p ~/.config/media-mcp
cp config.json.example ~/.config/media-mcp/config.json
# Edit with real API key
```

- [ ] **Step 4: Smoke test the MCP server**

```bash
# Test that the binary starts and responds to initialize
echo '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"test","version":"1.0"}}}' | timeout 5 ./target/release/media-mcp --config ~/.config/media-mcp/config.json 2>/dev/null
```

Expected: JSON response with server info

- [ ] **Step 5: Add hook to Claude Code settings**

Add to `~/.claude/settings.json` under the top level:

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Read",
        "hooks": [
          {
            "type": "command",
            "command": "bash D:/workspace/RustProjects/media-mcp/hooks/guard-multimedia.sh"
          }
        ]
      }
    ]
  }
}
```

- [ ] **Step 6: Add MCP server to Claude Code settings**

Add to `~/.claude/settings.json` under the top level:

```json
{
  "mcpServers": {
    "media-mcp": {
      "command": "D:/workspace/RustProjects/media-mcp/target/release/media-mcp.exe",
      "args": ["--config", "D:/workspace/RustProjects/media-mcp/config.json"]
    }
  }
}
```

- [ ] **Step 7: Verify in Claude Code**

Restart Claude Code, then:
1. Try to read a PNG file with the Read tool — hook should block it
2. Call `read_media(file_path="/path/to/test.png")` — should return metadata + OCR + description
3. Call `list_supported_types` — should return type list and status

- [ ] **Step 8: Final commit**

```bash
git add -A
git commit -m "feat: complete media-mcp v0.1 with Claude Code integration"
```
