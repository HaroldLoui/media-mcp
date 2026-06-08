# media-mcp

**MCP 服务器** — 让不支持多模态的模型也能"看懂"图片。

在 Claude Code 中使用不支持图片输入的大模型（如 DeepSeek v4）时，这个 MCP 工具可以将多媒体文件转为纯文本（元数据 + OCR 文字 + AI 描述），让模型能够理解图片内容。

## 功能

- **OCR 文字识别** — 支持 Tesseract 和 PaddleOCR 引擎
- **AI 图片描述** — 通过兼容 Anthropic 格式的 API 生成自然语言描述
- **智能降级** — 高置信度的纯文字图片只走 OCR（快），复杂图片自动回退到 Vision API
- **灵活配置** — 只用 OCR、只用 Vision API、或两者搭配使用

## 快速开始

### 安装

详细安装步骤见 [安装指南](docs/INSTALL.md)，包含：

1. 下载二进制或源码编译
2. 配置 `config.json`
3. 注册 MCP 服务器
4. 安装 OCR 引擎（可选）
5. 安装 PreToolUse Hook（可选）

### 配置示例

```json
{
  "vision_api": {
    "base_url": "https://your-api-endpoint.com/anthropic",
    "api_key": "your-api-key",
    "model": "mimo-v2.5"
  },
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

参考 [`config.json.example`](config.json.example)。

### 验证

连接 MCP 后在 Claude Code 中调用：

```
read_media("C:\path\to\image.png")
```

返回结果包含：
- `file` — 文件元数据（名称、大小、尺寸、MIME 类型）
- `ocr_text` — OCR 识别的文字内容
- `ai_description` — AI 生成的图片描述
- `warnings` — 告警信息

## 三种调用模式

`read_media` 支持 `vision` 参数：

| 模式 | 行为 |
|------|------|
| `"auto"`（默认） | 先 OCR，置信度低于阈值时自动回退到 Vision API |
| `"skip"` | 只跑 OCR，不用 Vision API（纯文字图片适用） |
| `"always"` | 强制 OCR + Vision API（模型需要理解图形内容时用） |

## OCR 引擎

| 引擎 | 安装 | 优先级 | 准确度 |
|------|------|--------|--------|
| **Tesseract** | `winget install` / `apt install` | 低 | 中等 |
| **PaddleOCR** | `pip install paddleocr` | 高 | 较高 |

引擎选择：`config.json` 中设置 `default_engine: "auto"` 自动检测可用引擎，PaddleOCR 优先。

## 项目结构

```
src/
├── main.rs              # 入口，启动 MCP 服务器
├── lib.rs               # 库 crate
├── config.rs            # 配置加载与结构定义
├── handlers.rs          # MCP 工具处理（read_media, list_supported_types）
└── tools/
    ├── mod.rs
    ├── metadata.rs      # 文件元数据提取
    ├── vision.rs        # Vision API 客户端
    └── ocr/
        ├── mod.rs       # OcrEngine trait + EngineRegistry
        ├── tesseract.rs # Tesseract 引擎
        └── paddle.rs    # PaddleOCR 引擎
hooks/
└── guard-multimedia.sh  # PreToolUse hook
docs/
├── INSTALL.md           # 安装指南
└── superpowers/         # 设计文档与实现计划
```

## 开发

```bash
cargo build --release
cargo test
cargo clippy
```

## 许可证

MIT OR Apache-2.0
