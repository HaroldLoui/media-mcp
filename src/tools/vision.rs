use crate::config::VisionApiConfig;
use base64::Engine;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    messages: Vec<Message>,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: Vec<ContentBlock>,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum ContentBlock {
    #[serde(rename = "image")]
    Image { source: ImageSource },
    #[serde(rename = "text")]
    Text { text: String },
}

#[derive(Debug, Serialize)]
struct ImageSource {
    #[serde(rename = "type")]
    source_type: String,
    media_type: String,
    data: String,
}

#[derive(Debug, Deserialize)]
struct AnthropicResponse {
    content: Vec<ResponseContent>,
    stop_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ResponseContent {
    #[allow(dead_code)]
    #[serde(rename = "type")]
    content_type: String,
    text: Option<String>,
}

pub struct VisionResult {
    pub description: Option<String>,
    pub warning: Option<String>,
}

/// Call vision API to describe an image. Returns description or warning.
pub async fn describe_image(
    client: &reqwest::Client,
    file_path: &str,
    mime_type: &str,
    config: &VisionApiConfig,
) -> VisionResult {
    // Read and encode image
    let image_data = match std::fs::read(file_path) {
        Ok(data) => data,
        Err(e) => {
            return VisionResult {
                description: None,
                warning: Some(format!("Failed to read image file: {}", e)),
            };
        }
    };

    // Resize if too large (>10MB)
    let data = if image_data.len() > 10 * 1024 * 1024 {
        match resize_image(&image_data) {
            Ok(resized) => resized,
            Err(e) => {
                return VisionResult {
                    description: None,
                    warning: Some(format!("Failed to resize large image: {}", e)),
                };
            }
        }
    } else {
        image_data
    };

    let b64 = base64::engine::general_purpose::STANDARD.encode(&data);

    let request = AnthropicRequest {
        model: config.model.clone(),
        max_tokens: config.max_tokens,
        messages: vec![Message {
            role: "user".to_string(),
            content: vec![
                ContentBlock::Image {
                    source: ImageSource {
                        source_type: "base64".to_string(),
                        media_type: mime_type.to_string(),
                        data: b64,
                    },
                },
                ContentBlock::Text {
                    text: config.prompt.clone(),
                },
            ],
        }],
    };

    let url = format!("{}/v1/messages", config.base_url.trim_end_matches('/'));

    // Retry once on failure
    for attempt in 0..2 {
        match client
            .post(&url)
            .header("x-api-key", &config.api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request)
            .send()
            .await
        {
            Ok(resp) => {
                if !resp.status().is_success() {
                    let status = resp.status();
                    let body = resp.text().await.unwrap_or_default();
                    if attempt == 0 {
                        tracing::warn!("Vision API returned {}, retrying...", status);
                        continue;
                    }
                    return VisionResult {
                        description: None,
                        warning: Some(format!("Vision API error {}: {}", status, body)),
                    };
                }
                match resp.json::<AnthropicResponse>().await {
                    Ok(api_resp) => {
                        let text: String = api_resp
                            .content
                            .iter()
                            .filter_map(|c| c.text.as_deref())
                            .collect::<Vec<_>>()
                            .join("");
                        if text.is_empty() {
                            return VisionResult {
                                description: None,
                                warning: Some("Vision API returned empty response".to_string()),
                            };
                        }
                        let mut warning = None;
                        if api_resp.stop_reason.as_deref() == Some("max_tokens") {
                            warning = Some("AI description was truncated (hit max_tokens limit). Increase max_tokens in config for longer descriptions.".to_string());
                        }
                        return VisionResult {
                            description: Some(text),
                            warning,
                        };
                    }
                    Err(e) => {
                        if attempt == 0 {
                            tracing::warn!(
                                "Failed to parse vision API response: {}, retrying...",
                                e
                            );
                            continue;
                        }
                        return VisionResult {
                            description: None,
                            warning: Some(format!("Failed to parse vision API response: {}", e)),
                        };
                    }
                }
            }
            Err(e) => {
                if attempt == 0 {
                    tracing::warn!("Vision API request failed: {}, retrying...", e);
                    continue;
                }
                return VisionResult {
                    description: None,
                    warning: Some(format!("Vision API request failed: {}", e)),
                };
            }
        }
    }

    VisionResult {
        description: None,
        warning: Some("Vision API failed after retry".to_string()),
    }
}

fn resize_image(data: &[u8]) -> anyhow::Result<Vec<u8>> {
    let format = image::guess_format(data).unwrap_or(image::ImageFormat::Png);
    let img = image::load_from_memory(data)?;
    // Scale down to max 2048px on longest side
    let longest = img.width().max(img.height());
    if longest <= 2048 {
        return Ok(data.to_vec());
    }
    let scale = 2048.0 / longest as f32;
    let new_w = (img.width() as f32 * scale) as u32;
    let new_h = (img.height() as f32 * scale) as u32;
    let resized = img.resize_exact(new_w, new_h, image::imageops::FilterType::Lanczos3);
    let mut buf = std::io::Cursor::new(Vec::new());
    resized.write_to(&mut buf, format)?;
    Ok(buf.into_inner())
}
