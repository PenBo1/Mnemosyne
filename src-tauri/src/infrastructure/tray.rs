//! ═══════════════════════════════════════════════════════════════════════════
//! 系统托盘 - 系统托盘菜单与窗口管理
//! ═══════════════════════════════════════════════════════════════════════════

use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::{TrayIcon, TrayIconBuilder},
    AppHandle, Manager, Runtime,
};
use crate::ShutdownToken;

// ── 托盘菜单项 ID ──────────────────────────────────────────────────────────

const MENU_SHOW: &str = "show";
const MENU_ABOUT: &str = "about";
const MENU_PROCESS_MONITOR: &str = "process_monitor";
const MENU_LOG_VIEWER: &str = "log_viewer";
const MENU_QUIT: &str = "quit";

// ── 托盘构建 ────────────────────────────────────────────────────────────────

/// 构建系统托盘
///
/// 托盘菜单包含：
/// - 打开 Mnemosyne（显示主窗口）
/// - 关于 Mnemosyne（打开关于窗口）
/// - 进程监视器（打开进程监视器窗口）
/// - 查看日志（打开日志查看器窗口）
/// - 退出（完全退出应用）
pub fn build_tray<R: Runtime>(app: &AppHandle<R>) -> Result<TrayIcon<R>, Box<dyn std::error::Error>> {
    tracing::info!("[tray] 开始构建系统托盘");
    
    // 构建托盘菜单
    let show_item = MenuItemBuilder::new("打开 Mnemosyne")
        .id(MENU_SHOW)
        .build(app)?;
    let about_item = MenuItemBuilder::new("关于 Mnemosyne")
        .id(MENU_ABOUT)
        .build(app)?;
    let process_item = MenuItemBuilder::new("进程监视器")
        .id(MENU_PROCESS_MONITOR)
        .build(app)?;
    let log_item = MenuItemBuilder::new("查看日志")
        .id(MENU_LOG_VIEWER)
        .build(app)?;
    let quit_item = MenuItemBuilder::new("退出")
        .id(MENU_QUIT)
        .build(app)?;
    
    tracing::info!(
        show_id = ?show_item.id().as_ref(),
        about_id = ?about_item.id().as_ref(),
        process_id = ?process_item.id().as_ref(),
        log_id = ?log_item.id().as_ref(),
        quit_id = ?quit_item.id().as_ref(),
        "[tray] 菜单项创建完成"
    );

    let menu = MenuBuilder::new(app)
        .item(&show_item)
        .separator()
        .item(&about_item)
        .item(&process_item)
        .item(&log_item)
        .separator()
        .item(&quit_item)
        .build()?;
    
    tracing::info!("[tray] 菜单构建完成");

    // 构建托盘图标
    let tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .on_menu_event(|app, event| {
            tracing::info!(
                menu_id = ?event.id.as_ref(),
                "[tray] 菜单事件触发"
            );
            
            match event.id.as_ref() {
                MENU_SHOW => {
                    tracing::info!("[tray] MENU_SHOW 事件处理");
                    if let Some(window) = app.get_webview_window("main") {
                        tracing::info!("[tray] 找到主窗口，正在显示");
                        if let Err(e) = window.show() {
                            tracing::error!("[tray] 显示主窗口失败: {}", e);
                        }
                        if let Err(e) = window.set_focus() {
                            tracing::error!("[tray] 聚焦主窗口失败: {}", e);
                        }
                    } else {
                        tracing::warn!("[tray] 未找到主窗口");
                    }
                }
                MENU_ABOUT => {
                    tracing::info!("[tray] MENU_ABOUT 事件处理");
                    open_about_window(app);
                }
                MENU_PROCESS_MONITOR => {
                    tracing::info!("[tray] MENU_PROCESS_MONITOR 事件处理");
                    open_process_monitor_window(app);
                }
                MENU_LOG_VIEWER => {
                    tracing::info!("[tray] MENU_LOG_VIEWER 事件处理");
                    open_log_viewer_window(app);
                }
                MENU_QUIT => {
                    tracing::info!("[tray] MENU_QUIT 事件处理");
                    if let Some(shutdown_token) = app.try_state::<ShutdownToken>() {
                        shutdown_token.0.cancel();
                    }
                    app.exit(0);
                }
                id => {
                    tracing::warn!("[tray] 未知的菜单项 ID: {}", id);
                }
            }
        })
        .build(app)?;

    tracing::info!("[tray] 托盘构建成功");
    Ok(tray)
}

// ── 窗口打开函数 ────────────────────────────────────────────────────────────

/// 打开关于窗口（独立窗口）
fn open_about_window<R: Runtime>(app: &AppHandle<R>) {
    let label = "about";
    tracing::info!("[tray] 开始打开关于窗口, label={}", label);
    
    // 如果窗口已存在，聚焦它
    if let Some(window) = app.get_webview_window(label) {
        tracing::info!("[tray] 关于窗口已存在，正在聚焦");
        if let Err(e) = window.show() {
            tracing::error!("[tray] 显示关于窗口失败: {}", e);
        }
        if let Err(e) = window.set_focus() {
            tracing::error!("[tray] 聚焦关于窗口失败: {}", e);
        }
        return;
    }

    tracing::info!("[tray] 创建新的关于窗口");
    
    // 创建新窗口 - 使用独立入口文件
    let result = tauri::WebviewWindowBuilder::new(
        app,
        label,
        tauri::WebviewUrl::App("src/about/index.html".into()),
    )
    .title("关于 Mnemosyne")
    .inner_size(500.0, 600.0)
    .resizable(false)
    .maximizable(false)
    .closable(true)
    .build();

    match result {
        Ok(_) => tracing::info!("[tray] 关于窗口创建成功"),
        Err(e) => tracing::error!("[tray] 关于窗口创建失败: {}", e),
    }
}

/// 打开进程监视器窗口
fn open_process_monitor_window<R: Runtime>(app: &AppHandle<R>) {
    let label = "process_monitor";
    tracing::info!("[tray] 开始打开进程监视器窗口, label={}", label);
    
    // 如果窗口已存在，聚焦它
    if let Some(window) = app.get_webview_window(label) {
        tracing::info!("[tray] 进程监视器窗口已存在，正在聚焦");
        if let Err(e) = window.show() {
            tracing::error!("[tray] 显示进程监视器窗口失败: {}", e);
        }
        if let Err(e) = window.set_focus() {
            tracing::error!("[tray] 聚焦进程监视器窗口失败: {}", e);
        }
        return;
    }

    tracing::info!("[tray] 创建新的进程监视器窗口");
    
    // 创建新窗口 - 使用正确的构建路径
    let result = tauri::WebviewWindowBuilder::new(
        app,
        label,
        tauri::WebviewUrl::App("src/process-monitor/index.html".into()),
    )
    .title("进程监视器")
    .inner_size(600.0, 400.0)
    .closable(true)
    .build();

    match result {
        Ok(_) => tracing::info!("[tray] 进程监视器窗口创建成功"),
        Err(e) => tracing::error!("[tray] 进程监视器窗口创建失败: {}", e),
    }
}

/// 打开日志查看器窗口
fn open_log_viewer_window<R: Runtime>(app: &AppHandle<R>) {
    let label = "log_viewer";
    tracing::info!("[tray] 开始打开日志查看器窗口, label={}", label);
    
    // 如果窗口已存在，聚焦它
    if let Some(window) = app.get_webview_window(label) {
        tracing::info!("[tray] 日志查看器窗口已存在，正在聚焦");
        if let Err(e) = window.show() {
            tracing::error!("[tray] 显示日志查看器窗口失败: {}", e);
        }
        if let Err(e) = window.set_focus() {
            tracing::error!("[tray] 聚焦日志查看器窗口失败: {}", e);
        }
        return;
    }

    tracing::info!("[tray] 创建新的日志查看器窗口");
    
    // 创建新窗口 - 使用正确的构建路径
    let result = tauri::WebviewWindowBuilder::new(
        app,
        label,
        tauri::WebviewUrl::App("src/log-viewer/index.html".into()),
    )
    .title("日志查看器")
    .inner_size(800.0, 600.0)
    .closable(true)
    .build();

    match result {
        Ok(_) => tracing::info!("[tray] 日志查看器窗口创建成功"),
        Err(e) => tracing::error!("[tray] 日志查看器窗口创建失败: {}", e),
    }
}