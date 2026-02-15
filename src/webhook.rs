//! Webhook 发送模块

use crate::config::{Config, WebhookConfig};
use chrono::Local;
use reqwest::Client;
use std::sync::Arc;
use tokio::sync::RwLock;

/// 生成开机消息的 Markdown 内容
pub fn boot_markdown() -> String {
    let now = Local::now();
    format!(
        "## 🖥️ 电脑已开机\n\n**时间**: {}\n\n*WinTrack 自动记录*",
        now.format("%Y-%m-%d %H:%M:%S")
    )
}

/// 生成关机消息的 Markdown 内容
pub fn shutdown_markdown() -> String {
    let now = Local::now();
    format!(
        "## 🔌 电脑已关机\n\n**时间**: {}\n\n*WinTrack 自动记录*",
        now.format("%Y-%m-%d %H:%M:%S")
    )
}

/// 生成定时消息的 Markdown 内容
pub fn scheduled_markdown(hours: u64, minutes: u64) -> String {
    let now = Local::now();
    format!(
        "## ⏰ 定时提醒\n\n**时间**: {}\n\n**距离开机已过**: {} 小时 {} 分钟\n\n*WinTrack 定时提醒*",
        now.format("%Y-%m-%d %H:%M:%S"),
        hours,
        minutes
    )
}

/// 将 Markdown 插入到 body 模板的指定位置
fn build_body(template: &str, markdown: &str) -> String {
    template.replace("{{MARKDOWN}}", markdown)
}

/// 发送 webhook 请求
pub async fn send_webhook(
    config: &WebhookConfig,
    markdown: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if config.url.is_empty() {
        return Ok(());
    }

    let body = build_body(&config.body_template, markdown);

    // 尝试解析为 JSON 以验证格式
    let json_value: serde_json::Value = serde_json::from_str(&body)
        .map_err(|e| format!("Body 必须是有效 JSON，且 {{MARKDOWN}} 占位符会被替换: {}", e))?;

    let client = Client::new();
    let response = client
        .post(&config.url)
        .header("Content-Type", &config.content_type)
        .json(&json_value)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!(
            "Webhook 请求失败: {} {}",
            response.status(),
            response.text().await.unwrap_or_default()
        )
        .into());
    }

    Ok(())
}

/// 后台发送 webhook（不阻塞）
pub fn send_webhook_background(config: Arc<RwLock<Config>>, markdown: String) {
    tokio::spawn(async move {
        let config = config.read().await;
        if let Err(e) = send_webhook(&config.webhook, &markdown).await {
            log::error!("Webhook 发送失败: {}", e);
        }
    });
}
