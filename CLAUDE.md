# Media MCP

Rust MCP server for reading multimedia files as text (metadata + OCR + AI description), with a bash hook to prevent Claude Code's Read tool from sending images to non-multimodal models.

## Problem

When using models that don't support multimodal input (e.g. `mimo-v2.5-pro[1m]`), Claude Code's built-in `Read` tool tries to send images as base64, causing API errors that break the conversation.

## Solution

Two-layer defense:
1. **Hook layer**: PreToolUse bash hook intercepts Read calls on multimedia file extensions
2. **MCP layer**: `read_media` tool returns text-only results (metadata + OCR + AI description)

## Architecture

```
Claude Code Read("x.png") → Hook blocks → Model uses read_media() instead
read_media() → metadata + OCR (tesseract) + Vision API (mimo-v2-omni) → pure text
```

## Tech Stack

- Rust (edition 2024), rmcp 1.7, image 0.25, tesseract 0.15, reqwest 0.12
- Bash hook script for PreToolUse
- Vision API: Anthropic-compatible format, configurable endpoint

## Key Docs

- Spec: `docs/superpowers/specs/2026-06-05-media-mcp-design.md`
- Plan: `docs/superpowers/plans/2026-06-05-media-mcp-impl.md`

## MCP Tools

- `read_media(file_path, language?)` — read multimedia file, return metadata + OCR + AI description
- `list_supported_types()` — list supported types and check OCR/API availability

## Config

Config file: `~/.config/media-mcp/config.json` or `--config <path>` or `.media-mcp.json`

Key settings:
- `vision_api.base_url` / `api_key` / `model` — vision API endpoint
- `ocr.languages` — OCR language list (default: chi_sim+eng)
- `ocr.tesseract_cmd` — sets TESSDATA_PREFIX env var
- `blocked_extensions` — file extensions to block in Read hook

## MVP Scope (v0.1)

- ✅ Image files: metadata + OCR + AI description
- ✅ Bash hook for multimedia blocking
- ✅ Config file loading with graceful degradation
- v0.2: PDF text extraction, video frame extraction, audio metadata
