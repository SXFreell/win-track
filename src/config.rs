//! 配置管理模块

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 应用配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Webhook 配置
    pub webhook: WebhookConfig,
    /// 开机/关机消息配置
    pub session_messages: SessionMessageConfig,
    /// 定时消息配置
    pub scheduled_messages: Vec<ScheduledMessage>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            webhook: WebhookConfig::default(),
            session_messages: SessionMessageConfig::default(),
            scheduled_messages: vec![],
        }
    }
}

/// Webhook 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookConfig {
    /// Webhook URL
    pub url: String,
    /// 自定义请求体模板 (JSON 字符串)，使用 {{MARKDOWN}} 作为 Markdown 内容占位符
    pub body_template: String,
    /// Content-Type 请求头
    pub content_type: String,
}

impl Default for WebhookConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            body_template: r#"{"content": "{{MARKDOWN}}"}"#.to_string(),
            content_type: "application/json".to_string(),
        }
    }
}

/// 开机/关机消息配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMessageConfig {
    /// 开机时是否发送
    pub on_boot: bool,
    /// 关机时是否发送
    pub on_shutdown: bool,
}

impl Default for SessionMessageConfig {
    fn default() -> Self {
        Self {
            on_boot: true,
            on_shutdown: true,
        }
    }
}

/// 定时消息配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduledMessage {
    /// 间隔分钟数 (如 60 表示每小时)
    pub interval_minutes: u64,
    /// 是否启用
    pub enabled: bool,
}

impl Config {
    /// 获取配置文件路径
    pub fn config_path() -> Option<PathBuf> {
        directories::ProjectDirs::from("com", "win-track", "WinTrack")
            .map(|d| d.config_dir().join("config.json"))
    }

    /// 加载配置
    pub fn load() -> Self {
        match Self::config_path() {
            Some(path) if path.exists() => {
                match std::fs::read_to_string(&path) {
                    Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
                    Err(_) => Config::default(),
                }
            }
            _ => Config::default(),
        }
    }

    /// 保存配置
    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(path) = Self::config_path() {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(path, serde_json::to_string_pretty(self)?)?;
        }
        Ok(())
    }
}
