# Media MCP — 多媒体文件读取 MCP Server 设计文档

## 问题陈述

使用不支持多模态的模型（如 `mimo-v2.5-pro[1m]`）时，Claude Code 内置的 `Read` 工具会尝试将图片以 base64 传给模型，导致 API 报错（"There's an issue with the selected model"）且中断整个对话，无法恢复。

## 目标

1. **防御层**：通过 PreToolUse Hook 拦截对多媒体文件的 Read 调用，阻止错误发生
2. **替代层**：提供 MCP 工具 `read_media`，将多媒体文件转换为纯文本信息（元数据 + OCR + AI 描述）返回给模型

## 架构

```
┌─────────────────┐     PreToolUse Hook      ┌──────────────────┐
│  Claude Code    │ ───────────────────────►  │  guard.sh        │
│  Read("x.png")  │                           │  检查扩展名       │
│                 │ ◄─── 拒绝 + 提示信息 ───── │  匹配→返回block   │
└─────────────────┘                           └──────────────────┘
                                                       │
                                                       ▼ 用户/m模型重试
┌─────────────────┐     stdio (JSON-RPC)     ┌──────────────────┐
│  Claude Code    │ ───────────────────────►  │  media-mcp       │
│  read_media()   │                           │  (Rust binary)   │
│                 │ ◄── 元数据+OCR+AI描述 ─── │                  │
└─────────────────┘                           └──────────────────┘
```

## 技术选型

| 组件 | 选型 | 理由 |
|------|------|------|
| MCP Server 语言 | Rust | 内存占用极小（~2-5MB），单二进制分发 |
| MCP SDK | `rmcp` | Rust 官方 MCP SDK，支持 stdio transport |
| Hook 脚本 | Bash | 零依赖，Claude Code 原生支持 command hook |
| OCR 引擎 | Tesseract（via `tesseract` crate） | 开源、支持中英文 |
| Vision API | 可配置，Anthropic 兼容格式 | 灵活，用户可指向任意兼容端点 |
| 图片元数据 | `image` crate | 纯 Rust，无外部依赖 |

## 项目结构

```
D:/workspace/RustProjects/media-mcp/
├── Cargo.toml
├── config.json.example        # 配置示例
├── src/
│   ├── main.rs                # 入口，MCP server stdio 启动
│   ├── config.rs              # 配置加载与验证
│   ├── tools/
│   │   ├── mod.rs
│   │   ├── metadata.rs        # 图片/文件元数据提取
│   │   ├── ocr.rs             # Tesseract OCR 封装
│   │   └── vision.rs          # AI 视觉 API 调用
│   └── handlers.rs            # MCP tool handler 注册
└── hooks/
    └── guard-multimedia.sh    # PreToolUse hook 脚本
```

## 配置格式

### config.json

```json
{
  "vision_api": {
    "base_url": "https://token-plan-cn.xiaomimimo.com/anthropic",
    "api_key": "tp-xxx",
    "model": "mimo-v2-omni",
    "max_tokens": 1024,
    "timeout_seconds": 30
  },
  "ocr": {
    "languages": ["chi_sim", "eng"],
    "tesseract_cmd": null
  },
  "blocked_extensions": [
    ".png", ".jpg", ".jpeg", ".gif", ".bmp", ".webp", ".svg", ".ico", ".tiff",
    ".mp4", ".avi", ".mkv", ".mov", ".webm",
    ".mp3", ".wav", ".flac", ".aac", ".ogg",
    ".pdf"
  ]
}
```

配置文件查找顺序：
1. 命令行 `--config` 参数指定路径
2. `~/.config/media-mcp/config.json`（全局）
3. 当前目录 `.media-mcp.json`（项目级）

## MCP 工具定义

### `read_media`

**参数：**
- `file_path: string`（必填）— 文件绝对路径
- `language: string`（可选）— OCR 语言提示，默认 "chi_sim+eng"

**返回 JSON：**
```json
{
  "file": {
    "name": "screenshot.png",
    "path": "/absolute/path/screenshot.png",
    "size_bytes": 245760,
    "mime_type": "image/png",
    "format": "PNG",
    "dimensions": "1920x1080"
  },
  "ocr_text": "从图片中提取的文字内容...",
  "ai_description": "这是一张系统设置截图，显示了网络配置页面...",
  "warnings": []
}
```

### `list_supported_types`

无参数，返回当前支持的文件类型列表和各处理器状态（OCR 可用性、API 可用性）。

## 处理流程

### 图片文件（MVP 完整支持）

1. `image` crate 读取元数据（格式、尺寸、颜色空间）
2. Tesseract OCR 提取文字
3. 如果图片 > 10MB，自动缩放至合理尺寸
4. base64 编码，调用 Vision API 生成中文描述
5. 汇总返回

### PDF 文件（v0.2）

1. 元数据提取（页数、大小）
2. 优先使用 `pdf-extract` 提取文本层
3. 如果无文本层（扫描件），逐页 OCR

### 视频文件（v0.2）

1. 元数据（时长、分辨率、编码格式）
2. 通过 ffprobe 提取技术信息
3. 抽取关键帧，调用 Vision API 描述

### 音频文件（v0.2）

1. 元数据（时长、采样率、编码格式）
2. 后续可集成 whisper 实现转录

## Hook 脚本

`hooks/guard-multimedia.sh`：

- 从 stdin 读取 Claude Code 传入的 JSON
- 检查 `tool_name` 是否为 `Read`
- 提取 `tool_input.file_path`，比对扩展名
- 匹配则输出 `{"decision":"block","reason":"..."}` 阻止调用
- 不匹配则静默退出（放行）

## 错误处理

| 场景 | 处理方式 |
|------|----------|
| 文件不存在 | 返回明确错误信息 |
| OCR 引擎未安装 | 跳过 OCR，在 warnings 中提示安装 tesseract |
| Vision API 不可用 | 跳过 AI 描述，在 warnings 中提示检查配置 |
| 图片过大(>10MB) | 自动缩放后发送，保留元数据 |
| 不支持的格式 | 返回元数据 + warnings |
| API 超时 | 重试一次，失败则跳过 |

所有错误均为降级处理，不会 panic 或中断 MCP 连接。

## Claude Code 配置集成

### settings.json hook 配置

```json
{
  "hooks": {
    "PreToolUse": [
      {
        "matcher": "Read",
        "hooks": [
          {
            "type": "command",
            "command": "bash /path/to/media-mcp/hooks/guard-multimedia.sh"
          }
        ]
      }
    ]
  }
}
```

### settings.json MCP server 配置

```json
{
  "mcpServers": {
    "media-mcp": {
      "command": "/path/to/media-mcp/target/release/media-mcp",
      "args": ["--config", "/path/to/media-mcp/config.json"]
    }
  }
}
```

## 依赖 Crate

| crate | 版本 | 用途 |
|-------|------|------|
| `rmcp` | latest | MCP 协议（stdio transport） |
| `image` | 0.25 | 图片元数据、尺寸读取 |
| `tesseract` | 0.14 | OCR 引擎绑定 |
| `reqwest` | 0.12 | HTTP 客户端 |
| `serde` / `serde_json` | 1.x | JSON 序列化 |
| `base64` | 0.22 | 图片 base64 编码 |
| `tokio` | 1.x | 异步运行时 |
| `mime_guess` | 2.x | MIME 类型推断 |
| `clap` | 4.x | 命令行参数解析 |

## MVP 范围（v0.1）

- ✅ 图片文件完整支持（元数据 + OCR + AI 描述）
- ✅ Hook 拦截脚本
- ✅ 配置文件加载
- ✅ 错误降级处理
- ✅ `list_supported_types` 工具

## 后续迭代（v0.2）

- PDF 文本提取与 OCR
- 视频 ffprobe 信息 + 抽帧
- 音频元数据增强
- 缓存机制（避免重复处理同一文件）
