use anyhow::{Context, Result};

/// Sends PNG bytes directly from memory; no intermediate disk file is required.
pub fn send_photo(png: Vec<u8>) -> Result<()> {
    let token = std::env::var("TELEGRAM_BOT_TOKEN")
        .context("TELEGRAM_BOT_TOKEN is required for Telegram delivery")?;
    let chat_id = std::env::var("TELEGRAM_CHAT_ID")
        .context("TELEGRAM_CHAT_ID is required for Telegram delivery")?;
    let client = reqwest::blocking::Client::new();
    let form = reqwest::blocking::multipart::Form::new()
        .text("chat_id", chat_id)
        .part(
            "photo",
            reqwest::blocking::multipart::Part::bytes(png)
                .file_name("screenshot.png")
                .mime_str("image/png")?,
        );
    let response = client
        .post(format!("https://api.telegram.org/bot{token}/sendPhoto"))
        .multipart(form)
        .send()
        .context("calling Telegram sendPhoto")?;
    if !response.status().is_success() {
        anyhow::bail!("Telegram sendPhoto returned {}", response.status());
    }
    tracing::info!("Telegram upload succeeded");
    Ok(())
}
