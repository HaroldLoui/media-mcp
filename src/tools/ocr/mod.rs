use std::path::Path;

pub mod tesseract;

pub struct OcrResult {
    pub text: Option<String>,
    pub warning: Option<String>,
}

pub struct OcrResultWithConfidence {
    pub text: Option<String>,
    pub warning: Option<String>,
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

        // Register engines in priority order
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
