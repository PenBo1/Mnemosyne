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
    // 构建托盘菜单
    let show_item = MenuItemBuilder::with_id("打开 Mnemosyne", MENU_SHOW).build(app)?;
    let about_item = MenuItemBuilder::with_id("关于 Mnemosyne", MENU_ABOUT).build(app)?;
    let process_item = MenuItemBuilder::with_id("进程监视器", MENU_PROCESS_MONITOR).build(app)?;
    let log_item = MenuItemBuilder::with_id("查看日志", MENU_LOG_VIEWER).build(app)?;
    let quit_item = MenuItemBuilder::with_id("退出", MENU_QUIT).build(app)?;

    let menu = MenuBuilder::new(app)
        .item(&show_item)
        .separator()
        .item(&about_item)
        .item(&process_item)
        .item(&log_item)
        .separator()
        .item(&quit_item)
        .build()?;

    // 构建托盘图标
    let tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .on_menu_event(|app, event| {
            match event.id.as_ref() {
                MENU_SHOW => {
                    // 显示主窗口
                    if let Some(window) = app.get_webview_window("main") {
                        window.show().ok();
                        window.set_focus().ok();
                    }
                }
                MENU_ABOUT => {
                    // 打开关于窗口
                    open_about_window(app);
                }
                MENU_PROCESS_MONITOR => {
                    // 打开进程监视器窗口
                    open_process_monitor_window(app);
                }
                MENU_LOG_VIEWER => {
                    // 打开日志查看器窗口
                    open_log_viewer_window(app);
                }
                MENU_QUIT => {
                    // 触发关闭令牌，优雅停止后台任务
                    if let Some(shutdown_token) = app.try_state::<ShutdownToken>() {
                        shutdown_token.0.cancel();
                    }
                    // 完全退出应用
                    app.exit(0);
                }
                _ => {}
            }
        })
        .build(app)?;

    Ok(tray)
}

// ── 窗口打开函数 ────────────────────────────────────────────────────────────

/// 打开关于窗口
fn open_about_window<R: Runtime>(app: &AppHandle<R>) {
    let label = "about";
    
    // 如果窗口已存在，聚焦它
    if let Some(window) = app.get_webview_window(label) {
        window.show().ok();
        window.set_focus().ok();
        return;
    }

    // 创建新窗口
    let _ = tauri::WebviewWindowBuilder::new(
        app,
        label,
        tauri::WebviewUrl::App("about".into()),
    )
    .title("关于 Mnemosyne")
    .inner_size(400.0, 300.0)
    .resizable(false)
    .maximizable(false)
    .build();
}

/// 打开进程监视器窗口
fn open_process_monitor_window<R: Runtime>(app: &AppHandle<R>) {
    let label = "process_monitor";
    
    // 如果窗口已存在，聚焦它
    if let Some(window) = app.get_webview_window(label) {
        window.show().ok();
        window.set_focus().ok();
        return;
    }

    // 创建新窗口
    let _ = tauri::WebviewWindowBuilder::new(
        app,
        label,
        tauri::WebviewUrl::App("process-monitor".into()),
    )
    .title("进程监视器")
    .inner_size(600.0, 400.0)
    .build();
}

/// 打开日志查看器窗口
fn open_log_viewer_window<R: Runtime>(app: &AppHandle<R>) {
    let label = "log_viewer";
    
    // 如果窗口已存在，聚焦它
    if let Some(window) = app.get_webview_window(label) {
        window.show().ok();
        window.set_focus().ok();
        return;
    }

    // 创建新窗口
    let _ = tauri::WebviewWindowBuilder::new(
        app,
        label,
        tauri::WebviewUrl::App("log-viewer".into()),
    )
    .title("日志查看器")
    .inner_size(800.0, 600.0)
    .build();
}