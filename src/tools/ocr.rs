pub struct OcrResult {
    pub text: Option<String>,
    pub warning: Option<String>,
}

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
    use std::path::Path;
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
