//! 配置加载和解密
//!
//! 负责从不同来源加载配置并进行解密

use super::encryption;
use super::types::{AppConfig, ServerConfig};
use crate::core::error::{Error, Result};
use config::{Config, FileFormat};

/// 编译期从 OUT_DIR 中引入加密后的配置二进制
const ENCRYPTED_CONFIG: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/config_encrypted.bin"));

/// 解密配置内容
fn decrypt_config() -> Result<String> {
    let decrypted =
        encryption::decrypt(ENCRYPTED_CONFIG).map_err(|e| Error::ConfigDecryptFailed(e))?;
    String::from_utf8(decrypted).map_err(|e| Error::ConfigDecryptFailed(e.to_string()))
}

/// 从字符串加载配置
pub fn load_from_str(config_str: &str) -> Result<AppConfig> {
    let config = Config::builder()
        .add_source(config::File::from_str(config_str, FileFormat::Toml))
        .add_source(config::File::with_name(".").required(false))
        .add_source(config::Environment::with_prefix("APP"))
        .build()
        .map_err(|e| Error::ConfigLoadFailed(e.to_string()))?;

    config.try_deserialize().map_err(|e| Error::ConfigParseFailed(e.to_string()))
}

/// 从文件路径加载配置
pub fn load_from_path(config_path: &str) -> Result<AppConfig> {
    let config = Config::builder()
        .add_source(config::File::with_name(config_path))
        .add_source(config::File::with_name(".").required(false))
        .add_source(config::Environment::with_prefix("APP"))
        .build()
        .map_err(|e| Error::ConfigLoadFailed(e.to_string()))?;

    config.try_deserialize().map_err(|e| Error::ConfigParseFailed(e.to_string()))
}

/// 从用户配置文件加载服务器配置
///
/// 文件为 TOML 格式，结构如下：
/// ```toml
/// [server]
/// base_url = "https://example.com/api/"
/// version = "v1"
/// secret_key = "..."
/// ```
/// 文件不存在返回 Ok(None)；文件存在但结构缺失 server 段也返回 Ok(None)。
pub fn load_user_config(path: &std::path::Path) -> Result<Option<ServerConfig>> {
    if !path.exists() {
        return Ok(None);
    }

    let content = std::fs::read_to_string(path).map_err(|e| {
        Error::ConfigLoadFailed(format!("读取用户配置文件失败: {}", e))
    })?;

    let config = Config::builder()
        .add_source(config::File::from_str(&content, FileFormat::Toml))
        .build()
        .map_err(|e| Error::ConfigLoadFailed(format!("解析用户配置文件失败: {}", e)))?;

    match config.try_deserialize::<ServerConfig>() {
        Ok(server) => Ok(Some(server)),
        Err(e) => {
            log::warn!("[config] 解析用户配置文件 server 段失败: {}", e);
            Ok(None)
        }
    }
}

/// 将服务器配置保存到用户配置文件
pub fn save_user_config(path: &std::path::Path, server: &ServerConfig) -> Result<()> {
    use std::io::Write;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| Error::ConfigSaveFailed(format!("创建配置目录失败: {}", e)))?;
    }

    let content = format!(
        "[server]\nbase_url = {:?}\nversion = {:?}\nsecret_key = {:?}\n",
        server.base_url, server.version, server.secret_key
    );

    let mut file = std::fs::File::create(path)
        .map_err(|e| Error::ConfigSaveFailed(format!("创建配置文件失败: {}", e)))?;
    file.write_all(content.as_bytes())
        .map_err(|e| Error::ConfigSaveFailed(format!("写入配置文件失败: {}", e)))?;

    Ok(())
}

/// 加载嵌入的加密配置
pub fn load_embedded() -> Result<AppConfig> {
    let config_str = decrypt_config()?;
    load_from_str(&config_str)
}
