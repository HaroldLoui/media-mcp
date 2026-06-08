use std::path::Path;
use std::process::Command;
use super::{OcrResult, OcrResultWithConfidence, OcrEngine, validate_image};

/// Find tesseract executable in common locations
fn find_tesseract(cmd: &Option<String>) -> Option<String> {
    if cmd.as_ref().is_some_and(|p| Path::new(p).exists()) {
        return cmd.clone();
    }
    // Check PATH first
    if let Ok(output) = Command::new("tesseract").arg("--version").output()
        && output.status.success()
    {
        return Some("tesseract".to_string());
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
        if let Err(w) = validate_image(file_path) {
            return OcrResult { text: None, warning: Some(w) };
        }

        let tess_exe = match find_tesseract(&self.cmd) {
            Some(exe) => exe,
            None => {
                return OcrResult {
                    text: None,
                    warning: Some(
                        "Tesseract not found in PATH or common install locations. \
                         Install from https://github.com/UB-Mannheim/tesseract/wiki"
                            .to_string(),
                    ),
                };
            }
        };

        let output = match Command::new(&tess_exe)
            .arg(file_path)
            .arg("stdout")
            .arg("-l")
            .arg(language)
            .arg("--psm")
            .arg("3")
            .output()
        {
            Ok(o) => o,
            Err(e) => {
                return OcrResult {
                    text: None,
                    warning: Some(format!("Failed to execute tesseract: {:?}", e)),
                };
            }
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
        if let Err(w) = validate_image(file_path) {
            return OcrResultWithConfidence {
                text: None,
                warning: Some(w),
                average_confidence: 0.0,
            };
        }

        let tess_exe = match find_tesseract(&self.cmd) {
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
            .arg(language)
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

        // Parse TSV output: level, page_num, block_num, par_num, line_num, word_num, left, top, width, height, conf, text
        let tsv = String::from_utf8_lossy(&output.stdout);
        let mut confidences: Vec<f64> = Vec::new();
        let mut text_parts: Vec<String> = Vec::new();

        for line in tsv.lines().skip(1) {
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
}
