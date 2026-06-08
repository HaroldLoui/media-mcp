---
name: ocr-vision-fallback-design
description: Smart fallback between OCR and Vision API based on OCR confidence, with user-requestable Vision support
---

# OCR-Vision API Fallback Design

## Problem

`read_media` always runs both OCR (tesseract) and Vision API (mimo-v2.5) for every image, even when OCR alone suffices. This introduces unnecessary latency (5-20s per call) and API costs for text-heavy images, which is the primary use case (non-multimodal LLMs reading text from screenshots/documents).

## Design

Add a `vision` parameter to `read_media` that controls when Vision API is invoked:

```
read_media(file_path, language?, vision?)
```

### Vision Parameter

| Value     | Behavior |
|-----------|----------|
| `"auto"`  | (default) Run OCR, check confidence. Skip Vision if confidence ≥ threshold. |
| `"skip"`  | Run OCR only, always skip Vision. |
| `"always"`| Run OCR + Vision unconditionally. |

### Fallback Logic (auto mode)

1. Run OCR with tesseract in TSV mode to get per-word confidence scores
2. Calculate average confidence score. If OCR returns no text, treat confidence as 0 (Vision runs).
3. If `average_confidence ≥ threshold` (default 60, configurable via `ocr.confidence_threshold`) → treat as text image, return `{ ocr_text: ... }` without `ai_description`
4. If `average_confidence < threshold` → run Vision API, return `{ ocr_text: ..., ai_description: ... }`

### User Interaction Flow

When the model receives an OCR-only result and the user asks about visual content (objects, layout, charts), the model can re-call `read_media` with `vision="always"` to get an AI description. No separate tool needed.

## Changes Required

### `src/tools/ocr.rs`
- Modify `run_ocr` to return TSV-formatted output
- Parse TSV to extract per-word confidence values
- Add function `run_ocr_with_confidence(file_path, languages) -> (OcrResult, f64)`

### `src/handlers.rs`
- Add `vision: Option<String>` to `ReadMediaRequest`
- Modify `read_media` handler to implement the auto/skip/always logic
- Only call `vision::describe_image` when confidence is low or `vision="always"`

### `src/config.rs`
- Add optional `ocr.confidence_threshold` to config (default 60)
- Add `read_media.vision_default` to config (default "auto")

## Error Handling

- If tesseract is not installed or language data is missing, OCR returns `None`/warning, and `vision="auto"` falls through to Vision API (same as low confidence)
- If Vision API is misconfigured (empty API key), `vision="always"` returns a clear warning

## Open Questions

- None — design approved by user on 2026-06-08.
