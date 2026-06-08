# 安装指南

## 适用场景

本 MCP 服务主要为在 **Claude Code** 中使用**不支持多模态模型**（如 DeepSeek v4）的用户设计。

如果你使用的模型支持图片输入，则无需本工具。

你需要自行准备以下至少一项：
- **多模态 API**（如 mimo-v2.5、GPT-4V 等，兼容 Anthropic 格式）— 用于 `ai_description`
- **本地 OCR 引擎**（Tesseract / PaddleOCR，可选）— 用于 `ocr_text`

你也可以两者都配：OCR 负责文字提取，Vision API 负责复杂图片理解。
如果都不配，仅 `list_supported_types` 和 `file` 元数据可用。

---

## 一、快速开始（1 分钟体验）

### 1. 下载二进制

从 [Releases](https://github.com/YOUR_USERNAME/media-mcp/releases) 页面下载对应平台的 `media-mcp` 可执行文件。

### 2. 配置 config.json

```json
{
  "vision_api": {
    "base_url": "https://your-api-endpoint.com/anthropic",
    "api_key": "your-api-key",
    "model": "mimo-v2.5",
    "max_tokens": 10240,
    "timeout_seconds": 30
  },
  "ocr": {
    "default_engine": "auto",
    "engines": {
      "tesseract": {
        "tesseract_cmd": null,
        "languages": ["chi_sim", "eng"]
      }
    },
    "confidence_threshold": 60
  },
  "blocked_extensions": [
    ".png", ".jpg", ".jpeg", ".gif", ".bmp", ".webp",
    ".svg", ".ico", ".tiff", ".mp4", ".avi", ".mkv",
    ".mov", ".webm", ".mp3", ".wav", ".flac", ".aac",
    ".ogg", ".pdf"
  ]
}
```

参考 `config.json.example`。

### 3. 注册 MCP 服务器

在 Claude Code 中运行 `/mcp`，在界面中添加：

| 字段 | 值 |
|------|-----|
| Transport | `stdio` |
| Command | `D:\path\to\media-mcp.exe` |
| Args | `--config D:\path\to\config.json` |

或直接编辑项目下的 `.mcp.json`（或全局 `claude_desktop_config.json`）：

```json
{
  "mcpServers": {
    "media-mcp": {
      "type": "stdio",
      "command": "D:\\path\\to\\media-mcp.exe",
      "args": ["--config", "D:\\path\\to\\config.json"]
    }
  }
}
```

### 4. 验证

连接成功后，在对话中发送一张图片，或手动调用：

```
read_media("C:\\path\\to\\image.png")
```

如果一切正常，会返回 `file`（元数据）、`ocr_text`（OCR 文字）、`ai_description`（AI 描述）。

---

## 二、OCR 引擎安装（可选）

### Tesseract（推荐，免费开源）

**Windows：**
```bash
winget install UB-Mannheim.TesseractOCR
# 或从 https://github.com/UB-Mannheim/tesseract/wiki 下载安装包
```

**macOS：**
```bash
brew install tesseract tesseract-lang
```

**Linux：**
```bash
sudo apt install tesseract-ocr tesseract-ocr-chi-sim tesseract-ocr-eng
```

如果需要中文识别，确保 `chi_sim.traineddata` 在 `TESSDATA_PREFIX/tessdata/` 目录下。

### PaddleOCR（准确度更高）

```bash
pip install paddleocr
```

首次运行会自动下载模型文件（约 100MB）。

---

## 三、从源码编译

需要 [Rust 工具链](https://rustup.rs/)（最低版本 1.80）。

```bash
git clone https://github.com/YOUR_USERNAME/media-mcp.git
cd media-mcp
cargo build --release
# 编译产物在 target/release/media-mcp.exe
```

---

## 四、Hook 安装（可选，推荐）

`hooks/guard-multimedia.sh` 是一个 **PreToolUse hook**，可以在你使用 `Read` 工具读取图片时自动拦截，并提示改用 `read_media`。

在 Claude Code 的 settings 中配置：

```json
{
  "hooks": {
    "PreToolUse": "D:\\path\\to\\media-mcp\\hooks\\guard-multimedia.sh"
  }
}
```

**settings.json 位置：**

| 级别 | 路径 |
|------|------|
| 项目级 | `.claude/settings.json`（项目根目录） |
| 用户级 | `%USERPROFILE%\\.claude\\settings.json` |

配置后效果：当 Claude 尝试用 `Read` 打开图片时，会自动提示改用 `read_media`。

---

## 五、配置说明

### vision_api 配置

| 字段 | 说明 | 默认值 |
|------|------|--------|
| `base_url` | 兼容 Anthropic 格式的 API 地址 | — |
| `api_key` | API 密钥 | — |
| `model` | 模型名称 | `mimo-v2.5` |
| `max_tokens` | 描述最大 token 数 | `10240` |
| `timeout_seconds` | 请求超时时间（秒） | `30` |

### ocr 配置

| 字段 | 说明 | 默认值 |
|------|------|--------|
| `default_engine` | 引擎选择：`"auto"` / `"tesseract"` / `"paddle"` | `"auto"` |
| `confidence_threshold` | OCR 置信度阈值（低于此值则触发 Vision fallback） | `60` |
| `engines.tesseract.tesseract_cmd` | Tesseract 路径（`null`=自动查找 PATH/默认路径） | `null` |
| `engines.tesseract.languages` | OCR 语言列表，用 `+` 分隔 | `["chi_sim", "eng"]` |
| `engines.paddle.paddle_cmd` | PaddleOCR 路径（`null`=自动查找 PATH） | `null` |
| `engines.paddle.lang` | PaddleOCR 语言 | `"ch"` |

### blocked_extensions 配置

被 hook 拦截的多媒体文件后缀列表，匹配 hook 脚本中的 `BLOCKED_EXTENSIONS`。

---

## 六、常见问题

### Q: 为什么 Vision API 返回空？

检查 `config.json` 中的 `vision_api.api_key` 和 `base_url` 是否正确。
如果使用的是 mimo 类 API，确认 endpoint 兼容 Anthropic 格式。

### Q: OCR 不工作？

运行 `list_supported_types` 查看 `ocr_available` 和 `engines` 字段：

```
list_supported_types
```

如果 `available` 为 `false`，说明未找到 OCR 引擎。确保 Tesseract 或 PaddleOCR 已安装且在 PATH 中。

### Q: 如何只用 OCR 不用 Vision API？

两种方式：
1. 调用时传入 `vision="skip"`：`read_media("image.png", vision="skip")`
2. 配置不填 `vision_api`，只配 OCR

### Q: 如何只用 Vision API 不用 OCR？

调用时传入 `vision="always"`：`read_media("image.png", vision="always")`

### Q: config.json 在哪？

默认查找顺序：
1. `--config <path>` 参数指定的路径
2. `~/.config/media-mcp/config.json`（全局）
3. `.media-mcp.json`（项目根目录）

### Q: 需要 GPU 吗？

Tesseract 纯 CPU 运行。PaddleOCR 默认使用 CPU（`use_gpu: false`），如需 GPU 需额外安装 CUDA 版 PaddlePaddle。
