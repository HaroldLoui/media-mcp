# OCR-Vision API Fallback Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `vision` parameter to `read_media` that controls whether Vision API is called, with an "auto" mode that uses OCR confidence to decide.

**Architecture:** Three-file change. `ocr.rs` gets a new function that runs tesseract in TSV mode and returns an average confidence score. `config.rs` gets an optional `confidence_threshold` field. `handlers.rs` gets the `vision` parameter and fallback logic.

**Tech Stack:** Rust, rmcp 1.7, tesseract (CLI), serde, reqwest

---

### Task 1: Add confidence parsing to `src/tools/ocr.rs`

**Files:**
- Modify: `src/tools/ocr.rs`

- [ ] **Step 1: Add new types and function signature**

Add after `OcrResult`:

```rust
pub struct OcrResultWithConfidence {
    pub text: Option<String>,
    pub warning: Option<String>,
    pub average_confidence: f64,
}
```

- [ ] **Step 2: Implement `run_ocr_with_confidence`**

Add the new function after the existing `is_available()`. It calls tesseract with `tsv` output mode, parses the TSV, extracts per-word confidence values (level 5 rows with conf != -1), and returns the average:

```rust
/// Run OCR with TSV confidence scoring. Returns extracted text and average confidence.
/// average_confidence is 0.0 if no words were recognized.
pub fn run_ocr_with_confidence(file_path: &str, languages: &str) -> OcrResultWithConfidence {
    let path = Path::new(file_path);

    if !path.exists() {
        return OcrResultWithConfidence {
            text: None,
            warning: Some(format!("File not found: {}", file_path)),
            average_confidence: 0.0,
        };
    }

    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    let ocrable = ["png", "jpg", "jpeg", "bmp", "tiff", "tif", "webp"];
    if !ocrable.contains(&ext.as_str()) {
        return OcrResultWithConfidence {
            text: None,
            warning: Some(format!("OCR not supported for .{} files", ext)),
            average_confidence: 0.0,
        };
    }

    let tess_exe = match find_tesseract() {
        Some(exe) => exe,
        None => {
            return OcrResultWithConfidence {
                text: None,
                warning: Some("Tesseract not found".to_string()),
                average_confidence: 0.0,
            };
        }
    };

    // Run tesseract with TSV output format
    let output = match Command::new(&tess_exe)
        .arg(file_path)
        .arg("stdout")
        .arg("-l")
        .arg(languages)
        .arg("--psm")
        .arg("3")
        .arg("tsv")
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            return OcrResultWithConfidence {
                text: None,
                warning: Some(format!("Failed to execute tesseract: {:?}", e)),
                average_confidence: 0.0,
            };
        }
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return OcrResultWithConfidence {
            text: None,
            warning: Some(format!("Tesseract failed: {}", stderr.trim())),
            average_confidence: 0.0,
        };
    }

    // Parse TSV output
    // TSV columns: level	page_num	block_num	par_num	line_num	word_num	left	top	width	height	conf	text
    // We care about level=5 (word level) rows with conf != -1
    let tsv = String::from_utf8_lossy(&output.stdout);
    let mut confidences: Vec<f64> = Vec::new();
    let mut text_parts: Vec<String> = Vec::new();

    for line in tsv.lines().skip(1) {
        // Skip empty lines
        if line.trim().is_empty() {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 12 {
            continue;
        }
        let level: i32 = cols[0].parse().unwrap_or(0);
        let conf: f64 = cols[10].parse().unwrap_or(-1.0);

        if level == 5 {
            if conf >= 0.0 {
                confidences.push(conf);
            }
            let word = cols[11].trim();
            if !word.is_empty() {
                text_parts.push(word.to_string());
            }
        }
    }

    let full_text = text_parts.join(" ");

    if full_text.trim().is_empty() {
        return OcrResultWithConfidence {
            text: None,
            warning: Some("OCR returned empty text".to_string()),
            average_confidence: 0.0,
        };
    }

    let avg_conf = if confidences.is_empty() {
        0.0
    } else {
        confidences.iter().sum::<f64>() / confidences.len() as f64
    };

    OcrResultWithConfidence {
        text: Some(full_text),
        warning: None,
        average_confidence: avg_conf,
    }
}
```

- [ ] **Step 3: Verify it compiles**

Run: `cargo check` — expect success (the new function is unused yet, but compiler shouldn't error)

- [ ] **Step 4: Commit**

```bash
git add src/tools/ocr.rs
git commit -m "feat: add run_ocr_with_confidence for TSV-based confidence scoring"
```

---

### Task 2: Add confidence_threshold to config

**Files:**
- Modify: `src/config.rs`

- [ ] **Step 1: Add `confidence_threshold` field to `OcrConfig`**

```rust
#[derive(Debug, Clone, Deserialize)]
pub struct OcrConfig {
    #[serde(default = "default_languages")]
    pub languages: Vec<String>,
    pub tesseract_cmd: Option<String>,
    #[serde(default = "default_confidence_threshold")]
    pub confidence_threshold: u32,
}

fn default_confidence_threshold() -> u32 {
    60
}
```

- [ ] **Step 2: Verify it compiles**

Run: `cargo check` — expect success

- [ ] **Step 3: Commit**

```bash
git add src/config.rs
git commit -m "feat: add confidence_threshold config to OcrConfig"
```

---

### Task 3: Implement the vision parameter and fallback logic in handlers

**Files:**
- Modify: `src/handlers.rs`

- [ ] **Step 1: Add `vision` field to `ReadMediaRequest`**

```rust
#[derive(Debug, Deserialize, schemars::JsonSchema)]
pub struct ReadMediaRequest {
    #[schemars(description = "Absolute path to the multimedia file")]
    pub file_path: String,
    #[schemars(description = "OCR language hint, e.g. 'chi_sim+eng'")]
    pub language: Option<String>,
    #[schemars(description = "Vision API mode: 'auto' (default, OCR-based fallback), 'always' (force Vision), 'skip' (OCR only)")]
    pub vision: Option<String>,
}
```

- [ ] **Step 2: Update the `read_media` signature to extract vision parameter**

Change the destructuring to include `vision`:

```rust
async fn read_media(
    &self,
    Parameters(ReadMediaRequest { file_path, language, vision }): Parameters<ReadMediaRequest>,
) -> String {
```

- [ ] **Step 3: Replace the OCR + Vision sections with the fallback logic**

Replace lines 95-121 (the OCR and Vision sections) with:

```rust
        let mut warnings: Vec<String> = Vec::new();

        // Determine vision mode
        let vision_mode = vision.as_deref().unwrap_or("auto");

        // 2. OCR (for images) — always run OCR for images
        let mut ocr_text: Option<String> = None;
        let mut need_vision = vision_mode == "always";

        if meta.mime_type.starts_with("image/") {
            if vision_mode == "skip" || vision_mode == "always" {
                // Plain OCR, no confidence needed
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
```

- [ ] **Step 4: Build to verify**

Run: `cargo check` — expect success

- [ ] **Step 5: Commit**

```bash
git add src/handlers.rs
git commit -m "feat: add vision parameter with auto/skip/always to read_media"
```

---

### Task 4: Update config.json with the new field

**Files:**
- Modify: `config.json`

- [ ] **Step 1: Add `confidence_threshold` to the OCR section**

```json
"ocr": {
    "languages": ["chi_sim", "eng"],
    "tesseract_cmd": null,
    "confidence_threshold": 60
}
```

- [ ] **Step 2: Commit**

```bash
git add config.json
git commit -m "chore: add confidence_threshold to config.json"
```

---

### Task 5: Test end-to-end

**Files:**
- Test: `src/bin/test_ocr.rs` (already exists)

- [ ] **Step 1: Update test_ocr to verify auto mode skips Vision for text images**

Modify `src/bin/test_ocr.rs` to send a second call with `vision="always"` and verify the difference:

```rust
    // Test 1: default (auto) — should skip Vision for text-heavy images
    let call1 = r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"read_media","arguments":{"file_path":"D:\\workspace\\RustProjects\\media-mcp\\ScreenShot_2026-06-05_164142_386.png","language":"chi_sim+eng"}}}"#;
    let resp1 = send_and_read(&mut stdin, &mut stdout, call1);
    println!("=== AUTO mode ===");
    if resp1.contains("ai_description") {
        println!("Vision was used (OCR confidence was low)");
    } else {
        println!("OCR only (confidence was high enough)");
    }
    // Truncate to avoid flooding terminal
    println!("Response snippet: {}", &resp1[..500.min(resp1.len())]);
    println!();

    // Test 2: vision="always" — should force Vision
    let call2 = r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"read_media","arguments":{"file_path":"D:\\workspace\\RustProjects\\media-mcp\\ScreenShot_2026-06-05_164142_386.png","language":"chi_sim+eng","vision":"always"}}}"#;
    let resp2 = send_and_read(&mut stdin, &mut stdout, call2);
    println!("=== ALWAYS mode ===");
    println!("Has ai_description: {}", resp2.contains("ai_description"));
    println!("Response snippet: {}", &resp2[..500.min(resp2.len())]);
```

- [ ] **Step 2: Run the test**

Run: `cargo run --bin test_ocr`

Expected:
- Auto mode output should either have `ai_description` (if OCR confidence was low) or not (if confidence was high enough)
- Always mode output should definitely contain `ai_description`

- [ ] **Step 3: Commit**

```bash
git add src/bin/test_ocr.rs
git commit -m "test: update test_ocr to verify vision fallback modes"
```

---

## Spec Coverage Check

| Spec Requirement | Task |
|---|---|
| `vision` parameter on `read_media` | Task 3 Step 1 |
| `"auto"` mode: OCR + confidence check | Task 3 Step 3 |
| `"skip"` mode: OCR only | Task 3 Step 3 |
| `"always"` mode: OCR + Vision | Task 3 Step 3 |
| Confidence threshold from config (default 60) | Task 2 Step 1 |
| If tesseract unavailable, fall through to Vision | Covered in auto mode — OCR returns confidence 0.0 < threshold |
| OCR returns empty text → confidence is 0 → Vision runs | Covered in auto mode (average_confidence = 0.0) |
