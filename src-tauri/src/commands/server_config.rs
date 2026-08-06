//! 服务器配置命令
//!
//! 提供前端配置向导所需的能力：
//! - get_server_config：读取当前生效的服务器配置
//! - save_server_config：保存用户自定义的服务器配置（重启后生效）
//! - test_server_connection：测试服务器连通性

use crate::core::config;
use crate::core::config::ServerConfig;
use crate::core::error::Result;
use serde::{Deserialize, Serialize};
use tauri::command;

/// 服务器配置返回结构
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ServerConfigDto {
    /// 服务器基础 URL
    pub base_url: String,
    /// API 版本
    pub version: String,
    /// 密钥
    pub secret_key: String,
    /// 是否为用户自定义配置（否则为编译时内置配置）
    pub is_user_configured: bool,
}

impl From<&ServerConfig> for ServerConfigDto {
    fn from(c: &ServerConfig) -> Self {
        Self {
            base_url: c.base_url.clone(),
            version: c.version.clone(),
            secret_key: c.secret_key.clone(),
            is_user_configured: false,
        }
    }
}

/// 读取当前生效的服务器配置（用户配置优先，否则内置配置）
#[command]
pub fn get_server_config() -> Result<ServerConfigDto> {
    let user = config::load_user_server_config()?;
    match user {
        Some(user_cfg) => {
            let mut dto = ServerConfigDto::from(&user_cfg);
            dto.is_user_configured = true;
            Ok(dto)
        }
        None => {
            let builtin = config::get_or_err()?.server.clone();
            Ok(ServerConfigDto::from(&builtin))
        }
    }
}

/// 保存用户自定义的服务器配置
///
/// 写入用户配置文件，重启应用后生效（config 在启动时加载）。
/// 返回写入结果，成功后可调用前端 relaunch 重启应用。
#[command]
pub fn save_server_config(
    base_url: String,
    version: String,
    secret_key: String,
) -> Result<()> {
    let server = ServerConfig {
        base_url: base_url.trim().to_string(),
        version: version.trim().to_string(),
        secret_key: secret_key.trim().to_string(),
    };

    // 校验
    if server.base_url.is_empty() {
        return Err("服务器地址不能为空".into());
    }
    if !server.base_url.starts_with("http://") && !server.base_url.starts_with("https://") {
        return Err("服务器地址必须以 http:// 或 https:// 开头".into());
    }
    if server.version.is_empty() {
        return Err("API 版本不能为空".into());
    }
    if server.secret_key.is_empty() {
        return Err("secret key 不能为空".into());
    }

    // 规范化：确保 base_url 以 / 结尾，避免 Url::join 时丢失路径段
    let mut server = server;
    if !server.base_url.ends_with('/') {
        server.base_url.push('/');
    }

    config::save_user_server_config(&server)?;
    Ok(())
}

/// 测试服务器连通性
///
/// 使用传入的配置向服务器的公开接口发起请求：
/// - GET {base_url}/{version}/time/now  （无需加密的公开接口）
/// - GET {base_url}/secret/public/key   （公钥接口）
///
/// 两者任一成功即视为连通。
#[command]
pub async fn test_server_connection(
    base_url: String,
    version: String,
    _secret_key: String,
) -> Result<TestConnectionResult> {
    let base_url = base_url.trim().trim_end_matches('/').to_string();
    let version = version.trim().trim_start_matches('/').to_string();

    if base_url.is_empty() {
        return Err("服务器地址不能为空".into());
    }
    if !base_url.starts_with("http://") && !base_url.starts_with("https://") {
        return Err("服务器地址必须以 http:// 或 https:// 开头".into());
    }

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .danger_accept_invalid_certs(true) // 自建服务器可能使用自签名证书
        .build()
        .map_err(|e| format!("创建 HTTP 客户端失败: {}", e))?;

    // 尝试多个接口，任一成功即连通
    let endpoints = [
        format!("{}/{}/time/now", base_url, version),
        format!("{}/{}/secret/public/key", base_url, version),
    ];

    let mut last_error: Option<String> = None;
    for url in endpoints.iter() {
        match client.get(url).send().await {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    return Ok(TestConnectionResult {
                        ok: true,
                        message: format!("连接成功（HTTP {}）", status.as_u16()),
                        http_status: Some(status.as_u16()),
                    });
                }
                last_error = Some(format!("HTTP {}", status.as_u16()));
            }
            Err(e) => {
                last_error = Some(e.to_string());
            }
        }
    }

    Ok(TestConnectionResult {
        ok: false,
        message: format!("连接失败: {}", last_error.unwrap_or_else(|| "未知错误".into())),
        http_status: None,
    })
}

/// 连接测试结果
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TestConnectionResult {
    /// 是否连接成功
    pub ok: bool,
    /// 提示信息
    pub message: String,
    /// HTTP 状态码（若有）
    pub http_status: Option<u16>,
}
