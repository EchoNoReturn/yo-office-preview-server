use std::{env, io::Write, process::Command};
use tempfile::NamedTempFile;

/**
 * 转换文件
 * 转换后的文件会保存在临时目录，并在一个小时后删除。
 */
pub async fn convert_file_to_pdf(input_url: &str) -> Result<String, Box<dyn std::error::Error>> {
    let libreoffice = env::var("LIBREOFFICE_PATH").expect("请在 .env 中配置 LIBREOFFICE_PATH");
    let input_file = cache_file(input_url).await?;
    let output_dir =
        env::var("OUTPUT_DIR").unwrap_or(env::temp_dir().to_str().unwrap().to_string());

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
        let tmp_output_path = format!(
            "{}/{}.pdf",
            output_dir,
            std::path::Path::new(input_file.as_str())
                .file_stem()
                .unwrap()
                .to_str()
                .unwrap()
        );
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
    println!("Caching file...");
    let is_remote = crate::url_helper::is_remote_url(file_url);
    let path = if is_remote {
        // 执行下载
        println!("Downloading remote file: {}", file_url);
        let saved = download_file_to_tmp(file_url).await?;
        // 验证文件的 mime 类型为合法的 office 文件类型
        if !vailedate_file(&saved) {
            let _ = std::fs::remove_file(&saved);
            return Err("不支持的文件类型".into());
        }
        saved
    } else {
        file_url.to_string()
    };
    Ok(path)
}

fn vailedate_file(file_path: &str) -> bool {
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
    let header = response.headers();

    // Determine file extension: prefer extension from URL, then Content-Disposition/Content-Type headers.
    let ext = std::path::Path::new(url)
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .map(|s| s.to_string())
        .or_else(|| get_ext_from_header(header))
        .unwrap_or_else(|| "tmp".to_string());

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

/**
 * 从 HTTP 头信息中获取文件后缀
 */
fn get_ext_from_header(header: &reqwest::header::HeaderMap) -> Option<String> {
    // 1. 从 Content-disposition 中获取
    if let Some(content_disposition) = header.get(reqwest::header::CONTENT_DISPOSITION) {
        let result = content_disposition_to_ext(content_disposition.to_str().ok()?);
        if result.is_some() {
            return result;
        }
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
        if part.starts_with("filename=") {
            let filename = part.trim_start_matches("filename=").trim_matches('"');
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
}
