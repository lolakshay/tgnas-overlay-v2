use anyhow::{bail, Context, Result};
use base64::prelude::*;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use crate::config::LlmConfig;

#[derive(Serialize)]
struct ChatCompletionRequest {
    model: String,
    messages: Vec<ChatMessage>,
    max_tokens: u32,
}

#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: MessageContent,
}

#[derive(Serialize)]
#[serde(untagged)]
enum MessageContent {
    Text(String),
    Parts(Vec<ContentPart>),
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ContentPart {
    Text { text: String },
    ImageUrl { image_url: ImageUrlObject },
}

#[derive(Serialize)]
struct ImageUrlObject {
    url: String,
}

#[derive(Deserialize)]
struct ChatCompletionResponse {
    choices: Option<Vec<Choice>>,
    error: Option<ApiError>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: Option<String>,
}

#[derive(Deserialize)]
struct ApiError {
    message: Option<String>,
}


fn get_http_client(config: &LlmConfig) -> reqwest::blocking::Client {
    let mut builder = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(30))
        .pool_idle_timeout(Duration::from_secs(120))
        .tcp_nodelay(true);

    if config.vpn_enabled {
        // 1. If custom proxy configured (SOCKS5/HTTP), route through it
        if let Some(ref proxy_url) = config.proxy_url {
            let trimmed = proxy_url.trim();
            if !trimmed.is_empty() {
                if let Ok(proxy) = reqwest::Proxy::all(trimmed) {
                    tracing::info!(proxy = %trimmed, "Routing LLM requests through stealth proxy");
                    builder = builder.proxy(proxy);
                } else {
                    tracing::warn!(proxy = %trimmed, "Failed to parse proxy URL");
                }
            }
        }

        // 2. Direct-to-Edge Encrypted Anycast Resolution:
        // Cloudflare Anycast IP addresses for openrouter.ai
        // Bypasses campus DNS blocks and DNS poisoning completely!
        if let (Ok(ip1), Ok(ip2), Ok(ip3)) = (
            "104.26.10.150:443".parse(),
            "104.26.11.150:443".parse(),
            "172.67.69.117:443".parse(),
        ) {
            builder = builder
                .resolve("openrouter.ai", ip1)
                .resolve("openrouter.ai", ip2)
                .resolve("openrouter.ai", ip3);
        }
    }

    builder.build().unwrap_or_else(|_| reqwest::blocking::Client::new())
}

/// Sends screenshot (JPEG/PNG) to OpenRouter and asks the LLM to output ONLY the option and the answer.
/// Automatically executes a fallback strategy across all configured API keys if one fails.
pub fn query_openrouter(
    config: &LlmConfig,
    image_bytes: &[u8],
    mime_type: &str,
    custom_question: Option<&str>,
) -> Result<String> {
    if !config.enabled {
        bail!("LLM processing is disabled in configuration");
    }
    let keys = config.resolved_api_keys();
    if keys.is_empty() {
        bail!("No OpenRouter API keys provided. Please set OPENROUTER_API_KEY in .env or config.toml");
    }

    let b64_img = BASE64_STANDARD.encode(image_bytes);
    let data_url = format!("data:{};base64,{}", mime_type, b64_img);

    let system_message = ChatMessage {
        role: "system".into(),
        content: MessageContent::Text(config.get_effective_system_prompt().to_string()),
    };

    let user_instruction = if let Some(q) = custom_question {
        let trimmed = q.trim();
        if !trimmed.is_empty() {
            format!("Analyze the screenshot and answer this specific question: {}", trimmed)
        } else {
            "Analyze the screenshot and provide an accurate, clear answer.".to_string()
        }
    } else {
        match config.prompt_mode {
            crate::config::PromptMode::Mcq => "Analyze the screenshot. Identify the question and options. If the question asks for multiple options (e.g., 'Select one or more', 'Select 2 options', checkboxes), output ALL correct options, each on its own line formatted as 'Option <letter>: <Answer text>'. Otherwise output strictly the single correct 'Option <letter>: <Answer text>'. No filler text.".to_string(),
            crate::config::PromptMode::Code => "Analyze the screenshot. Provide clean, correct, runnable code that solves the problem. No unnecessary commentary.".to_string(),
            crate::config::PromptMode::Math => "Analyze the screenshot. Provide the final numerical or symbolic answer first, followed by a concise 2-3 line step breakdown.".to_string(),
            crate::config::PromptMode::TrueFalse => "Analyze the screenshot. For True/False output strictly TRUE or FALSE. For fill-in-the-blank output strictly the exact word or phrase.".to_string(),
            crate::config::PromptMode::Custom => {
                let trimmed = config.custom_prompt.trim();
                if !trimmed.is_empty() {
                    format!("Analyze the screenshot and answer this specific question: {}", trimmed)
                } else {
                    "Analyze the screenshot and provide an accurate, clear answer.".to_string()
                }
            }
        }
    };

    let user_message = ChatMessage {
        role: "user".into(),
        content: MessageContent::Parts(vec![
            ContentPart::Text {
                text: user_instruction,
            },
            ContentPart::ImageUrl {
                image_url: ImageUrlObject { url: data_url },
            },
        ]),
    };

    let effective_max_tokens = match config.prompt_mode {
        crate::config::PromptMode::Code => config.max_tokens.max(600),
        crate::config::PromptMode::Math => config.max_tokens.max(400),
        crate::config::PromptMode::Custom => config.max_tokens.max(500),
        crate::config::PromptMode::Mcq => config.max_tokens.max(350),
        _ => {
            if custom_question.is_some() {
                config.max_tokens.max(450)
            } else {
                config.max_tokens.max(300)
            }
        }
    };

    let request_payload = ChatCompletionRequest {
        model: config.model.clone(),
        messages: vec![system_message, user_message],
        max_tokens: effective_max_tokens,
    };

    let body_json = serde_json::to_string(&request_payload)
        .context("Failed to serialize LLM request payload")?;

    let client = get_http_client(config);

    let total_keys = keys.len();
    let mut last_error = String::from("unknown error");

    for (idx, key) in keys.iter().enumerate() {
        let key_num = idx + 1;
        let masked = mask_key(key);
        tracing::info!(
            key_index = key_num,
            total_keys = total_keys,
            key = %masked,
            model = %config.model,
            vpn_active = config.vpn_enabled,
            "Querying OpenRouter LLM..."
        );

        match send_single_request(&client, config, key, &body_json) {
            Ok(answer) => {
                if idx > 0 {
                    tracing::info!(
                        key_index = key_num,
                        "Fallback API key succeeded!"
                    );
                }
                return Ok(answer);
            }
            Err(e) => {
                tracing::warn!(
                    key_index = key_num,
                    total_keys = total_keys,
                    key = %masked,
                    error = %e,
                    "API key failed. Attempting fallback..."
                );
                last_error = e.to_string();
            }
        }
    }

    bail!(
        "All {} OpenRouter API key(s) failed. Last error: {}",
        total_keys,
        last_error
    )
}

fn mask_key(key: &str) -> String {
    if key.len() > 12 {
        format!("{}...{}", &key[..6], &key[key.len() - 4..])
    } else {
        "***".to_string()
    }
}

fn send_single_request(
    client: &reqwest::blocking::Client,
    config: &LlmConfig,
    api_key: &str,
    body_json: &str,
) -> Result<String> {
    let target_endpoint = if config.vpn_enabled {
        if let Some(ref relay) = config.vpn_relay_url {
            let trimmed = relay.trim();
            if !trimmed.is_empty() {
                trimmed
            } else {
                &config.endpoint
            }
        } else {
            &config.endpoint
        }
    } else {
        &config.endpoint
    };

    let response = client
        .post(target_endpoint)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .header("HTTP-Referer", "https://localhost")
        .header("X-Title", "Overlay Host")
        .body(body_json.to_string())
        .send()
        .context("Failed to connect to OpenRouter API endpoint")?;

    let status = response.status();
    let body_text = response.text().context("Failed to read OpenRouter response")?;

    if !status.is_success() {
        tracing::error!(status = %status, body = %body_text, "OpenRouter API request failed");
        bail!("OpenRouter returned HTTP {}: {}", status, body_text);
    }

    let completion: ChatCompletionResponse = serde_json::from_str(&body_text)
        .with_context(|| format!("Failed to parse OpenRouter response: {}", body_text))?;

    if let Some(err) = completion.error {
        let msg = err.message.unwrap_or_else(|| "Unknown API error".into());
        bail!("OpenRouter error: {}", msg);
    }

    if let Some(choices) = completion.choices {
        if let Some(first) = choices.into_iter().next() {
            if let Some(content) = first.message.content {
                let trimmed = content.trim();
                if !trimmed.is_empty() {
                    tracing::info!(answer = %trimmed, "LLM successfully produced answer");
                    return Ok(trimmed.to_string());
                }
            }
        }
    }

    bail!("OpenRouter response contained no message content")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_openrouter_success_response() {
        let sample = r#"{
            "id": "gen-1234",
            "choices": [
                {
                    "message": {
                        "role": "assistant",
                        "content": "Option B: Direct Memory Access (DMA)"
                    }
                }
            ]
        }"#;

        let completion: ChatCompletionResponse = serde_json::from_str(sample).unwrap();
        let content = completion.choices.unwrap().remove(0).message.content.unwrap();
        assert_eq!(content, "Option B: Direct Memory Access (DMA)");
    }

    #[test]
    fn test_request_payload_serialization() {
        let req = ChatCompletionRequest {
            model: "google/gemini-2.5-flash".into(),
            messages: vec![ChatMessage {
                role: "user".into(),
                content: MessageContent::Text("Hello".into()),
            }],
            max_tokens: 150,
        };
        let serialized = serde_json::to_string(&req).unwrap();
        assert!(serialized.contains("google/gemini-2.5-flash"));
        assert!(serialized.contains("\"max_tokens\":150"));
    }

    #[test]
    fn test_custom_mode_handling() {
        let mut cfg = LlmConfig::default();
        cfg.prompt_mode = crate::config::PromptMode::Custom;
        cfg.custom_prompt = "Find time complexity of function".into();
        assert_eq!(cfg.prompt_mode.short_name(), "Custom");
        assert!(cfg.prompt_mode.system_prompt().contains("expert exam"));
    }
}

