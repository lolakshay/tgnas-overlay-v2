use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Config {
    pub overlay: OverlayConfig,
    pub capture: CaptureConfig,
    pub telegram: TelegramConfig,
    pub upload: UploadConfig,
    pub llm: LlmConfig,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct OverlayConfig {
    pub enabled: bool,
    pub topmost: bool,
    pub click_through: bool,
    pub status_visible: bool,
    pub status_text: String,
    pub start_silent: bool,
    pub menu_x: Option<i32>,
    pub menu_y: Option<i32>,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct CaptureConfig {
    pub directory: PathBuf,
    pub format: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct TelegramConfig {
    pub enabled: bool,
    pub mode: DeliveryMode,
}
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DeliveryMode {
    LocalOnly,
    LocalAndTelegram,
    TelegramOnly,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum PromptMode {
    #[default]
    Mcq,
    Code,
    Math,
    TrueFalse,
    Custom,
}

#[allow(dead_code)]
impl PromptMode {
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "code" | "coding" | "dev" => PromptMode::Code,
            "math" | "steps" | "calculation" => PromptMode::Math,
            "tf" | "true_false" | "truefalse" | "fill" => PromptMode::TrueFalse,
            "custom" | "user" | "ask" | "question" => PromptMode::Custom,
            _ => PromptMode::Mcq,
        }
    }

    pub fn to_str(&self) -> &'static str {
        match self {
            PromptMode::Mcq => "mcq",
            PromptMode::Code => "code",
            PromptMode::Math => "math",
            PromptMode::TrueFalse => "tf",
            PromptMode::Custom => "custom",
        }
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            PromptMode::Mcq => "MCQ",
            PromptMode::Code => "Code",
            PromptMode::Math => "Math",
            PromptMode::TrueFalse => "T/F",
            PromptMode::Custom => "Custom",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            PromptMode::Mcq => "MCQ Only",
            PromptMode::Code => "Code Mode",
            PromptMode::Math => "Math & Steps",
            PromptMode::TrueFalse => "True/False & Fill",
            PromptMode::Custom => "Custom Q&A",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            PromptMode::Mcq => "Option <letter>: <Answer> (Minimal text)",
            PromptMode::Code => "Clean runnable indented code snippet",
            PromptMode::Math => "Final answer + 2-line step formula breakdown",
            PromptMode::TrueFalse => "Strictly TRUE/FALSE or exact fill-in word",
            PromptMode::Custom => "User-defined custom question / instruction",
        }
    }

    pub fn system_prompt(&self) -> &'static str {
        match self {
            PromptMode::Mcq => {
                "You are an expert computer science professor and exam-solving engine. Analyze the screenshot with extreme precision.\nRULES:\n1. Detect whether the question is Single-Choice OR Multi-Select (e.g., 'Select one or more', 'Select 2 options', checkboxes).\n2. For Single-Choice: Output strictly 'Option <letter>: <Answer text>'.\n3. For Multi-Select: Output EVERY correct option on its own line: Option <letter>: <Answer text>.\n4. Carefully verify code syntax, 0-based indices, string bounds, and language rules before outputting.\n5. Output directly with no conversational filler."
            }
            PromptMode::Code => {
                "You are an expert software engineer and coding exam assistant. Analyze the problem in the screenshot and output clean, optimal, and correct code (Python/Java/C++/SQL as appropriate). Include only minimal inline comments if necessary. Output the runnable code directly."
            }
            PromptMode::Math => {
                "You are an expert mathematics and physics exam assistant. Analyze the problem in the screenshot. First provide the final answer, followed by a concise 2-3 line step-by-step formula breakdown / key derivation."
            }
            PromptMode::TrueFalse => {
                "You are an expert exam assistant. For True/False questions, output strictly 'TRUE' or 'FALSE'. For fill-in-the-blank questions, output strictly the exact word or phrase that completes the blank. No other text."
            }
            PromptMode::Custom => {
                "You are an expert exam, coding, and problem-solving assistant. Analyze the screenshot and provide an accurate, clear, and direct answer according to the user's specific instruction."
            }
        }
    }

    pub fn next(&self) -> Self {
        match self {
            PromptMode::Mcq => PromptMode::Code,
            PromptMode::Code => PromptMode::Math,
            PromptMode::Math => PromptMode::TrueFalse,
            PromptMode::TrueFalse => PromptMode::Custom,
            PromptMode::Custom => PromptMode::Mcq,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct UploadConfig {
    pub enabled: bool,
    pub url: String,
    pub token: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct LlmConfig {
    pub enabled: bool,
    pub api_key: String,
    pub api_keys: Vec<String>,
    pub model: String,
    pub system_prompt: String,
    pub endpoint: String,
    pub max_tokens: u32,
    pub caption_opacity: u8,
    pub hud_x: Option<i32>,
    pub hud_y: Option<i32>,
    pub hud_w: Option<i32>,
    pub hud_h: Option<i32>,
    pub font_size: u32,
    pub theme: String,
    pub auto_dim: bool,
    pub prompt_mode: PromptMode,
    pub custom_prompt: String,
    pub vpn_enabled: bool,
    pub vpn_relay_url: Option<String>,
    pub proxy_url: Option<String>,
}
impl LlmConfig {
    pub fn resolved_api_keys(&self) -> Vec<String> {
        let mut keys = Vec::new();

        // 1. Check api_keys list in config
        for k in &self.api_keys {
            for sub in k.split([',', '\n', ';']) {
                let trimmed = sub.trim();
                if !trimmed.is_empty() && !keys.contains(&trimmed.to_string()) {
                    keys.push(trimmed.to_string());
                }
            }
        }

        // 2. Check single api_key in config
        let single = self.api_key.trim();
        if !single.is_empty() {
            for sub in single.split([',', '\n', ';']) {
                let trimmed = sub.trim();
                if !trimmed.is_empty() && !keys.contains(&trimmed.to_string()) {
                    keys.push(trimmed.to_string());
                }
            }
        }

        // 3. Check OPENROUTER_API_KEYS env var
        if let Ok(env_keys) = std::env::var("OPENROUTER_API_KEYS") {
            for sub in env_keys.split([',', '\n', ';']) {
                let trimmed = sub.trim();
                if !trimmed.is_empty() && !keys.contains(&trimmed.to_string()) {
                    keys.push(trimmed.to_string());
                }
            }
        }

        // 4. Check OPENROUTER_API_KEY env var
        if let Ok(env_key) = std::env::var("OPENROUTER_API_KEY") {
            let trimmed = env_key.trim();
            if !trimmed.is_empty() && !keys.contains(&trimmed.to_string()) {
                keys.push(trimmed.to_string());
            }
        }

        keys
    }

    #[allow(dead_code)]
    pub fn resolved_api_key(&self) -> String {
        self.resolved_api_keys().into_iter().next().unwrap_or_default()
    }

    pub fn get_effective_system_prompt(&self) -> &str {
        self.prompt_mode.system_prompt()
    }
}
impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            api_key: String::new(),
            api_keys: Vec::new(),
            model: "google/gemini-2.5-flash".into(),
            system_prompt: "You are an expert exam and test assistant. Only give the answer and the option only. No explanations, no markdown intro or outro. Output strictly as: Option <letter>: <Answer>".into(),
            endpoint: "https://openrouter.ai/api/v1/chat/completions".into(),
            max_tokens: 350,
            caption_opacity: 180, // ~70% semi-transparent
            hud_x: None,
            hud_y: None,
            hud_w: None,
            hud_h: None,
            font_size: 22,
            theme: "white".into(),
            auto_dim: true,
            prompt_mode: PromptMode::Mcq,
            custom_prompt: "Analyze the screenshot and solve this problem.".into(),
            vpn_enabled: false,
            vpn_relay_url: None,
            proxy_url: None,
        }
    }
}
impl Default for Config {
    fn default() -> Self {
        Self {
            overlay: OverlayConfig::default(),
            capture: CaptureConfig::default(),
            telegram: TelegramConfig::default(),
            upload: UploadConfig::default(),
            llm: LlmConfig::default(),
        }
    }
}
impl Default for OverlayConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            topmost: true,
            click_through: true,
            status_visible: true,
            status_text: "Active".into(),
            start_silent: false,
            menu_x: None,
            menu_y: None,
        }
    }
}
impl Default for CaptureConfig {
    fn default() -> Self {
        Self {
            directory: PathBuf::from("screenshots"),
            format: "png".into(),
        }
    }
}
impl Default for TelegramConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: DeliveryMode::LocalOnly,
        }
    }
}
impl Default for UploadConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            url: "https://your-vercel-app.vercel.app/api/upload".into(),
            token: "your-device-upload-token".into(),
        }
    }
}
impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let path = PathBuf::from("config.toml");
        if !path.exists() {
            return Ok(Self::default());
        }
        let value = toml::from_str(&fs::read_to_string(path)?)?;
        Ok(value)
    }
    pub fn save(&self) -> anyhow::Result<()> {
        fs::write("config.toml", toml::to_string_pretty(self)?)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_llm_config_defaults() {
        let config = Config::default();
        assert!(config.llm.enabled);
        assert_eq!(config.llm.model, "google/gemini-2.5-flash");
        assert_eq!(config.llm.endpoint, "https://openrouter.ai/api/v1/chat/completions");
        assert!(config.llm.system_prompt.contains("Only give the answer and the option only"));
    }

    #[test]
    fn test_resolved_api_key() {
        let mut llm = LlmConfig::default();
        llm.api_key = "  sk-or-test-key  ".into();
        assert_eq!(llm.resolved_api_key(), "sk-or-test-key");
    }

    #[test]
    fn test_multiple_resolved_api_keys() {
        let mut llm = LlmConfig::default();
        llm.api_keys = vec!["key1".into(), "key2, key3".into()];
        llm.api_key = "key4, key1".into(); // key1 is duplicate
        let keys = llm.resolved_api_keys();
        assert_eq!(keys, vec!["key1", "key2", "key3", "key4"]);
    }

    #[test]
    fn test_prompt_mode_cycling() {
        let mode = PromptMode::Mcq;
        assert_eq!(mode.next(), PromptMode::Code);
        assert_eq!(mode.next().next(), PromptMode::Math);
        assert_eq!(mode.next().next().next(), PromptMode::TrueFalse);
        assert_eq!(mode.next().next().next().next(), PromptMode::Custom);
        assert_eq!(mode.next().next().next().next().next(), PromptMode::Mcq);
    }

    #[test]
    fn test_prompt_mode_system_prompts() {
        assert!(PromptMode::Mcq.system_prompt().contains("Option"));
        assert!(PromptMode::Code.system_prompt().contains("runnable code"));
        assert!(PromptMode::Math.system_prompt().contains("mathematics"));
        assert!(PromptMode::TrueFalse.system_prompt().contains("TRUE"));
        assert!(PromptMode::Custom.system_prompt().contains("expert exam"));
    }

    #[test]
    fn test_prompt_mode_serialization() {
        #[derive(Serialize, Deserialize)]
        struct TestWrapper {
            mode: PromptMode,
        }
        let w = TestWrapper { mode: PromptMode::Code };
        let toml_str = toml::to_string(&w).unwrap();
        assert!(toml_str.contains("mode = \"code\""));
        let parsed: TestWrapper = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.mode, PromptMode::Code);
    }
}

