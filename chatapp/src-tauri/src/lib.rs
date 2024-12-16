// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

use std::sync::Arc;

use anyhow::Result;
use arc_swap::ArcSwap;
use commands::{get_app_dir, get_config, greet};
use config::AppConfig;
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, SubmenuBuilder},
    tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
    webview::PageLoadPayload,
    App, AppHandle, Builder, Manager, Runtime, Webview, WebviewUrl, WebviewWindowBuilder, Window,
    WindowEvent, Wry,
};
use tauri_plugin_log::{Target, TargetKind};
use tracing::{debug, info};
use utils::log_dir;

mod commands;
mod config;
mod utils;

const APP_NAME: &str = "chatapp";

pub struct AppState {
    config: Arc<ArcSwap<AppConfig>>,
}

// #[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn app() -> Result<Builder<Wry>> {
    let builder = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(logger().build())
        .invoke_handler(tauri::generate_handler![greet, get_app_dir, get_config])
        .setup(setup)
        .on_page_load(page_load_handler)
        .on_window_event(window_event_handler);

    Ok(builder)
}

fn setup(app: &mut App) -> Result<(), Box<dyn std::error::Error>> {
    info!("Setting up app");

    let state = AppState {
        config: Arc::new(ArcSwap::from_pointee(AppConfig::try_new()?)),
    };
    app.manage(state);

    let handle = app.handle();

    #[cfg(desktop)]
    {
        handle.plugin(tauri_plugin_window_state::Builder::default().build())?;
    }

    setup_menu(handle)?;

    let mut builder = WebviewWindowBuilder::new(app, "main", WebviewUrl::default());

    #[cfg(desktop)]
    {
        builder = builder
            .user_agent(&format!("{} - {}", APP_NAME, std::env::consts::OS))
            .title("Chat app")
            .inner_size(1200., 800.)
            .min_inner_size(800., 600.)
            .resizable(true)
            .content_protected(true);
    }

    let webview = builder.build()?;

    #[cfg(debug_assertions)]
    webview.open_devtools();

    Ok(())
}

fn page_load_handler(webview: &Webview, _payload: &PageLoadPayload<'_>) {
    info!("Page loaded: {:?}", webview.label());
}

fn window_event_handler(window: &Window, event: &WindowEvent) {
    debug!("Window event {:?} on {:?}", event, window.label());
    if let WindowEvent::CloseRequested { api, .. } = event {
        info!("Close requested on {:?}", window.label());
        if window.label() == "main" {
            api.prevent_close();
            window.hide().unwrap()
        }
    }
    // match event {
    //     WindowEvent::CloseRequested { api, .. } => {
    //         info!("Close requested on {:?}", window.label());
    //         if window.label() == "main" {
    //             api.prevent_close();
    //             window.hide().unwrap()
    //         }
    //     }
    //     _ => {}
    // }
}

fn logger() -> tauri_plugin_log::Builder {
    tauri_plugin_log::Builder::default()
        .targets([
            Target::new(TargetKind::Webview),
            Target::new(TargetKind::Folder {
                path: log_dir(),
                file_name: None,
            }),
            Target::new(TargetKind::Stdout),
        ])
        .level(tracing::log::LevelFilter::Info)
}

fn setup_menu<R: Runtime>(app: &AppHandle<R>) -> Result<(), tauri::Error> {
    let icon = app.default_window_icon().unwrap().clone();
    // create submenus
    let file_menu = SubmenuBuilder::with_id(app, "file", "File")
        .item(&MenuItem::with_id(
            app,
            "open",
            "Open",
            true,
            Some("CmdOrCtrl+O"),
        )?)
        .item(&MenuItem::with_id(
            app,
            "save",
            "Save",
            true,
            Some("CmdOrCtrl+S"),
        )?)
        .item(&MenuItem::with_id(
            app,
            "save_as",
            "Save As",
            true,
            Some("CmdOrCtrl+Shift+S"),
        )?)
        .separator()
        .quit()
        .build()?;
    let edit_menu = SubmenuBuilder::with_id(app, "edit", "Edit")
        .item(&MenuItem::with_id(
            app,
            "process",
            "Process",
            true,
            Some("CmdOrCtrl+P"),
        )?)
        .separator()
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .separator()
        .select_all()
        .item(&CheckMenuItem::with_id(
            app,
            "check_me",
            "Check Me",
            true,
            true,
            None::<&str>,
        )?)
        .build()?;
    let tray_menu = SubmenuBuilder::with_id(app, "tray", "Tray")
        .item(&MenuItem::with_id(app, "show", "Show", true, None::<&str>)?)
        .item(&MenuItem::with_id(app, "hide", "Hide", true, None::<&str>)?)
        .separator()
        .quit()
        .build()?;

    TrayIconBuilder::with_id(format!("{}-tray", APP_NAME))
        .tooltip("Hacker News")
        .icon(icon)
        .menu(&tray_menu)
        .menu_on_left_click(true)
        .on_tray_icon_event(|tray, event| {
            // info!("Tray icon event: {:?}", event);
            if let TrayIconEvent::Click {
                button: MouseButton::Right,
                ..
            } = event
            {
                open_main(tray.app_handle()).unwrap();
            }
        })
        .build(app)?;
    // create menu
    // add menu to tray
    // add menu to window

    let menu = Menu::with_items(app, &[&file_menu, &edit_menu])?;
    app.set_menu(menu)?;
    app.on_menu_event(|app, event| {
        info!("Menu event: {:?}", event);
        let app_handle = app.clone();
        match event.id.as_ref() {
            "open" => {
                tokio::spawn(async move {
                    open_main(&app_handle).unwrap();
                });
            }
            "save" => {
                // save
            }
            "save_as" => {
                // save as
            }
            "process" => {
                // process
            }
            "check_me" => {
                // toggle check me status and update config and runtime state
                // for runtime state - Arc<Mutex<State>> / ArcSwap
            }
            "show" => {
                tokio::spawn(async move {
                    open_main(&app_handle).unwrap();
                });
            }
            "hide" => {
                app.get_webview_window("main")
                    .unwrap()
                    .hide()
                    .expect("Failed to hide window");
            }
            _ => {}
        }
    });

    Ok(())
}

fn open_main<R: Runtime>(handle: &AppHandle<R>) -> Result<(), tauri::Error> {
    handle
        .get_webview_window("main")
        .ok_or(tauri::Error::WindowNotFound)?
        .show()?;

    Ok(())
}
