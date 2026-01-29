use crate::url_helper::is_remote_url;
use percent_encoding::percent_decode_str;
use std::{env, fmt::Debug, io::Write, process::Command};
use tempfile::NamedTempFile;

#[derive(Debug)]
struct UnsupportedFileTypeError {
    cache_path: String,
}

impl std::fmt::Display for UnsupportedFileTypeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "unsupported file type")
    }
}

impl std::error::Error for UnsupportedFileTypeError {}

pub fn get_output_dir() -> String {
    let output_dir =
        env::var("OUTPUT_DIR").unwrap_or(env::temp_dir().to_str().unwrap().to_string());
    let output_path = std::path::Path::new(&output_dir);
    let new_path = output_path.join("cache");
    // 确保目录存在
    let output_dir = new_path.as_os_str().to_str().unwrap();
    if new_path.exists() {
        output_dir.to_string()
    } else {
        std::fs::create_dir_all(&output_dir).unwrap();
        output_dir.to_string()
    }
}

/**
 * 转换文件
 * 转换后的文件会保存在临时目录，并在一个小时后删除。
 */
pub async fn convert_file_to_pdf(input_url: &str) -> Result<String, Box<dyn std::error::Error>> {
    let libreoffice = env::var("LIBREOFFICE_PATH").expect("请在 .env 中配置 LIBREOFFICE_PATH");
    let output_dir = get_output_dir();
    let input_file = match cache_file(input_url).await {
        Ok(path) => path,
        Err(e) => {
            // 这里报错只有两种可能：下载失败，或者文件类型不支持
            // 如果是 UnsupportedFileTypeError，则不进行转换，直接返回下载的文件
            if let Some(err) = e.downcast_ref::<UnsupportedFileTypeError>() {
                // 移动文件到输出目录
                let file_name = std::path::Path::new(&err.cache_path)
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap();
                let dest_path = format!("{}/{}", output_dir, file_name);
                std::fs::copy(&err.cache_path, &dest_path)?;
                // 移除临时缓存的文件
                let _ = std::fs::remove_file(&err.cache_path);
                return Ok(dest_path);
            } else {
                return Err(e);
            }
        }
    };

    dbg!("输入文件路径: {}", &input_file);

    let status = Command::new(libreoffice)
        .args([
            "--headless",
            "--convert-to",
            "pdf",
            input_file.as_str(),
            "--outdir",
            output_dir.as_str(),
        ])
        .status()?;

    if status.success() {
        println!("转换成功，输出目录: {}", output_dir);
        let pre_file_name = std::path::Path::new(input_file.as_str())
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap();
        let tmp_output_path = format!("{}/{}.pdf", output_dir, pre_file_name,);
        println!("输出文件路径: {}", tmp_output_path);
        // 删除缓存文件
        if crate::url_helper::is_remote_url(&input_url) {
            let _ = std::fs::remove_file(&input_file);
        }
        Ok(tmp_output_path)
    } else {
        eprintln!("转换失败");
        Err("转换失败".into())
    }
}

async fn cache_file(file_url: &str) -> Result<String, Box<dyn std::error::Error>> {
    // Cache remote files to /tmp with their filename
    let decoded = percent_decode_str(&file_url)
        .decode_utf8()
        .unwrap_or_else(|_| file_url.into())
        .into_owned();
    let decoded_file_url = decoded.as_str();
    println!("Caching file...");
    let is_remote = is_remote_url(decoded_file_url);
    let path = if is_remote {
        // 执行下载
        println!("Downloading remote file: {}", decoded_file_url);
        let saved = download_file_to_tmp(decoded_file_url).await?;
        // 验证文件的 mime 类型为合法的 office 文件类型
        if !validate_file(&saved) {
            let err = UnsupportedFileTypeError {
                cache_path: saved.clone(),
            };
            return Err(Box::new(err));
        }
        saved
    } else {
        decoded_file_url.to_string()
    };
    Ok(path)
}

fn validate_file(file_path: &str) -> bool {
    // Placeholder implementation for validating file
    let is_office = is_office_file(file_path);
    println!("Validating file: {} ==> {}", file_path, is_office);
    // etc. 其他的校验逻辑
    // let is_size_valid = check_file_size(file_path);
    // is_office && is_size_valid;
    is_office
}

fn is_office_file(filename: &str) -> bool {
    let ext = filename.to_lowercase();
    ext.ends_with(".doc")
        || ext.ends_with(".docx")
        || ext.ends_with(".xls")
        || ext.ends_with(".xlsx")
        || ext.ends_with(".ppt")
        || ext.ends_with(".pptx")
}

/**
 * 下载远程文件到本地。此方法会把文件缓存到临时目录，并在一个小时后删除文件
 */
async fn download_file_to_tmp(url: &str) -> Result<String, Box<dyn std::error::Error>> {
    // 从请求通中获取文件名和后缀，更新后缀
    let response = reqwest::get(url).await?;

    let success = response.status().is_success();
    if !success {
        return Err(format!("Failed to download file: HTTP {}", response.status()).into());
    }

    let header = response.headers();

    // Determine file extension: prefer extension from URL, then Content-Disposition/Content-Type headers.
    let ext = std::path::Path::new(url)
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .map(|s| s.to_string())
        .or_else(|| get_ext_from_header(header))
        .unwrap_or_else(|| "tmp".to_string());

    let ext = if ext.len() > 10 {
        get_ext_from_header(header).unwrap_or_else(|| "tmp".to_string())
    } else {
        ext
    };

    println!("Determined file extension: {}", ext);

    if ext == "tmp" {
        return Err("Download a unsupported file extension".into());
    }

    // 创建临时文件并写入内容
    let mut tmp_file = NamedTempFile::new()?;
    let bytes = response.bytes().await?;
    tmp_file.write_all(bytes.as_ref())?;

    // 持久化带后缀的文件名
    let tmp_path = tmp_file.path().to_owned();
    let new_temp_path = format!("{}.{}", tmp_path.to_str().unwrap(), ext);
    tmp_file.persist(&new_temp_path)?;

    println!("Downloaded file to temporary path: {}", new_temp_path);
    Ok(new_temp_path)
}

fn header_value_lossy(v: &reqwest::header::HeaderValue) -> &str {
    // 注意：返回 &'static str 不安全，这里演示用 Cow
    Box::leak(
        String::from_utf8_lossy(v.as_bytes())
            .into_owned()
            .into_boxed_str(),
    )
}

fn validate_content_disposition(header: &reqwest::header::HeaderMap) -> Option<&str> {
    if let Some(content_disposition) = header.get(reqwest::header::CONTENT_DISPOSITION) {
        if content_disposition.is_empty() {
            return None;
        }
        return Some(header_value_lossy(content_disposition));
    }
    None
}

/**
 * 从 HTTP 头信息中获取文件后缀
 */
fn get_ext_from_header(header: &reqwest::header::HeaderMap) -> Option<String> {
    // 1. 从 Content-disposition 中获取
    if let Some(content_disposition) = validate_content_disposition(header) {
        return content_disposition_to_ext(content_disposition);
    }

    // 2. 从 Content-type 中获取
    let content_type = header.get(reqwest::header::CONTENT_TYPE)?;
    let content_type_str = content_type.to_str().ok()?;
    return content_type_to_ext(content_type_str);
}

fn content_disposition_to_ext(content_disposition: &str) -> Option<String> {
    let parts: Vec<&str> = content_disposition.split(';').collect();
    for part in parts {
        let part = part.trim();
        let match_str = if part.starts_with("filename=") {
            "filename="
        } else if part.starts_with("filename*=") {
            "filename*="
        } else {
            ""
        };
        if match_str.len() > 0 {
            let filename = part.trim_start_matches(match_str).trim_matches('"');
            let ext = std::path::Path::new(filename)
                .extension()
                .and_then(std::ffi::OsStr::to_str)?;
            return Some(ext.to_string());
        }
    }
    None
}

fn content_type_to_ext(content_type: &str) -> Option<String> {
    match content_type {
        "application/msword" => Some("doc".to_string()),
        "application/vnd.openxmlformats-officedocument.wordprocessingml.document" => {
            Some("docx".to_string())
        }
        "application/vnd.ms-excel" => Some("xls".to_string()),
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => {
            Some("xlsx".to_string())
        }
        "application/vnd.ms-powerpoint" => Some("ppt".to_string()),
        "application/vnd.openxmlformats-officedocument.presentationml.presentation" => {
            Some("pptx".to_string())
        }
        "application/pdf" => Some("pdf".to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dotenv;

    fn init() {
        dotenv::from_path("../.env").ok();
        let _ = dotenv::dotenv();
    }

    #[tokio::test]
    async fn test_download_file() {
        let url = "https://yodeb-dev.oss-cn-shanghai.aliyuncs.com/audio.wav";
        let result = download_file_to_tmp(url).await;
        if result.is_ok() {
            assert!(result.as_ref().unwrap().ends_with(".wav"));
        } else {
            panic!("下载失败: {:?}", result.err());
        }
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_cache_file() {
        let input_file = "https://yodeb-dev.oss-cn-shanghai.aliyuncs.com/test.docx";
        let result = cache_file(input_file).await;
        assert!(result.is_ok());
        println!("缓存后的文件路径: {:?}", result.unwrap());
    }

    #[tokio::test]
    async fn test_convert_file() {
        init();
        let input_file = "https://yodeb-dev.oss-cn-shanghai.aliyuncs.com/test.docx";
        let result = convert_file_to_pdf(input_file).await;
        assert!(result.is_ok());
        println!("转换后的文件路径: {:?}", result.unwrap());
    }

    async fn test_reqwest() {
        let url = "";
    }
}
