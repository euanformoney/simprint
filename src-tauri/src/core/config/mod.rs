//! 配置管理模块
//!
//! 提供配置的加载、验证和全局访问

mod encryption;
mod loader;
mod types;
mod validator;

pub use types::{AppConfig, ServerConfig, UpdaterConfig};

use crate::core::error::{Error, Result};
use std::{fs, path::PathBuf, sync::OnceLock};

/// 全局配置实例
static CONFIG: OnceLock<AppConfig> = OnceLock::new();

/// 用户配置文件名称（与 store.json 同级，存放用户自定义的服务器配置）
pub const USER_CONFIG_FILE_NAME: &str = "server-config.toml";

/// 初始化配置（从嵌入的加密配置）
///
/// 这是最常用的初始化方式，配置在编译时被加密并嵌入到二进制中
pub fn init() -> Result<()> {
    let config = if let Some(config_name) = local_config_file_name() {
        let config_path = resolve_local_config_path(config_name);
        let config_str = fs::read_to_string(&config_path)
            .map_err(|e| Error::ConfigLoadFailed(format!("{}: {}", config_path.display(), e)))?;
        loader::load_from_str(&config_str)?
    } else {
        loader::load_embedded()?
    };

    // 用户自定义配置优先：若存在用户配置文件，则用其覆盖内置配置
    let config = apply_user_config(config)?;

    validator::validate(&config)?;

    CONFIG.set(config).map_err(|_| Error::ConfigAlreadyInitialized)?;
    Ok(())
}

/// 尝试加载用户自定义的服务器配置（server-config.toml），存在则覆盖内置配置
fn apply_user_config(mut config: AppConfig) -> Result<AppConfig> {
    if let Ok(path) = user_config_path() {
        if path.exists() {
            match loader::load_user_config(&path) {
                Ok(Some(user_server)) => {
                    config.server = user_server;
                }
                Ok(None) => {}
                Err(e) => {
                    // 用户配置文件损坏时回退到内置配置，不阻塞启动
                    log::warn!("[config] 读取用户配置文件失败，回退到内置配置: {}", e);
                }
            }
        }
    }
    Ok(config)
}

/// 用户配置文件路径：应用配置目录下的 server-config.toml
pub fn user_config_path() -> Result<PathBuf> {
    crate::core::paths::PathManager::get_config_dir()
        .map(|dir| dir.join(USER_CONFIG_FILE_NAME))
        .map_err(|e| Error::ConfigLoadFailed(format!("获取配置目录失败: {}", e)))
}

/// 保存用户自定义的服务器配置（覆盖内置配置，重启后生效）
pub fn save_user_server_config(server: &ServerConfig) -> Result<()> {
    let path = user_config_path()?;
    loader::save_user_config(&path, server)
}

/// 读取用户自定义的服务器配置（若存在）
pub fn load_user_server_config() -> Result<Option<ServerConfig>> {
    let path = user_config_path()?;
    loader::load_user_config(&path)
}

/// 当前生效的服务器配置（用户配置优先，否则回退到内置配置）
pub fn effective_server_config() -> Result<ServerConfig> {
    if let Some(user) = load_user_server_config()? {
        return Ok(user);
    }
    Ok(get_or_err()?.server.clone())
}

/// 初始化配置（从字符串）
///
/// 用于测试或从其他来源加载配置
pub fn init_from_str(config_str: &str) -> Result<()> {
    let config = loader::load_from_str(config_str)?;
    validator::validate(&config)?;

    CONFIG.set(config).map_err(|_| Error::ConfigAlreadyInitialized)?;
    Ok(())
}

/// 初始化配置（从文件路径）
///
/// 用于开发环境或特殊场景
pub fn init_from_path(config_path: &str) -> Result<()> {
    let config = loader::load_from_path(config_path)?;
    validator::validate(&config)?;

    CONFIG.set(config).map_err(|_| Error::ConfigAlreadyInitialized)?;
    Ok(())
}

/// 获取配置（返回 Option）
///
/// 如果配置未初始化，返回 None
pub fn get() -> Option<&'static AppConfig> {
    CONFIG.get()
}

/// 获取配置（必须已初始化）
///
/// 如果配置未初始化，返回错误
pub fn get_or_err() -> Result<&'static AppConfig> {
    CONFIG.get().ok_or(Error::ConfigNotInitialized)
}

/// 获取配置（必须已初始化，否则 panic）
///
/// 仅在确定配置已初始化的场景使用
pub fn get_or_panic() -> &'static AppConfig {
    CONFIG.get().expect("Config not initialized. Call config::init() first.")
}

fn local_config_file_name() -> Option<&'static str> {
    if cfg!(feature = "test") {
        Some("config.test.toml")
    } else if cfg!(feature = "development") {
        Some("config.development.toml")
    } else {
        None
    }
}

fn resolve_local_config_path(config_name: &str) -> PathBuf {
    let cwd_path = PathBuf::from(config_name);
    if cwd_path.exists() {
        return cwd_path;
    }

    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(config_name)
}
