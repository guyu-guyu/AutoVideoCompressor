#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use app::AppCore;
use autovideocompressor_lib::{app, commands, logger, windows_task_scheduler};
use autovideocompressor_lib::config::directory_config::DirectoryConfig;
use fs4::FileExt;
use std::fs::{File, OpenOptions};
use std::sync::Arc;
use tauri::{Emitter, Manager, tray::TrayIconBuilder, menu::{Menu, MenuItem}};

// ======================================================================
// 单例锁：通过文件排他锁确保只有一个实例活跃
// ======================================================================

/// 尝试获取单例锁。成功返回 Some(File)（锁持有时进程退出才释放），
/// 失败返回 None（另一实例已持有）。
fn try_acquire_singleton() -> Option<File> {
    let path = autovideocompressor_lib::app::singleton_lock_path();
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(&path)
        .ok()?;
    if file.try_lock_exclusive().is_ok() {
        Some(file)
    } else {
        None
    }
}

// ======================================================================
// IPC：向已运行的 GUI 实例写入计划压缩请求
// ======================================================================

/// 向主实例写入一条计划压缩请求，写完后立即退出。
fn request_compression(dir: &str) -> Result<(), String> {
    let path = autovideocompressor_lib::util::fs_util::config_base_dir().join("pending");
    std::fs::create_dir_all(&path)
        .map_err(|e| format!("无法创建 IPC 目录: {e}"))?;
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let filename = format!("{ts}.pending");
    let temp_filename = format!("{ts}.pending.tmp");
    let content = serde_json::json!({"dir": dir}).to_string();
    let temp_path = path.join(temp_filename);
    std::fs::write(&temp_path, &content)
        .map_err(|e| format!("无法写入 IPC 请求: {e}"))?;
    std::fs::rename(&temp_path, path.join(filename))
        .map_err(|e| format!("无法提交 IPC 请求: {e}"))?;
    Ok(())
}

// ======================================================================
// 目录有效性检查 & 清理
// ======================================================================

/// 验证目录是否有效。如果无效则清理对应的 Windows 计划任务，
/// 返回 true 表示可以继续压缩。
fn validate_and_run_or_cleanup(core: &Arc<AppCore>, dir: &str) -> bool {
    // 1. 目录是否在磁盘上存在
    if !std::path::Path::new(dir).exists() {
        eprintln!("[main] 目录 '{dir}' 不存在，清理 Windows 计划任务");
        cleanup_schedule_task(dir);
        return false;
    }
    // 2. 配置文件是否有效
    let cfg = DirectoryConfig::load(dir);
    if !cfg.valid {
        eprintln!("[main] 目录 '{dir}' 配置无效，清理 Windows 计划任务");
        cleanup_schedule_task(dir);
        return false;
    }
    // 有效 → 执行压缩
    core.execute_for_directory(dir, false);
    true
}

/// 安全清理 Windows 计划任务（任务不存在时自动忽略）。
fn cleanup_schedule_task(dir: &str) {
    match windows_task_scheduler::WindowsTaskScheduler::new().remove_task(dir) {
        Ok(()) => eprintln!("[main] 已清理 Windows 计划任务: {dir}"),
        Err(e) => eprintln!("[main] 清理 Windows 计划任务失败: {e}"),
    }
}

// ======================================================================
// run_once 逻辑
// ======================================================================

/// 对指定目录（或全部已启用目录）执行一轮压缩。
/// 此函数假定调用方已获得单例锁。
fn run_once(core: &Arc<AppCore>, dir_filter: Option<&str>) {
    match dir_filter {
        Some(dir) => {
            validate_and_run_or_cleanup(core, dir);
        }
        None => {
            let dirs = core.config.lock().unwrap().directories.clone();
            for d in &dirs {
                if d.enabled {
                    validate_and_run_or_cleanup(core, &d.path);
                }
            }
        }
    }
}

// ======================================================================
// --run-once 入口
// ======================================================================

/// --run-once [--directory <path>] 处理流程：
///   1. 尝试获取单例锁
///   2. 获取成功 → 执行本次压缩 → 退出
///   3. 获取失败 → 将指定目录交给 GUI 主实例 → 退出
fn handle_run_once(core: &Arc<AppCore>, args: &[String]) {
    let dir_filter = args.windows(2)
        .find(|w| w[0] == "--directory")
        .map(|w| w[1].as_str());

    if let Some(_lock) = try_acquire_singleton() {
        // Headless 模式只执行本次命令指定的范围，不处理计划任务队列。
        run_once(core, dir_filter);
    } else {
        // 主实例正在运行，通过 IPC 委托
        if let Some(dir) = dir_filter {
            eprintln!("[main] 主实例正在运行，通过 IPC 请求压缩: {dir}");
            if let Err(e) = request_compression(dir) {
                eprintln!("[main] IPC 请求失败: {e}");
            }
        }
        // 无 --directory 时（旧版全目录压缩），主实例的调度器已处理，静默退出
    }
}

// ======================================================================
// main
// ======================================================================

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let core = AppCore::new();

    let scheduled_dir = if args.iter().any(|arg| arg == "--scheduled") {
        match args
            .windows(2)
            .find(|pair| pair[0] == "--directory")
            .map(|pair| pair[1].clone())
        {
            Some(dir) if !dir.trim().is_empty() => Some(dir),
            _ => {
                eprintln!("[main] --scheduled 必须指定 --directory <path>");
                return;
            }
        }
    } else {
        None
    };

    // --run-once [--directory <path>]: 手动 headless 单次压缩。
    if args.iter().any(|a| a == "--run-once") {
        handle_run_once(&core, &args);
        return;
    }

    // 交互模式：获取单例锁，防止重复启动。
    let _singleton = match try_acquire_singleton() {
        Some(l) => l,
        None => {
            if let Some(dir) = scheduled_dir {
                eprintln!("[main] GUI 实例正在运行，提交计划压缩请求: {dir}");
                if let Err(error) = request_compression(&dir) {
                    eprintln!("[main] 计划压缩请求失败: {error}");
                }
            } else {
                eprintln!("[main] 已有实例正在运行，退出");
            }
            return;
        }
    };

    if let Some(dir) = scheduled_dir {
        core.queue_scheduled_directory(dir);
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
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
        .manage(Arc::clone(&core))
        .invoke_handler(tauri::generate_handler![
            commands::frontend_ready,
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

            // 启动 IPC 监控（接收其他计划任务进程提交的目录请求）。
            core_for_setup.start_pending_jobs_monitor();

            // 调度后端选择：Windows 计划任务或 inprocess（默认）。
            let (use_task_scheduler, wake_to_run) = {
                let config = core_for_setup.config.lock().unwrap();
                (
                    config.use_windows_task_scheduler,
                    config.wake_computer_for_scheduled_tasks,
                )
            };
            if use_task_scheduler {
                // 将目录级调度同步为 Windows 计划任务，
                // 由 Windows Task Scheduler 触发 exe --scheduled --directory <path>。
                // 应用本身不启动轮询调度器，避免双重触发。
                eprintln!("[main] 使用 Windows 计划任务调度后端");
                let task_scheduler = windows_task_scheduler::WindowsTaskScheduler::new();
                let dirs = core_for_setup.config.lock().unwrap().directories.clone();
                task_scheduler.sync_all(&dirs, wake_to_run);
                core_for_setup.refresh_schedule_table();
            } else {
                // 默认 inprocess 模式：应用内轮询调度。
                let core_cb = Arc::clone(&core_for_setup);
                core_for_setup.scheduler.start(move |dir| {
                    core_cb.start_for_directory(dir, true);
                });
                core_for_setup.refresh_schedule_table();
            }
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
