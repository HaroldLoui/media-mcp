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

/// Find tesseract executable in common locations
fn find_tesseract() -> Option<String> {
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

/// Run OCR on an image file. Returns extracted text or None if OCR is unavailable.
/// Gracefully degrades: if tesseract is not installed or the feature is disabled,
/// returns None with a warning message.
pub fn run_ocr(file_path: &str, languages: &str) -> OcrResult {
    let path = Path::new(file_path);

    // Check file exists
    if !path.exists() {
        return OcrResult {
            text: None,
            warning: Some(format!("File not found: {}", file_path)),
        };
    }

    // Check extension
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let ocrable = ["png", "jpg", "jpeg", "bmp", "tiff", "tif", "webp"];
    if !ocrable.contains(&ext.as_str()) {
        return OcrResult {
            text: None,
            warning: Some(format!("OCR not supported for .{} files", ext)),
        };
    }

    // Find tesseract binary
    let tess_exe = match find_tesseract() {
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

    // Build tesseract command:
    // tesseract <image> stdout -l <lang> --psm 3 2>&1
    let output = match Command::new(&tess_exe)
        .arg(file_path)
        .arg("stdout")
        .arg("-l")
        .arg(languages)
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

/// Check if OCR (tesseract) is available on this system.
pub fn is_available() -> bool {
    find_tesseract().is_some()
}

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

    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
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
    // TSV columns: level, page_num, block_num, par_num, line_num, word_num, left, top, width, height, conf, text
    // We care about level=5 (word level) rows with conf != -1
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
