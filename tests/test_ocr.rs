/// Integration tests for OCR engines.
use media_mcp::tools::ocr::{self, tesseract::TesseractEngine, OcrEngine};

const TEST_IMAGE: &str = r"D:\workspace\RustProjects\media-mcp\ScreenShot_2026-06-05_164142_386.png";

fn test_engine() -> TesseractEngine {
    TesseractEngine::new(None)
}

#[test]
fn test_tesseract_engine_available() {
    let engine = test_engine();
    assert!(engine.is_available(), "Tesseract should be available on this system");
}

#[test]
fn test_ocr_confidence_returns_valid_data() {
    let engine = test_engine();
    let result = engine.run_with_confidence(TEST_IMAGE, "chi_sim+eng");

    assert!(result.text.is_some(), "OCR should return text for screenshot");
    assert!(result.text.as_ref().unwrap().len() > 10, "OCR text should be substantial");
    assert!(
        result.average_confidence >= 0.0 && result.average_confidence <= 100.0,
        "Confidence should be in valid range, got {}",
        result.average_confidence
    );
    assert!(result.warning.is_none(), "OCR should not have warnings: {:?}", result.warning);
}

#[test]
fn test_ocr_basic_run() {
    let engine = test_engine();
    let result = engine.run(TEST_IMAGE, "chi_sim+eng");

    assert!(result.text.is_some(), "OCR should return text for screenshot");
    assert!(result.text.as_ref().unwrap().len() > 10, "OCR text should be substantial");
}

#[test]
fn test_ocr_nonexistent_file() {
    let engine = test_engine();
    let result = engine.run_with_confidence(r"D:\nonexistent.png", "eng");

    assert!(result.text.is_none(), "No text for nonexistent file");
    assert!(result.warning.is_some(), "Should have warning for nonexistent file");
    assert_eq!(result.average_confidence, 0.0, "Confidence should be 0 for nonexistent file");
}

#[test]
fn test_ocr_unsupported_extension() {
    let engine = test_engine();
    let result = engine.run_with_confidence(r"D:\test.doc", "eng");

    assert!(result.text.is_none(), "No text for unsupported extension");
    assert!(result.warning.is_some(), "Should have warning for unsupported extension");
    assert_eq!(result.average_confidence, 0.0, "Confidence should be 0 for unsupported file");
}

#[test]
fn test_engine_registry() {
    use media_mcp::config::OcrConfig;
    use media_mcp::tools::ocr::EngineRegistry;

    // Create a minimal OCR config for registry test
    let config = OcrConfig {
        default_engine: "auto".to_string(),
        engines: media_mcp::config::OcrEngines::default(),
        confidence_threshold: 60,
    };
    let registry = EngineRegistry::new(&config);
    let engine = registry.get_engine("auto");
    assert!(engine.is_some(), "At least one OCR engine should be available");
}
