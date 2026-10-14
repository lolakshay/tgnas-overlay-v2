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
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning: Option<ReasoningParam>,
}

#[derive(Serialize, Clone)]
struct ReasoningParam {
    effort: String,
    exclude: bool,
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


static CLIENT_CACHE: std::sync::Mutex<Option<(bool, Option<String>, reqwest::blocking::Client)>> =
    std::sync::Mutex::new(None);

fn get_http_client(config: &LlmConfig) -> reqwest::blocking::Client {
    if let Ok(guard) = CLIENT_CACHE.lock() {
        if let Some((vpn, ref proxy, ref client)) = *guard {
            if vpn == config.vpn_enabled && *proxy == config.proxy_url {
                return client.clone();
            }
        }
    }

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

    let client = builder.build().unwrap_or_else(|_| reqwest::blocking::Client::new());
    if let Ok(mut guard) = CLIENT_CACHE.lock() {
        *guard = Some((config.vpn_enabled, config.proxy_url.clone(), client.clone()));
    }
    client
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
            crate::config::PromptMode::Mcq => "Analyze the screenshot. You must be 100% accurate.\n\
CRITICAL RULES:\n\
1. IF THE SCREENSHOT CONTAINS ANY OPTIONS (such as A, B, C, D, radio buttons, or checkboxes), THIS IS AN MCQ!\n\
   DO NOT output code blocks. DO NOT rewrite code. DO NOT explain code. Output strictly the option letter and text.\n\
2. Internally simulate all code/logic execution step-by-step to be 100% sure before answering.\n\
3. Single Multiple-Choice format: Option <letter>: <Answer text>\n\
4. Multiple-Select format (e.g., checkboxes, 'Select two', 'Select one or more'): Output ALL correct options, each on its own line: Option <letter>: <Answer text>\n\
5. Only if there are NO options anywhere on screen:\n\
   - Fill-in-the-Blank: Fill: <exact word, phrase, or code>\n\
   - Order/Match: Order: <sequence>\n\
   - True/False: TRUE or FALSE\n\
Strictly follow these rules. Output only the answer line.".to_string(),
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

    let mut effective_max_tokens = match config.prompt_mode {
        crate::config::PromptMode::Code => config.max_tokens.max(650),
        crate::config::PromptMode::Math => config.max_tokens.max(450),
        crate::config::PromptMode::Custom => config.max_tokens.max(500),
        crate::config::PromptMode::Mcq => config.max_tokens.max(500),
        _ => {
            if custom_question.is_some() {
                config.max_tokens.max(500)
            } else {
                config.max_tokens.max(350)
            }
        }
    };

    // Reasoning models generate internal chain-of-thought tokens that count against max_tokens.
    // Ensure sufficient token budget (at least 1000 tokens) so internal thinking is never truncated.
    let reasoning = if is_reasoning_model(&config.model) {
        effective_max_tokens = effective_max_tokens.max(1000);
        Some(ReasoningParam {
            effort: "high".to_string(),
            exclude: true,
        })
    } else {
        None
    };

    let request_payload = ChatCompletionRequest {
        model: config.model.clone(),
        messages: vec![system_message, user_message],
        max_tokens: effective_max_tokens,
        reasoning,
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

fn is_reasoning_model(model: &str) -> bool {
    let lower = model.to_lowercase();
    lower.contains("gemini-2.5")
        || lower.contains("gemini-3")
        || lower.contains("gemini-4")
        || lower.contains("claude-3.7")
        || lower.contains("claude-4")
        || lower.contains("claude-5")
        || lower.contains("o3")
        || lower.contains("o1")
        || lower.contains("r1")
        || lower.contains("deepseek-r1")
        || lower.contains("thinking")
}

pub fn clean_llm_response(raw: &str, mode: crate::config::PromptMode) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    if mode == crate::config::PromptMode::Mcq {
        let mut option_lines = Vec::new();
        for line in trimmed.lines() {
            let l = line.trim();
            if l.is_empty() {
                continue;
            }
            let lower = l.to_lowercase();
            // Match Option <X>, X), (X), Fill:, Order:, TRUE, FALSE
            let is_opt_word = lower.starts_with("option ")
                || lower.starts_with("opt ");
            let is_letter_paren = l.len() >= 2
                && l.chars().next().map_or(false, |c| c.is_ascii_alphabetic())
                && (l.starts_with("a)") || l.starts_with("b)") || l.starts_with("c)") || l.starts_with("d)") || l.starts_with("e)")
                    || l.starts_with("A)") || l.starts_with("B)") || l.starts_with("C)") || l.starts_with("D)") || l.starts_with("E)")
                    || l.starts_with("a.") || l.starts_with("b.") || l.starts_with("c.") || l.starts_with("d.") || l.starts_with("e.")
                    || l.starts_with("A.") || l.starts_with("B.") || l.starts_with("C.") || l.starts_with("D.") || l.starts_with("E."));
            let is_special = lower.starts_with("fill:") || lower.starts_with("order:") || lower == "true" || lower == "false";

            if is_opt_word || is_letter_paren || is_special {
                option_lines.push(l);
            }
        }

        // If explicit option lines were found, return strictly those lines and drop any accidental code or preamble!
        if !option_lines.is_empty() {
            return option_lines.join("\n");
        }

        // If the model wrapped the output in markdown code fences without option prefix, strip fences
        if trimmed.starts_with("```") && trimmed.ends_with("```") {
            let lines: Vec<&str> = trimmed.lines().collect();
            if lines.len() >= 2 {
                let inner = lines[1..lines.len() - 1].join("\n");
                let clean_inner = inner.trim();
                if !clean_inner.is_empty() {
                    return clean_inner.to_string();
                }
            }
        }
    }

    trimmed.to_string()
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
                let cleaned = clean_llm_response(&content, config.prompt_mode);
                if !cleaned.is_empty() {
                    tracing::info!(answer = %cleaned, "LLM successfully produced answer");
                    return Ok(cleaned);
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
            reasoning: Some(ReasoningParam {
                effort: "high".into(),
                exclude: true,
            }),
        };
        let serialized = serde_json::to_string(&req).unwrap();
        assert!(serialized.contains("google/gemini-2.5-flash"));
        assert!(serialized.contains("\"max_tokens\":150"));
        assert!(serialized.contains("\"reasoning\":{\"effort\":\"high\",\"exclude\":true}"));
    }

    #[test]
    fn test_custom_mode_handling() {
        let mut cfg = LlmConfig::default();
        cfg.prompt_mode = crate::config::PromptMode::Custom;
        cfg.custom_prompt = "Find time complexity of function".into();
        assert_eq!(cfg.prompt_mode.short_name(), "Custom");
        assert!(cfg.prompt_mode.system_prompt().contains("expert"));
    }

    #[test]
    fn test_clean_llm_response_extracts_option_from_code() {
        let dirty = "```java\nSystem.out.println(\"1 1 1 3 1 3\");\n```\nOption d: 1 1 1 3 1 3";
        let cleaned = clean_llm_response(dirty, crate::config::PromptMode::Mcq);
        assert_eq!(cleaned, "Option d: 1 1 1 3 1 3");
    }

    #[test]
    fn test_clean_llm_response_multi_select() {
        let dirty = "Here is the explanation:\nOption a: First answer\nOption c: Third answer\nHope this helps!";
        let cleaned = clean_llm_response(dirty, crate::config::PromptMode::Mcq);
        assert_eq!(cleaned, "Option a: First answer\nOption c: Third answer");
    }

    #[test]
    fn test_is_reasoning_model() {
        assert!(is_reasoning_model("google/gemini-3.8-flash"));
        assert!(is_reasoning_model("google/gemini-3.7-flash"));
        assert!(is_reasoning_model("google/gemini-4-argon"));
        assert!(is_reasoning_model("google/gemini-2.5-pro"));
        assert!(is_reasoning_model("anthropic/claude-3.7-sonnet"));
        assert!(is_reasoning_model("openai/o3-mini"));
        assert!(!is_reasoning_model("openai/gpt-4o"));
    }
}

