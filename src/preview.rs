use std::{env, error::Error, fs};
use base64::{engine::general_purpose, Engine as _};

use crate::cache::{get_cache, set_cache};
use axum::{
    extract::Query,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};

pub async fn preview(
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let url = match params.get("url") {
        Some(u) => match decode_base64_url(u) {
            Ok(decoded) => decoded,
            Err(e) => return Err((StatusCode::BAD_REQUEST, format!("Invalid base64 url: {}", e))),
        },
        None => return Err((StatusCode::BAD_REQUEST, "Missing url parameter".to_string())),
    };
    match handle_preview(&url).await {
        Ok(bytes) => {
            let mut headers = HeaderMap::new();
            headers.insert("Content-Type", "application/pdf".parse().unwrap());
            Ok((headers, bytes))
        }
        Err(err) => {
            eprintln!("preview error: {:?}", err);
            Err((StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))
        }
    }
}

fn decode_base64_url(s: &String) -> Result<String, Box<dyn Error>> {
    let enabled = env::var("BASE64_PREVIEW_URL")
        .unwrap_or_else(|_| "false".to_string())
        .to_lowercase();
    match enabled.as_str() {
        "true" | "1" | "yes" => {
            let decoded_bytes = general_purpose::STANDARD.decode(s)?;
            let decoded_str = String::from_utf8(decoded_bytes)?;
            Ok(decoded_str)
        }
        _ => Ok(s.clone()),
    }
}

async fn handle_preview(url: &str) -> Result<Vec<u8>, Box<dyn Error>> {
    // 处理预览逻辑
    // 1. 检查缓存
    if let Some(cached_pdf_path) = get_cache(url).await {
        println!("Cache hit for URL: {}", url);
        return Ok(fs::read(cached_pdf_path)?);
    } else {
        println!("Cache miss for URL: {}", url);

        // 2. 执行转换
        let pdf_path = match crate::convert::convert_file_to_pdf(url).await {
            Ok(u) => u,
            Err(err) => {
                eprintln!("Conversion error for URL {}: {:?}", url, err);
                return Err(err);
            }
        };
        let pdf_bytes = fs::read(&pdf_path)?;

        // 3. 设置缓存
        set_cache(url.to_string(), pdf_path).await;
        Ok(pdf_bytes)
    }
}
