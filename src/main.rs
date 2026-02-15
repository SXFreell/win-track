//! WinTrack - Windows 使用日志统计与提醒

mod config;
mod webhook;

use config::Config;
use std::sync::Arc;
use tao::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoopBuilder},
    window::WindowBuilder,
};
use tokio::sync::RwLock;
use tray_icon::{menu::MenuEvent, menu::MenuItem, Icon, TrayIconBuilder};
use wry::WebViewBuilder;
use image::{ImageBuffer, Rgba};

fn create_tray_icon() -> Result<Icon, Box<dyn std::error::Error>> {
    let (w, h) = (16, 16);
    let mut img = ImageBuffer::new(w, h);
    for y in 0..h {
        for x in 0..w {
            let pixel = if (x as i32 - 8).abs() < 6 && (y as i32 - 8).abs() < 6 {
                Rgba([0, 120, 212, 255])
            } else {
                Rgba([0, 0, 0, 0])
            };
            img.put_pixel(x, y, pixel);
        }
    }
    let icon = Icon::from_rgba(img.into_raw(), w, h)?;
    Ok(icon)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();
    log::info!("WinTrack 启动");

    let config = Arc::new(RwLock::new(Config::load()));

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;

    // 开机时发送 webhook
    {
        let config = config.clone();
        rt.block_on(async {
            let c = config.read().await;
            if c.session_messages.on_boot && !c.webhook.url.is_empty() {
                let markdown = webhook::boot_markdown();
                drop(c);
                if let Err(e) =
                    webhook::send_webhook(&config.read().await.webhook, &markdown).await
                {
                    log::error!("开机 Webhook 发送失败: {}", e);
                }
            }
        });
    }

    #[derive(Clone)]
    enum UserEvent {
        Config,
        Exit,
        ConfigSaved,
        ConfigClosed,
    }

    let event_loop = EventLoopBuilder::<UserEvent>::with_user_event().build()?;
    let proxy = event_loop.create_proxy();

    // 创建托盘菜单 (muda API: MenuItem::new(text, enabled, accelerator))
    let config_item = MenuItem::new("配置", true, None);
    let quit_item = MenuItem::new("退出", true, None);
    let tray_menu = tray_icon::menu::Menu::new();
    tray_menu.append(&config_item)?;
    tray_menu.append(&quit_item)?;

    let config_id = config_item.id().clone();
    let quit_id = quit_item.id().clone();

    let icon = create_tray_icon()?;
    let _tray_icon = TrayIconBuilder::new()
        .with_menu(Box::new(tray_menu))
        .with_tooltip("WinTrack - 使用日志统计")
        .with_icon(icon)
        .build()?;

    MenuEvent::set_event_handler(Some(move |event| {
        if event.id == config_id {
            let _ = proxy.send_event(UserEvent::Config);
        } else if event.id == quit_id {
            let _ = proxy.send_event(UserEvent::Exit);
        }
    }));

    // 定时消息任务
    {
        let config = config.clone();
        rt.spawn(async move {
            let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
            let mut last_send = std::time::Instant::now();
            loop {
                interval.tick().await;
                let c = config.read().await;
                if let Some(schedule) = c.scheduled_messages.first() {
                    if schedule.enabled && schedule.interval_minutes > 0 {
                        let mins = last_send.elapsed().as_secs() / 60;
                        if mins >= schedule.interval_minutes {
                            let hours = mins / 60;
                            let remainder_mins = mins % 60;
                            let markdown = webhook::scheduled_markdown(hours, remainder_mins);
                            drop(c);
                            if let Err(e) =
                                webhook::send_webhook(&config.read().await.webhook, &markdown).await
                            {
                                log::error!("定时 Webhook 发送失败: {}", e);
                            }
                            last_send = std::time::Instant::now();
                        }
                    }
                }
            }
        });
    }

    let mut config_window_open = false;
    let mut config_window_id: Option<tao::window::WindowId> = None;
    let mut config_window: Option<tao::window::Window> = None;
    let mut config_webview: Option<wry::WebView> = None;

    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Wait);

        match event {
            Event::UserEvent(UserEvent::Config) => {
                if config_window_open {
                    return;
                }
                config_window_open = true;

                let config = config.clone();
                let elwt = elwt.clone();
                let proxy = elwt.create_proxy();

                let window_attrs = tao::window::WindowAttributes::default()
                    .with_title("WinTrack 配置")
                    .with_inner_size(tao::dpi::LogicalSize::new(520.0, 580.0))
                    .with_resizable(true);

                let window = match WindowBuilder::new()
                    .with_window_attrs(window_attrs)
                    .build(&elwt)
                {
                    Ok(w) => w,
                    Err(e) => {
                        log::error!("创建配置窗口失败: {}", e);
                        config_window_open = false;
                        return;
                    }
                };

                config_window_id = Some(window.id());

                let html = include_str!("../resources/config.html");
                let config_for_js = rt.block_on(async {
                    let c = config.read().await;
                    serde_json::to_string(&*c).unwrap_or_default()
                });

                // 注入配置到 HTML
                let html_with_config = html.replace(
                    "<script>",
                    &format!(
                        "<script>const __WINTRACK_CONFIG__={};</script><script>",
                        config_for_js
                    ),
                ).replace(
                    "window.loadConfig = function(config) {",
                    "window.loadConfig = function(config) { var c = config || (typeof __WINTRACK_CONFIG__!=='undefined' ? __WINTRACK_CONFIG__ : null);",
                ).replace(
                    "if (config) {",
                    "if (c) {",
                ).replace(
                    "document.getElementById('webhookUrl').value = config.webhook?.url || ''",
                    "document.getElementById('webhookUrl').value = c.webhook?.url || ''",
                ).replace(
                    "document.getElementById('bodyTemplate').value = config.webhook?.body_template || '{\"content\": \"{{MARKDOWN}}\"}'",
                    "document.getElementById('bodyTemplate').value = c.webhook?.body_template || '{\"content\": \"{{MARKDOWN}}\"}'",
                ).replace(
                    "document.getElementById('contentType').value = config.webhook?.content_type || 'application/json'",
                    "document.getElementById('contentType').value = c.webhook?.content_type || 'application/json'",
                ).replace(
                    "document.getElementById('onBoot').checked = config.session_messages?.on_boot !== false",
                    "document.getElementById('onBoot').checked = c.session_messages?.on_boot !== false",
                ).replace(
                    "document.getElementById('onShutdown').checked = config.session_messages?.on_shutdown !== false",
                    "document.getElementById('onShutdown').checked = c.session_messages?.on_shutdown !== false",
                ).replace(
                    "const schedule = config.scheduled_messages?.[0]",
                    "const schedule = c.scheduled_messages?.[0]",
                );

                let config_for_protocol = config.clone();

                let webview = match WebViewBuilder::new(&window)
                    .with_html(&html_with_config)
                    .with_navigation_handler(move |url| {
                        // Windows WebView2 可能将 wintrack:// 转为 http://wintrack./
                        if url.contains("save") && url.contains("config=") {
                            let query = url.split('?').nth(1).unwrap_or("");
                            for part in query.split('&') {
                                    if let Some((k, v)) = part.split_once('=') {
                                        if k == "config" {
                                            if let Ok(decoded) = urlencoding::decode(v) {
                                                if let Ok(new_config) =
                                                    serde_json::from_str::<Config>(&decoded)
                                                {
                                                    let cfg = config_for_protocol.clone();
                                                    tokio::spawn(async move {
                                                        let mut c = cfg.write().await;
                                                        *c = new_config;
                                                        if let Err(e) = c.save() {
                                                            log::error!("保存配置失败: {}", e);
                                                        }
                                                    });
                                                }
                                            }
                                        }
                                    }
                                }
                            let _ = proxy.send_event(UserEvent::ConfigSaved);
                            return false;
                        }
                        if (url.contains("wintrack") || url.contains("wintrack.")) && url.contains("close") {
                            let _ = proxy.send_event(UserEvent::ConfigClosed);
                            return false;
                        }
                        true
                    })
                    .build()
                {
                    Ok(w) => w,
                    Err(e) => {
                        log::error!("创建 WebView 失败: {}", e);
                        config_window_open = false;
                        config_window_id = None;
                        return;
                    }
                };

                config_window = Some(window);
                config_webview = Some(webview);
                config_window.as_ref().unwrap().set_focus();
            }
            Event::UserEvent(UserEvent::Exit) => {
                let config = config.clone();
                rt.block_on(async {
                    let c = config.read().await;
                    if c.session_messages.on_shutdown && !c.webhook.url.is_empty() {
                        let markdown = webhook::shutdown_markdown();
                        drop(c);
                        let _ =
                            webhook::send_webhook(&config.read().await.webhook, &markdown).await;
                    }
                });
                elwt.exit();
            }
            Event::UserEvent(UserEvent::ConfigSaved) | Event::UserEvent(UserEvent::ConfigClosed) => {
                config_webview = None;
                config_window = None;
                config_window_open = false;
                config_window_id = None;
            }
            Event::WindowEvent {
                window_id,
                event: WindowEvent::CloseRequested,
            } => {
                if config_window_id == Some(window_id) {
                    config_window_open = false;
                    config_window_id = None;
                }
            }
            _ => {}
        }
    })?;

    Ok(())
}
