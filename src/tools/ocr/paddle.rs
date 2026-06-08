use std::path::Path;
use std::process::Command;
use super::{OcrResult, OcrResultWithConfidence, OcrEngine, validate_image};

/// Find paddleocr executable
fn find_paddleocr(cmd: &Option<String>) -> Option<String> {
    if cmd.as_ref().is_some_and(|p| Path::new(p).exists()) {
        return cmd.clone();
    }
    // Check PATH
    if let Ok(output) = Command::new("paddleocr").arg("--help").output()
        && output.status.success()
    {
        return Some("paddleocr".to_string());
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
        if let Err(w) = validate_image(file_path) {
            return OcrResult { text: None, warning: Some(w) };
        }

        let exe = match find_paddleocr(&self.cmd) {
            Some(e) => e,
            None => {
                return OcrResult {
                    text: None,
                    warning: Some("PaddleOCR not found. Install with: pip install paddleocr".to_string()),
                };
            }
        };

        // paddleocr --image xxx.png --lang ch --use_angle_cls true --use_gpu false
        let output = match Command::new(&exe)
            .arg("--image")
            .arg(file_path)
            .arg("--lang")
            .arg(&self.lang)
            .arg("--use_angle_cls")
            .arg("true")
            .arg("--use_gpu")
            .arg("false")
            .output()
        {
            Ok(o) => o,
            Err(e) => {
                return OcrResult {
                    text: None,
                    warning: Some(format!("Failed to execute PaddleOCR: {:?}", e)),
                };
            }
        };

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return OcrResult {
                text: None,
                warning: Some(format!("PaddleOCR failed: {}", stderr.trim())),
            };
        }

        let text = extract_paddle_text(&String::from_utf8_lossy(&output.stdout));

        if text.is_empty() {
            OcrResult { text: None, warning: Some("PaddleOCR returned empty text".to_string()) }
        } else {
            OcrResult { text: Some(text), warning: None }
        }
    }

    fn run_with_confidence(&self, file_path: &str, language: &str) -> OcrResultWithConfidence {
        // PaddleOCR CLI doesn't expose per-word confidence easily; use plain run
        let result = self.run(file_path, language);
        OcrResultWithConfidence {
            text: result.text,
            warning: result.warning,
            average_confidence: 0.0,
        }
    }
}

/// Extract text from PaddleOCR JSON output.
/// PaddleOCR outputs: [[[bbox], (text, confidence)], ...]
fn extract_paddle_text(output: &str) -> String {
    let mut lines = Vec::new();
    // Try to parse as JSON array
    if let Ok(data) = serde_json::from_str::<serde_json::Value>(output) {
        if let Some(entries) = data.as_array() {
            for entry in entries {
                if let Some(inner) = entry.as_array()
                    && inner.len() >= 2
                    && let Some(text_conf) = inner[1].as_array()
                    && text_conf.len() >= 2
                    && let Some(text) = text_conf[0].as_str()
                {
                    lines.push(text.to_string());
                }
            }
        }
    } else {
        // Fallback: just return raw output if not valid JSON
        return output.trim().to_string();
    }
    lines.join("\n")
}
