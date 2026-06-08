use std::path::Path;

pub mod tesseract;
pub mod paddle;

/// Result from an OCR engine with extracted text.
pub struct OcrResult {
    /// Extracted text, if any was found.
    pub text: Option<String>,
    /// Warning message if OCR ran with issues (e.g., engine not available).
    pub warning: Option<String>,
}

/// Result from an OCR engine with extracted text and confidence score.
pub struct OcrResultWithConfidence {
    /// Extracted text, if any was found.
    pub text: Option<String>,
    /// Warning message if OCR ran with issues.
    pub warning: Option<String>,
    /// Average confidence score (0.0–100.0). 0.0 if no words were recognized.
    pub average_confidence: f64,
}

/// Check if file exists and extension is supported for OCR.
pub fn validate_image(file_path: &str) -> Result<(), String> {
    let path = Path::new(file_path);
    if !path.exists() {
        return Err(format!("File not found: {}", file_path));
    }
    let ext = path
        .extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .unwrap_or_default();
    let ocrable = ["png", "jpg", "jpeg", "bmp", "tiff", "tif", "webp"];
    if !ocrable.contains(&ext.as_str()) {
        return Err(format!("OCR not supported for .{} files", ext));
    }
    Ok(())
}

/// Pluggable OCR engine interface.
///
/// Implementations call external OCR tools (tesseract, paddleocr, etc.)
/// via CLI subprocess and return extracted text.
pub trait OcrEngine: Send + Sync {
    /// Engine identifier (e.g., "tesseract", "paddle").
    fn name(&self) -> &str;
    /// Run OCR and return extracted text.
    fn run(&self, file_path: &str, language: &str) -> OcrResult;
    /// Run OCR with confidence scoring. Used for Vision API fallback detection.
    fn run_with_confidence(&self, file_path: &str, language: &str) -> OcrResultWithConfidence;
    /// Check if this engine is available (binary found in PATH or configured path).
    fn is_available(&self) -> bool;
}

pub struct EngineRegistry {
    engines: Vec<Box<dyn OcrEngine>>,
}

impl EngineRegistry {
    pub fn new(config: &crate::config::OcrConfig) -> Self {
        let mut engines: Vec<Box<dyn OcrEngine>> = Vec::new();

        // Register engines in priority order (paddle first — higher accuracy)
        let paddle = paddle::PaddleEngine::new(
            config.engines.paddle.paddle_cmd.clone(),
            config.engines.paddle.lang.clone(),
        );
        engines.push(Box::new(paddle) as Box<dyn OcrEngine>);
        let tesseract = tesseract::TesseractEngine::new(
            config.engines.tesseract.tesseract_cmd.clone(),
        );
        engines.push(Box::new(tesseract) as Box<dyn OcrEngine>);

        EngineRegistry { engines }
    }

    pub fn get_engine(&self, name: &str) -> Option<&dyn OcrEngine> {
        if name == "auto" {
            self.engines.iter().find(|e| e.is_available()).map(|e| e.as_ref())
        } else {
            self.engines.iter().find(|e| e.name() == name).map(|e| e.as_ref())
        }
    }

    pub fn available_engines(&self) -> Vec<&str> {
        self.engines
            .iter()
            .filter(|e| e.is_available())
            .map(|e| e.name())
            .collect()
    }
}
