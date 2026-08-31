use std::io::Cursor;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use png::{Decoder, Encoder};
use thiserror::Error;
use crate::character::{CharacterCardV1, CharacterCardV2};

#[derive(Error, Debug)]
pub enum CardError {
    #[error("PNG error: {0}")]
    PngError(#[from] png::DecodingError),
    #[error("PNG encoding error: {0}")]
    PngEncodingError(#[from] png::EncodingError),
    #[error("Missing character metadata chunk")]
    MissingChunk,
    #[error("Base64 decoding failed: {0}")]
    Base64Error(#[from] base64::DecodeError),
    #[error("JSON parsing error: {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("Invalid UTF-8 sequence: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),
    #[error("Invalid or unrecognized card format")]
    InvalidFormat,
}

/// Parses character data from PNG bytes or JSON bytes.
/// Returns the parsed V2 card and an optional base64 avatar data URL.
pub fn parse_character_card(bytes: &[u8]) -> Result<(CharacterCardV2, Option<String>), CardError> {
    // 1. Try parsing as PNG with embedded metadata
    if is_png(bytes) {
        if let Ok(result) = parse_png_card(bytes) {
            return Ok(result);
        }
    }

    // 2. Try parsing as raw JSON
    if let Ok(json_str) = std::str::from_utf8(bytes) {
        if let Ok(card_v2) = serde_json::from_str::<CharacterCardV2>(json_str) {
            return Ok((card_v2, None));
        }
        if let Ok(card_v1) = serde_json::from_str::<CharacterCardV1>(json_str) {
            return Ok((card_v1.into(), None));
        }
        // Also check if JSON has "data" wrapper without spec
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(json_str) {
            if let Some(data_val) = val.get("data") {
                if let Ok(card_data) = serde_json::from_value(data_val.clone()) {
                    let card = CharacterCardV2 {
                        spec: "chara_card_v2".to_string(),
                        spec_version: "2.0".to_string(),
                        data: card_data,
                    };
                    return Ok((card, None));
                }
            }
        }
    }

    Err(CardError::InvalidFormat)
}

fn is_png(bytes: &[u8]) -> bool {
    bytes.len() >= 8 && &bytes[0..8] == b"\x89PNG\r\n\x1a\n"
}

fn parse_png_card(bytes: &[u8]) -> Result<(CharacterCardV2, Option<String>), CardError> {
    let cursor = Cursor::new(bytes);
    let decoder = Decoder::new(cursor);
    let reader = decoder.read_info()?;
    let info = reader.info();

    // Look for uncompressed latin1 text chunks with keywords "chara" or "ccv3"
    let mut found_text: Option<String> = None;

    for chunk in &info.uncompressed_latin1_text {
        if chunk.keyword.eq_ignore_ascii_case("chara") || chunk.keyword.eq_ignore_ascii_case("ccv3") {
            found_text = Some(chunk.text.clone());
            break;
        }
    }

    if found_text.is_none() {
        for chunk in &info.compressed_latin1_text {
            if chunk.keyword.eq_ignore_ascii_case("chara") || chunk.keyword.eq_ignore_ascii_case("ccv3") {
                if let Ok(text) = chunk.get_text() {
                    found_text = Some(text);
                    break;
                }
            }
        }
    }

    if found_text.is_none() {
        for chunk in &info.utf8_text {
            if chunk.keyword.eq_ignore_ascii_case("chara") || chunk.keyword.eq_ignore_ascii_case("ccv3") {
                if let Ok(text) = chunk.get_text() {
                    found_text = Some(text);
                    break;
                }
            }
        }
    }

    let raw_encoded = found_text.ok_or(CardError::MissingChunk)?;
    let decoded_bytes = BASE64.decode(raw_encoded.trim().as_bytes())?;
    let json_str = String::from_utf8(decoded_bytes)?;

    let card: CharacterCardV2 = if let Ok(v2) = serde_json::from_str::<CharacterCardV2>(&json_str) {
        v2
    } else if let Ok(v1) = serde_json::from_str::<CharacterCardV1>(&json_str) {
        v1.into()
    } else {
        serde_json::from_str::<CharacterCardV2>(&json_str)?
    };

    // Format avatar data url from PNG bytes
    let avatar_data_url = format!("data:image/png;base64,{}", BASE64.encode(bytes));

    Ok((card, Some(avatar_data_url)))
}

/// Exports a CharacterCardV2 into a PNG with embedded 'chara' metadata chunk.
/// If `avatar_png_bytes` is provided, the card data is written into that PNG.
/// Otherwise, a minimal 1x1 or colored PNG is created.
pub fn export_character_png(
    card: &CharacterCardV2,
    avatar_png_bytes: Option<&[u8]>,
) -> Result<Vec<u8>, CardError> {
    let json_bytes = serde_json::to_vec(card)?;
    let b64_text = BASE64.encode(json_bytes);

    let (width, height, image_data, color_type) = if let Some(bytes) = avatar_png_bytes {
        if is_png(bytes) {
            let decoder = Decoder::new(Cursor::new(bytes));
            let mut reader = decoder.read_info()?;
            let mut buf = vec![0; reader.output_buffer_size()];
            let output_info = reader.next_frame(&mut buf)?;
            buf.truncate(output_info.buffer_size());
            (output_info.width, output_info.height, buf, output_info.color_type)
        } else {
            create_default_pixel_data()
        }
    } else {
        create_default_pixel_data()
    };

    let mut output_bytes = Vec::new();
    {
        let mut encoder = Encoder::new(&mut output_bytes, width, height);
        encoder.set_color(color_type);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.add_text_chunk("chara".to_string(), b64_text).map_err(CardError::PngEncodingError)?;

        let mut writer = encoder.write_header().map_err(CardError::PngEncodingError)?;
        writer.write_image_data(&image_data).map_err(CardError::PngEncodingError)?;
    }

    Ok(output_bytes)
}

fn create_default_pixel_data() -> (u32, u32, Vec<u8>, png::ColorType) {
    // 64x64 simple dark purple avatar
    let width = 64;
    let height = 64;
    let mut data = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        for x in 0..width {
            let r = 40 + ((x * 20) / width) as u8;
            let g = 30 + ((y * 20) / height) as u8;
            let b = 70;
            let a = 255;
            data.extend_from_slice(&[r, g, b, a]);
        }
    }
    (width, height, data, png::ColorType::Rgba)
}

pub fn export_character_json(card: &CharacterCardV2) -> Result<String, CardError> {
    serde_json::to_string_pretty(card).map_err(CardError::JsonError)
}
