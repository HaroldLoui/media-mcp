---
name: multi-ocr-engine-and-installation-guide
description: Pluggable OCR engine architecture (Tesseract + PaddleOCR) and user-facing installation documentation
---

# Multi-OCR Engine & Installation Guide

## Problem

1. OCR is currently hardcoded to tesseract via direct CLI calls. Adding more engines (PaddleOCR, etc.) requires changing handler code.
2. No installation documentation exists for end users who want to deploy this MCP server.

## Multi-OCR Engine Design

### OcrEngine Trait

```rust
pub trait OcrEngine: Send + Sync {
    fn name(&self) -> &str;
    fn run(&self, file_path: &str, language: &str) -> OcrResult;
    fn run_with_confidence(&self, file_path: &str, language: &str) -> OcrResultWithConfidence;
    fn is_available(&self) -> bool;
}
```

### Engine Registry

A registry module that:
- Discovers available engines at startup (checks `is_available()`)
- Selects engine based on config `ocr.default_engine` ("auto" | "tesseract" | "paddle")
- Auto mode: iterates engines by priority, uses first available one (Paddle > Tesseract)

### File Structure

```
src/tools/ocr/
├── mod.rs          # OcrEngine trait, EngineRegistry, get_engine()
├── tesseract.rs    # TesseractEngine (moved from current ocr.rs)
└── paddle.rs       # PaddleEngine (new)
```

### PaddleEngine

Calls `paddleocr` CLI (same pattern as tesseract):

```bash
paddleocr --image xxx.png --lang ch --use_angle_cls true --use_gpu false
```

PaddleOCR outputs JSON array — parse and extract text.

### Config Changes

```json
{
  "ocr": {
    "default_engine": "auto",
    "engines": {
      "tesseract": {
        "tesseract_cmd": null,
        "languages": ["chi_sim", "eng"]
      },
      "paddle": {
        "paddle_cmd": null,
        "lang": "ch"
      }
    },
    "confidence_threshold": 60
  }
}
```

Auto mode: checks `tesseract_cmd`/PATH for tesseract, `paddle_cmd`/PATH for paddleocr.

## Installation Document

File: `docs/INSTALL.md`

Sections as approved:

1. **适用场景** — for non-multimodal model users in Claude Code
2. **快速开始** — download release, config, MCP registration, verify
3. **OCR 引擎安装（可选）** — Tesseract + PaddleOCR
4. **从源码编译** — git clone + cargo build --release
5. **Hook 安装（可选）** — guard-multimedia.sh PreToolUse
6. **配置说明** — all fields explained
7. **常见问题**

## Changes Required

### Code Changes
- `src/tools/ocr.rs` → `src/tools/ocr/` (directory module)
- `src/tools/ocr/mod.rs` — trait + registry
- `src/tools/ocr/tesseract.rs` — existing code moved
- `src/tools/ocr/paddle.rs` — new PaddleEngine
- `src/config.rs` — update OcrConfig structure
- `src/handlers.rs` — use engine registry instead of direct ocr::run_ocr()

### Documentation
- `docs/INSTALL.md` — new installation guide
- `CLAUDE.md` — update if needed

## Error Handling

- Unknown engine in config → fallback to auto detection with warning
- No engine available → clear error message telling user to install tesseract or paddleocr
- PaddleOCR missing `paddle_cmd` → check PATH
- New engines can be added without touching handlers (just implement trait + register)

## Open Questions

- None — design approved by user on 2026-06-08.
