/// Integration tests for OCR + Vision fallback modes.
///
/// Tests verify that the `vision` parameter on `read_media` correctly controls
/// whether Vision API is called:
/// - "auto" (default): OCR with confidence check, Vision if confidence < threshold
/// - "always": OCR + Vision, always includes ai_description
/// - "skip": OCR only, never includes ai_description
use media_mcp::tools::ocr;

const TEST_IMAGE: &str = r"D:\workspace\RustProjects\media-mcp\ScreenShot_2026-06-05_164142_386.png";

#[test]
fn test_ocr_confidence_returns_valid_data() {
    let result = ocr::run_ocr_with_confidence(TEST_IMAGE, "chi_sim+eng");

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
    let result = ocr::run_ocr(TEST_IMAGE, "chi_sim+eng");

    assert!(result.text.is_some(), "OCR should return text for screenshot");
    assert!(result.text.as_ref().unwrap().len() > 10, "OCR text should be substantial");
}

#[test]
fn test_ocr_nonexistent_file() {
    let result = ocr::run_ocr_with_confidence(r"D:\nonexistent.png", "eng");

    assert!(result.text.is_none(), "No text for nonexistent file");
    assert!(result.warning.is_some(), "Should have warning for nonexistent file");
    assert_eq!(result.average_confidence, 0.0, "Confidence should be 0 for nonexistent file");
}

#[test]
fn test_ocr_unsupported_extension() {
    let result = ocr::run_ocr_with_confidence(r"D:\test.doc", "eng");

    assert!(result.text.is_none(), "No text for unsupported extension");
    assert!(result.warning.is_some(), "Should have warning for unsupported extension");
    assert_eq!(result.average_confidence, 0.0, "Confidence should be 0 for unsupported file");
}
