#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use autocompress_lib::{app, commands, logger};
use app::AppCore;
use std::sync::Arc;
use tauri::{Emitter, Manager, tray::TrayIconBuilder, menu::{Menu, MenuItem}};

fn run_once(core: &Arc<AppCore>) {
    let dirs = core.config.lock().unwrap().directories.clone();
    for d in dirs {
        if d.enabled {
            core.execute_for_directory(&d.path, false);
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let core = AppCore::new();

    // --run-once: headless single cycle for Task Scheduler, then exit.
    if args.iter().any(|a| a == "--run-once") {
        run_once(&core);
        return;
    }

    // Startup: per-directory log cleanup using global retention.
    {
        let retention = core.config.lock().unwrap().log_retention_days;
        let dirs = core.config.lock().unwrap().directories.clone();
        for d in dirs {
            logger::Logger::new(&d.path).clean_old_logs(retention);
        }
    }

    let core_for_setup = Arc::clone(&core);
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
        .manage(Arc::clone(&core))
        .invoke_handler(tauri::generate_handler![
            commands::list_directories,
            commands::get_global_config,
            commands::save_global_config,
            commands::add_directory,
            commands::remove_directory,
            commands::set_directory_enabled,
            commands::get_directory_config,
            commands::save_directory_config,
            commands::create_directory_config,
            commands::get_config_mtime,
            commands::open_config_in_editor,
            commands::scan_directory,
            commands::list_run_history,
            commands::compress_directory_now,
            commands::stop_compression,
            commands::recheck_ffmpeg,
            commands::get_ffmpeg_status,
        ])
        .setup(move |app| {
            let handle = app.handle().clone();
            core_for_setup.set_app_handle(handle.clone());

            // Scheduler: trigger a single directory (scheduled → advance).
            let core_cb = Arc::clone(&core_for_setup);
            core_for_setup.scheduler.start(move |dir| {
                core_cb.start_for_directory(dir, true);
            });
            core_for_setup.refresh_schedule_table();
            core_for_setup.check_ffmpeg_async();

            // Tray
            let show = MenuItem::with_id(app, "show", "显示主窗口", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(move |app, event| match event.id().as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "quit" => { app.exit(0); }
                    _ => {}
                })
                .build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let core: tauri::State<Arc<AppCore>> = window.state();
                if core.compressor_running.load(std::sync::atomic::Ordering::SeqCst) {
                    // Ask the frontend to confirm; prevent immediate close.
                    api.prevent_close();
                    let _ = window.emit("close-requested-while-compressing", ());
                } else if core.config.lock().unwrap().minimize_to_tray {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
