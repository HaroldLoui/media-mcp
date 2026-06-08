# Multi-OCR Engine & Installation Guide Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor OCR into a pluggable engine architecture (trait + Tesseract/Paddle implementations) and write user-facing installation documentation.

**Architecture:** Split `src/tools/ocr.rs` into `src/tools/ocr/` directory module. Define an `OcrEngine` trait. Move existing tesseract code into `TesseractEngine`. Add `PaddleEngine` for PaddleOCR CLI integration. Add an `EngineRegistry` that discovers available engines and selects one based on config. The `Config` structure changes to support per-engine settings under an `engines` map.

**Tech Stack:** Rust, tesseract CLI, paddleocr CLI, serde

---

### Task 1: Create OcrEngine trait and EngineRegistry

**Files:**
- Create: `src/tools/ocr/mod.rs`
- Delete: `src/tools/ocr.rs` (old flat file)

- [ ] **Step 1: Remove old `src/tools/ocr.rs` and create `src/tools/ocr/mod.rs`**

Delete `src/tools/ocr.rs` and create `src/tools/ocr/mod.rs` with the shared types and trait:

```rust
use std::path::Path;
use std::process::Command;

pub struct OcrResult {
    pub text: Option<String>,
    pub warning: Option<String>,
}

pub struct OcrResultWithConfidence {
    pub text: Option<String>,
    pub warning: Option<String>,
    pub average_confidence: f64,
}

/// Common validation: check file exists and extension is supported
fn validate_image(file_path: &str) -> Result<(), String> {
    let path = Path::new(file_path);
    if !path.exists() {
        return Err(format!("File not found: {}", file_path));
    }
    let ext = path.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let ocrable = ["png", "jpg", "jpeg", "bmp", "tiff", "tif", "webp"];
    if !ocrable.contains(&ext.as_str()) {
        return Err(format!("OCR not supported for .{} files", ext));
    }
    Ok(())
}

pub trait OcrEngine: Send + Sync {
    fn name(&self) -> &str;
    fn run(&self, file_path: &str, language: &str) -> OcrResult;
    fn run_with_confidence(&self, file_path: &str, language: &str) -> OcrResultWithConfidence;
    fn is_available(&self) -> bool;
}

pub struct EngineRegistry {
    engines: Vec<Box<dyn OcrEngine>>,
}

impl EngineRegistry {
    pub fn new(config: &crate::config::OcrConfig) -> Self {
        let mut engines: Vec<Box<dyn OcrEngine>> = Vec::new();

        // Register engines in priority order (higher priority first)
        if let Some(engine) = create_engine("paddle", config) {
            engines.push(engine);
        }
        if let Some(engine) = create_engine("tesseract", config) {
            engines.push(engine);
        }

        EngineRegistry { engines }
    }

    pub fn get_engine(&self, name: &str) -> Option<&dyn OcrEngine> {
        if name == "auto" {
            // Return first available engine
            self.engines.iter().find(|e| e.is_available()).map(|e| e.as_ref())
        } else {
            self.engines.iter().find(|e| e.name() == name).map(|e| e.as_ref())
        }
    }

    pub fn available_engines(&self) -> Vec<&str> {
        self.engines.iter().filter(|e| e.is_available()).map(|e| e.name()).collect()
    }
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check`
Expected: compile errors because `create_engine` function doesn't exist yet and module files are missing

- [ ] **Step 3: Commit**

```bash
git rm src/tools/ocr.rs
git add src/tools/ocr/mod.rs
git commit -m "refactor: create OcrEngine trait and EngineRegistry skeleton"
```

---

### Task 2: Implement TesseractEngine

**Files:**
- Create: `src/tools/ocr/tesseract.rs`

- [ ] **Step 1: Write TesseractEngine implementing OcrEngine**

```rust
use std::path::Path;
use std::process::Command;
use super::{OcrResult, OcrResultWithConfidence, OcrEngine, validate_image};

/// Find tesseract executable in common locations
fn find_tesseract(paddle_cmd: &Option<String>) -> Option<String> {
    // If configured, use the configured path
    if let Some(cmd) = paddle_cmd {
        if Path::new(cmd).exists() {
            return Some(cmd.clone());
        }
    }
    // Check PATH first
    if let Ok(output) = Command::new("tesseract").arg("--version").output() {
        if output.status.success() {
            return Some("tesseract".to_string());
        }
    }
    // Common Windows install paths
    let candidates = [
        r"C:\Program Files\Tesseract-OCR\tesseract.exe",
        r"C:\Program Files (x86)\Tesseract-OCR\tesseract.exe",
    ];
    for path in &candidates {
        if Path::new(path).exists() {
            return Some(path.to_string());
        }
    }
    None
}

pub struct TesseractEngine {
    cmd: Option<String>,
}

impl TesseractEngine {
    pub fn new(cmd: Option<String>) -> Self {
        TesseractEngine { cmd }
    }
}

impl OcrEngine for TesseractEngine {
    fn name(&self) -> &str {
        "tesseract"
    }

    fn is_available(&self) -> bool {
        find_tesseract(&self.cmd).is_some()
    }

    fn run(&self, file_path: &str, language: &str) -> OcrResult {
        let tess_exe = match find_tesseract(&self.cmd) {
            Some(exe) => exe,
            None => return OcrResult {
                text: None,
                warning: Some("Tesseract not found".to_string()),
            },
        };
        let output = match Command::new(&tess_exe)
            .arg(file_path).arg("stdout")
            .arg("-l").arg(language)
            .arg("--psm").arg("3")
            .output()
        {
            Ok(o) => o,
            Err(e) => return OcrResult {
                text: None,
                warning: Some(format!("Failed to execute tesseract: {:?}", e)),
            },
        };
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return OcrResult {
                text: None,
                warning: Some(format!("Tesseract failed: {}", stderr.trim())),
            };
        }
        let text = String::from_utf8_lossy(&output.stdout).to_string();
        let trimmed = text.trim();
        if trimmed.is_empty() {
            OcrResult { text: None, warning: Some("OCR returned empty text".to_string()) }
        } else {
            OcrResult { text: Some(trimmed.to_string()), warning: None }
        }
    }

    fn run_with_confidence(&self, file_path: &str, language: &str) -> OcrResultWithConfidence {
        run_tesseract_tsv(file_path, language, &self.cmd)
    }
}

/// Run tesseract with TSV output and return confidence scores
fn tesseract_tsv(file_path: &str, language: &str, cmd: &Option<String>) -> OcrResultWithConfidence {
    // Same implementation as current run_ocr_with_confidence in ocr.rs
    // ... (copy the function body from existing run_ocr_with_confidence)
}
```

Copy the full `run_ocr_with_confidence` body from the current `src/tools/ocr.rs` into `tesseract_tsv`.

- [ ] **Step 2: Register TesseractEngine in the registry**

Add to `src/tools/ocr/mod.rs`:

```rust
mod tesseract;
mod paddle;
pub use tesseract::TesseractEngine;
pub use paddle::PaddleEngine;

fn create_engine(name: &str, config: &crate::config::OcrConfig) -> Option<Box<dyn OcrEngine>> {
    match name {
        "tesseract" => {
            let engine = TesseractEngine::new(config.engines.tesseract.tesseract_cmd.clone());
            Some(Box::new(engine))
        }
        "paddle" => {
            let engine = PaddleEngine::new(config.engines.paddle.paddle_cmd.clone(), config.engines.paddle.lang.clone());
            Some(Box::new(engine))
        }
        _ => None,
    }
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check` — expect success (the PaddleEngine stub will warn about unused, but that's OK)

- [ ] **Step 4: Commit**

```bash
git add src/tools/ocr/tesseract.rs src/tools/ocr/mod.rs
git commit -m "feat: add TesseractEngine implementing OcrEngine trait"
```

---

### Task 3: Implement PaddleEngine (stub)

**Files:**
- Create: `src/tools/ocr/paddle.rs`

- [ ] **Step 1: Write PaddleEngine**

```rust
use std::path::Path;
use std::process::Command;
use super::{OcrResult, OcrResultWithConfidence, OcrEngine, validate_image};

/// Find paddleocr executable
fn find_paddleocr(cmd: &Option<String>) -> Option<String> {
    if let Some(path) = cmd {
        if Path::new(path).exists() {
            return Some(path.clone());
        }
    }
    // Check PATH for paddleocr
    if let Ok(output) = Command::new("paddleocr").arg("--help").output() {
        if output.status.success() {
            return Some("paddleocr".to_string());
        }
    }
    None
}

pub struct PaddleEngine {
    cmd: Option<String>,
    lang: String,
}

impl PaddleEngine {
    pub fn new(cmd: Option<String>, lang: Option<String>) -> Self {
        PaddleEngine {
            cmd,
            lang: lang.unwrap_or_else(|| "ch".to_string()),
        }
    }
}

impl OcrEngine for PaddleEngine {
    fn name(&self) -> &str {
        "paddle"
    }

    fn is_available(&self) -> bool {
        find_paddleocr(&self.cmd).is_some()
    }

    fn run(&self, file_path: &str, _language: &str) -> OcrResult {
        let exe = match find_paddleocr(&self.cmd) {
            Some(e) => e,
            None => return OcrResult {
                text: None,
                warning: Some("PaddleOCR not found. Install with: pip install paddleocr".to_string()),
            },
        };

        // paddleocr --image xxx.png --lang ch --use_angle_cls true --use_gpu false
        let output = match Command::new(&exe)
            .arg("--image").arg(file_path)
            .arg("--lang").arg(&self.lang)
            .arg("--use_angle_cls").arg("true")
            .arg("--use_gpu").arg("false")
            .output()
        {
            Ok(o) => o,
            Err(e) => return OcrResult {
                text: None,
                warning: Some(format!("Failed to execute PaddleOCR: {:?}", e)),
            },
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return OcrResult {
                text: None,
                warning: Some(format!("PaddleOCR failed: {}", stderr.trim())),
            };
        }

        // PaddleOCR outputs JSON array: [[[x,y,x,y], (text, confidence)], ...]
        // Parse to extract text
        let stdout = String::from_utf8_lossy(&output.stdout);
        let text = extract_paddle_text(&stdout);

        if text.is_empty() {
            OcrResult { text: None, warning: Some("PaddleOCR returned empty text".to_string()) }
        } else {
            OcrResult { text: Some(text), warning: None }
        }
    }

    fn run_with_confidence(&self, file_path: &str, language: &str) -> OcrResultWithConfidence {
        let result = self.run(file_path, language);
        // PaddleOCR doesn't expose average confidence easily, use 0.0 fallback
        OcrResultWithConfidence {
            text: result.text,
            warning: result.warning,
            average_confidence: 0.0,
        }
    }
}

/// Extract text from PaddleOCR JSON output
fn extract_paddle_text(output: &str) -> String {
    // PaddleOCR outputs: [[[bbox], (text, conf)], ...]
    // Simple regex-less extraction: find quoted text between ( and ,
    output.lines()
        .filter_map(|line| {
            let line = line.trim();
            // Skip non-JSON lines
            if line.starts_with('[') || line.starts_with('"') {
                Some(line.to_string())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check` — expect success

- [ ] **Step 3: Commit**

```bash
git add src/tools/ocr/paddle.rs
git commit -m "feat: add PaddleOCR engine via CLI integration"
```

---

### Task 4: Update config.rs for new engine config structure

**Files:**
- Modify: `src/config.rs`

- [ ] **Step 1: Update OcrConfig with engines map and default_engine**

```rust
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
    #[serde(default)]
    pub paddle: PaddleConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TesseractConfig {
    pub tesseract_cmd: Option<String>,
    #[serde(default = "default_languages")]
    pub languages: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PaddleConfig {
    pub paddle_cmd: Option<String>,
    #[serde(default = "default_paddle_lang")]
    pub lang: Option<String>,
}

fn default_engine() -> String {
    "auto".to_string()
}

fn default_paddle_lang() -> Option<String> {
    Some("ch".to_string())
}

impl Default for TesseractConfig {
    fn default() -> Self {
        TesseractConfig {
            tesseract_cmd: None,
            languages: vec!["chi_sim".to_string(), "eng".to_string()],
        }
    }
}

impl Default for PaddleConfig {
    fn default() -> Self {
        PaddleConfig {
            paddle_cmd: None,
            lang: Some("ch".to_string()),
        }
    }
}
```

Keep existing `languages_string()` method (it now uses `self.engines.tesseract.languages`):

```rust
pub fn languages_string(&self) -> String {
    self.engines.tesseract.languages.join("+")
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check` — fix any errors

- [ ] **Step 3: Commit**

```bash
git add src/config.rs
git commit -m "feat: update OcrConfig with pluggable engine structure"
```

---

### Task 5: Update handlers.rs to use EngineRegistry

**Files:**
- Modify: `src/handlers.rs`

- [ ] **Step 1: Replace direct ocr module calls with engine registry**

```rust
// At the top of read_media, get the engine
let ocr_engine = ocr::EngineRegistry::new(&self.config.ocr);

// Replace:
//   ocr::run_ocr(...)  →  engine.run(...)
//   ocr::run_ocr_with_confidence(...)  →  engine.run_with_confidence(...)
//   ocr::is_available()  →  engine.get_engine("auto").is_some()
```

Specifically, in the OCR section (lines 107-131), replace:

```rust
if vision_mode == "skip" || vision_mode == "always" {
    // Plain OCR, no confidence check
    let result = ocr::run_ocr(&file_path, &lang);
    ...
} else {
    // "auto" mode: OCR with confidence check
    let result = ocr::run_ocr_with_confidence(&file_path, &lang);
    ...
}
```

With:

```rust
let engine = ocr_engine.get_engine(&self.config.ocr.default_engine);

if let Some(engine) = engine {
    if vision_mode == "skip" || vision_mode == "always" {
        let result = engine.run(&file_path, &lang);
        ...
    } else {
        let result = engine.run_with_confidence(&file_path, &lang);
        ...
    }
} else {
    warnings.push("No OCR engine available. Install Tesseract or PaddleOCR, or configure a Vision API.".to_string());
}
```

And update `list_supported_types`:

```rust
status["ocr_available"] = serde_json::json!({
    "available": engine_registry.get_engine("auto").is_some(),
    "engines": engine_registry.available_engines(),
});
```

- [ ] **Step 2: Build to verify**

Run: `cargo build` — fix any compilation errors

- [ ] **Step 3: Commit**

```bash
git add src/handlers.rs
git commit -m "feat: use EngineRegistry in handlers for pluggable OCR"
```

---

### Task 6: Update mod.rs and tests

**Files:**
- Modify: `src/tools/mod.rs`
- Modify: `tests/test_ocr.rs`

- [ ] **Step 1: Update `src/tools/mod.rs` (no change needed if directory module works)**

The directory module `src/tools/ocr/` with `mod.rs` is automatically discovered. The `pub mod ocr;` in `src/tools/mod.rs` already handles this correctly.

- [ ] **Step 2: Update `tests/test_ocr.rs` imports**

The test uses `media_mcp::tools::ocr::run_ocr` and `media_mcp::tools::ocr::run_ocr_with_confidence`. After refactoring, these are methods on engine instances. Update to use the engine:

```rust
use media_mcp::tools::ocr::{EngineRegistry, TesseractEngine};

// Test using TesseractEngine directly
#[test]
fn test_tesseract_engine() {
    let engine = TesseractEngine::new(None);
    assert!(engine.is_available());
    let result = engine.run(TEST_IMAGE, "chi_sim+eng");
    assert!(result.text.is_some());
}

#[test]
fn test_engine_registry() {
    let config = media_mcp::config::OcrConfig::default();
    let registry = EngineRegistry::new(&config);
    let engine = registry.get_engine("auto");
    assert!(engine.is_some(), "At least one OCR engine should be available");
}
```

- [ ] **Step 3: Run tests**

Run: `cargo test --test test_ocr` — expect all tests to pass

- [ ] **Step 4: Commit**

```bash
git add src/tools/mod.rs tests/test_ocr.rs
git commit -m "test: update tests for OCR engine trait"
```

---

### Task 7: Write installation guide

**Files:**
- Modify: `docs/INSTALL.md`

- [ ] **Step 1: Write the full installation document**

```markdown
# 安装指南

## 适用场景

本 MCP 服务主要为在 Claude Code 中使用**不支持多模态模型**（如 DeepSeek v4）的用户设计。

如果你使用的模型支持图片输入，则无需本工具。

你需要自行准备以下至少一项：
- **多模态 API**（如 mimov2、GPT-4V 等，兼容 Anthropic 格式）— 用于 `ai_description`
- **本地 OCR 引擎**（Tesseract / PaddleOCR，可选）— 用于 `ocr_text`

你也可以两者都配：OCR 负责文字提取，Vision API 负责复杂图片理解。
如果都不配，仅 `list_supported_types` 和 `file` 元数据可用。

---

## 一、快速开始（1 分钟体验）

### 1. 下载二进制

从 [Releases]() 页面下载对应平台的 `media-mcp` 可执行文件。

### 2. 配置 config.json

```json
{
  "vision_api": {
    "base_url": "https://your-api-endpoint.com/anthropic",
    "api_key": "your-api-key",
    "model": "mimo-v2.5"
  },
  "ocr": {
    "default_engine": "auto",
    "engines": {
      "tesseract": { "tesseract_cmd": null, "languages": ["chi_sim", "eng"] },
      "paddle": { "paddle_cmd": null, "lang": "ch" }
    },
    "confidence_threshold": 60
  }
}
```

参考 `config.json.example`。

### 3. 注册 MCP 服务器

在 Claude Code 中运行 `/mcp`，添加：

| 字段 | 值 |
|------|-----|
| Transport | stdio |
| Command | `D:\path\to\media-mcp.exe` |
| Args | `--config D:\path\to\config.json` |

或直接编辑项目下的 `.mcp.json`（或全局 `claude_desktop_config.json`）：

```json
{
  "mcpServers": {
    "media-mcp": {
      "type": "stdio",
      "command": "D:\\path\\to\\media-mcp.exe",
      "args": ["--config", "D:\\path\\to\\config.json"]
    }
  }
}
```

### 4. 验证

在对话中发送一张图片，或手动调用：

```
read_media("C:\\path\\to\\image.png")
```

如果一切正常，会返回 `file`、`ocr_text`、`ai_description`。

---

## 二、OCR 引擎安装（可选）

### Tesseract

**Windows：**
```bash
winget install UB-Mannheim.TesseractOCR
# 或从 https://github.com/UB-Mannheim/tesseract/wiki 下载安装包
```

**macOS：**
```bash
brew install tesseract tesseract-lang
```

**Linux：**
```bash
sudo apt install tesseract-ocr tesseract-ocr-chi-sim tesseract-ocr-eng
```

如果需要中文支持，下载 `chi_sim.traineddata` 放到 Tesseract 的 `tessdata` 目录。

### PaddleOCR

```bash
pip install paddleocr
```

首次运行会自动下载模型文件（约 100MB）。

---

## 三、从源码编译

```bash
git clone https://github.com/YOUR_USERNAME/media-mcp.git
cd media-mcp
cargo build --release
# 二进制在 target/release/media-mcp.exe
```

---

## 四、Hook 安装（可选，推荐）

`hooks/guard-multimedia.sh` 是一个 PreToolUse hook，可以在你使用 `Read` 工具读取图片时自动拦截并提示改用 `read_media`。

在 Claude Code 的 settings 中配置：

```json
{
  "hooks": {
    "PreToolUse": "D:\\path\\to\\media-mcp\\hooks\\guard-multimedia.sh"
  }
}
```

配置后效果：当 Claude 尝试用 `Read` 打开图片时，会提示改用 `read_media`。

---

## 五、配置说明

### vision_api

| 字段 | 说明 | 默认值 |
|------|------|--------|
| `base_url` | 兼容 Anthropic 格式的 API 地址 | — |
| `api_key` | API 密钥 | — |
| `model` | 模型名称 | `mimo-v2.5` |
| `max_tokens` | 描述最大 token 数 | `10240` |
| `timeout_seconds` | 请求超时时间 | `30` |

### ocr

| 字段 | 说明 | 默认值 |
|------|------|--------|
| `default_engine` | 引擎选择：`"auto"` / `"tesseract"` / `"paddle"` | `"auto"` |
| `confidence_threshold` | OCR 置信度阈值（低于此值则触发 Vision fallback） | `60` |
| `engines.tesseract.tesseract_cmd` | Tesseract 路径（null=自动查找） | `null` |
| `engines.tesseract.languages` | OCR 语言列表 | `["chi_sim", "eng"]` |
| `engines.paddle.paddle_cmd` | PaddleOCR 路径（null=自动查找） | `null` |
| `engines.paddle.lang` | PaddleOCR 语言 | `"ch"` |

### blocked_extensions

被 hook 拦截的多媒体文件后缀列表。

---

## 六、常见问题

### Q: 为什么 Vision API 返回空？

检查 `config.json` 中的 `vision_api.api_key` 和 `base_url` 是否正确。

### Q: OCR 不工作？

运行 `list_supported_types` 查看 `ocr_available` 状态。如果为 false，说明未找到 OCR 引擎。
确保 Tesseract 或 PaddleOCR 已安装且在 PATH 中。

### Q: 如何只用 OCR 不用 Vision API？

调用 `read_media` 时传入 `vision="skip"`，或在 config 中不配置 `vision_api`。

### Q: 如何只用 Vision API 不用 OCR？

调用 `read_media` 时传入 `vision="always"`，或安装 PaddleOCR 作为 OCR 引擎（PaddleOCR 准确度更高，置信度可能达到阈值以上从而跳过 Vision）。
```

- [ ] **Step 2: Commit**

```bash
git add docs/INSTALL.md
git commit -m "docs: add installation guide"
```

---

### Task 8: Update config.json.example

**Files:**
- Modify: `config.json.example`

- [ ] **Step 1: Update with new config structure**

```json
{
  "vision_api": {
    "base_url": "https://your-vision-api.example.com/anthropic",
    "api_key": "your-api-key-here",
    "model": "mimo-v2.5",
    "max_tokens": 10240,
    "timeout_seconds": 30
  },
  "ocr": {
    "default_engine": "auto",
    "engines": {
      "tesseract": {
        "tesseract_cmd": null,
        "languages": ["chi_sim", "eng"]
      },
      "paddle": {
        "paddle_cmd": null,
        "lang": "ch"
      }
    },
    "confidence_threshold": 60
  },
  "blocked_extensions": [
    ".png", ".jpg", ".jpeg", ".gif", ".bmp", ".webp", ".svg", ".ico", ".tiff",
    ".mp4", ".avi", ".mkv", ".mov", ".webm",
    ".mp3", ".wav", ".flac", ".aac", ".ogg",
    ".pdf"
  ]
}
```

- [ ] **Step 2: Commit**

```bash
git add config.json.example
git commit -m "chore: update config.json.example with OCR engine config"
```

---

## Spec Coverage Check

| Spec Requirement | Task |
|---|---|
| OcrEngine trait definition | Task 1 |
| EngineRegistry with auto detection | Task 1 |
| TesseractEngine (existing code moved) | Task 2 |
| PaddleEngine (CLI integration) | Task 3 |
| Config: default_engine + engines map | Task 4 |
| handlers.rs: use engine registry | Task 5 |
| tests updated for new API | Task 6 |
| docs/INSTALL.md: full installation guide | Task 7 |
| config.json.example: new config structure | Task 8 |
| Hook installation documented | Task 7 (INSTALL.md section 四) |
| Source compilation documented | Task 7 (INSTALL.md section 三) |
