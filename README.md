# WinTrack - Windows 使用日志统计与提醒

Windows 托盘应用，用于记录开机/关机时间，并通过 Webhook 发送消息。支持自定义 Body 模板和定时提醒。

## 功能特性

- **开机/关机记录**：记录开机和关机时间
- **Webhook 通知**：开机、关机时通过 Webhook 发送消息
- **自定义 Body**：支持配置自定义 JSON body 结构，使用 `{{MARKDOWN}}` 作为 Markdown 内容占位符
- **定时消息**：可配置定时提醒（如开机后每小时发送一次）
- **系统托盘**：最小化到托盘，右键菜单提供「配置」和「退出」
- **Fluent 风格配置界面**：基于 WebView 的配置弹窗

## 系统要求

- Windows 10/11
- [WebView2](https://developer.microsoft.com/en-us/microsoft-edge/webview2/) 运行时（Windows 11 通常已预装）

## 构建

```bash
# 在 Windows 上构建
cargo build --release

# 或指定 Windows 目标（在 macOS/Linux 上交叉编译）
cargo build --release --target x86_64-pc-windows-msvc
```

## 运行

```bash
cargo run --release
```

或直接运行 `target/release/win-track.exe`

## 开机自启

将 `win-track.exe` 的快捷方式放入以下目录即可实现开机自启：

```
%APPDATA%\Microsoft\Windows\Start Menu\Programs\Startup
```

## 配置

配置文件位于：

```
%APPDATA%\win-track\WinTrack\config\config.json
```

或通过托盘右键 → 配置 打开配置界面。

### 配置示例

```json
{
  "webhook": {
    "url": "https://your-webhook-url.com/...",
    "body_template": "{\"content\": \"{{MARKDOWN}}\"}",
    "content_type": "application/json"
  },
  "session_messages": {
    "on_boot": true,
    "on_shutdown": true
  },
  "scheduled_messages": [
    {
      "interval_minutes": 60,
      "enabled": true
    }
  ]
}
```

### Body 模板说明

- `{{MARKDOWN}}` 会被替换为实际的消息内容（Markdown 格式）
- 支持任意 JSON 结构，例如钉钉、飞书、企业微信等

钉钉示例：
```json
{
  "msgtype": "markdown",
  "markdown": {
    "title": "WinTrack",
    "text": "{{MARKDOWN}}"
  }
}
```

## 消息格式

- **开机消息**：包含开机时间
- **关机消息**：包含关机时间
- **定时消息**：包含当前时间和距离开机时长

## 技术栈

- Rust
- tao - 窗口与事件循环
- tray-icon - 系统托盘
- wry - WebView（配置界面）
- tokio - 异步运行时
- reqwest - HTTP 客户端
