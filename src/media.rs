use teloxide::prelude::*;

/// Detect image format from magic bytes (file signatures)
pub fn detect_image_format(bytes: &[u8]) -> Option<&'static str> {
    if bytes.len() < 4 {
        return None;
    }

    match bytes {
        // JPEG: FF D8 FF
        [0xFF, 0xD8, 0xFF, ..] => Some("image/jpeg"),
        // PNG: 89 50 4E 47
        [0x89, 0x50, 0x4E, 0x47, ..] => Some("image/png"),
        // WebP: RIFF....WEBP
        [0x52, 0x49, 0x46, 0x46, _, _, _, _, 0x57, 0x45, 0x42, 0x50, ..] => Some("image/webp"),
        _ => None,
    }
}

/// Download image from Telegram and convert to base64 data URL
/// Returns (mime_type, data_url) if successful
pub async fn to_data_url(bot: &Bot, file_id: &str) -> Option<(String, String)> {
    let file = bot.get_file(file_id.to_string()).await.ok()?;
    let url = format!(
        "https://api.telegram.org/file/bot{}/{}",
        bot.token(),
        file.path
    );

    // Download the file
    let client = reqwest::Client::new();
    let response = match client.get(&url).send().await {
        Ok(r) => r,
        Err(e) => {
            log::warn!("*download failed* {}: {}", url, e);
            return None;
        }
    };

    let bytes = match response.bytes().await {
        Ok(b) => b,
        Err(e) => {
            log::warn!("*bytes read failed* {}: {}", url, e);
            return None;
        }
    };

    // Detect format from magic bytes
    let mime_type = detect_image_format(&bytes).or_else(|| {
        // Fallback to extension-based detection
        let path = file.path.to_lowercase();
        if path.ends_with(".jpg") || path.ends_with(".jpeg") {
            Some("image/jpeg")
        } else if path.ends_with(".png") {
            Some("image/png")
        } else if path.ends_with(".webp") {
            Some("image/webp")
        } else {
            None
        }
    })?;

    // Encode to base64
    use base64::{engine::general_purpose, Engine as _};
    let base64_data = general_purpose::STANDARD.encode(&bytes);
    let data_url = format!("data:{};base64,{}", mime_type, base64_data);

    log::info!(
        "*data url created* file_id: {}, mime: {}, raw_size: {} bytes, data_url_len: {}",
        file_id,
        mime_type,
        bytes.len(),
        data_url.len()
    );

    Some((mime_type.to_string(), data_url))
}
