use crate::app::AppCore;
use crate::config::directory_config::DirectoryConfig;
use crate::error::{AppError, AppResult};
use crate::logger;
use crate::scanner;
use crate::types::*;
use crate::windows_task_scheduler::WindowsTaskScheduler;
use std::sync::Arc;
use tauri::State;

type Core<'a> = State<'a, Arc<AppCore>>;

#[tauri::command]
pub fn frontend_ready(core: Core) {
    core.mark_frontend_ready();
}

#[tauri::command]
pub fn list_directories(core: Core) -> Vec<DirCardInfo> {
    core.build_card_infos()
}

#[tauri::command]
pub fn get_global_config(core: Core) -> serde_json::Value {
    let c = core.config.lock().unwrap();
    serde_json::json!({
        "ffmpegPath": c.ffmpeg_path,
        "ffmpegTimeoutSeconds": c.ffmpeg_timeout_seconds,
        "minimizeToTray": c.minimize_to_tray,
        "startWithWindows": c.start_with_windows,
        "logRetentionDays": c.log_retention_days,
        "language": c.language,
        "useWindowsTaskScheduler": c.use_windows_task_scheduler,
        "wakeComputerForScheduledTasks": c.wake_computer_for_scheduled_tasks,
        "templates": c.templates.iter().map(|(n,p)|
            serde_json::json!({"name": n, "params": p})).collect::<Vec<_>>(),
    })
}

#[tauri::command]
pub fn save_global_config(core: Core, config: serde_json::Value) -> AppResult<()> {
    let old_use_task_scheduler: bool;
    let new_use_task_scheduler: bool;
    let old_wake_computer: bool;
    let new_wake_computer: bool;
    {
        let mut c = core.config.lock().unwrap();
        old_use_task_scheduler = c.use_windows_task_scheduler;
        old_wake_computer = c.wake_computer_for_scheduled_tasks;
        if let Some(v) = config.get("ffmpegPath").and_then(|x| x.as_str()) { c.ffmpeg_path = v.into(); }
        if let Some(v) = config.get("ffmpegTimeoutSeconds").and_then(|x| x.as_i64()) { c.ffmpeg_timeout_seconds = v; }
        if let Some(v) = config.get("minimizeToTray").and_then(|x| x.as_bool()) { c.minimize_to_tray = v; }
        if let Some(v) = config.get("startWithWindows").and_then(|x| x.as_bool()) { c.start_with_windows = v; }
        if let Some(v) = config.get("logRetentionDays").and_then(|x| x.as_i64()) { c.log_retention_days = v; }
        if let Some(v) = config.get("language").and_then(|x| x.as_str()) { c.language = v.into(); }
        if let Some(v) = config.get("useWindowsTaskScheduler").and_then(|x| x.as_bool()) { c.use_windows_task_scheduler = v; }
        if let Some(v) = config.get("wakeComputerForScheduledTasks").and_then(|x| x.as_bool()) { c.wake_computer_for_scheduled_tasks = v; }
        new_use_task_scheduler = c.use_windows_task_scheduler;
        new_wake_computer = c.wake_computer_for_scheduled_tasks;
        if let Some(arr) = config.get("templates").and_then(|x| x.as_array()) {
            c.templates = arr.iter().filter_map(|t| {
                let n = t.get("name").and_then(|x| x.as_str())?.to_string();
                let p = t.get("params").and_then(|x| x.as_str()).unwrap_or("").to_string();
                if n.is_empty() { None } else { Some((n, p)) }
            }).collect();
        }
        if !c.save() { return Err(AppError::new("保存全局配置失败")); }
        core.template_manager.lock().unwrap().set_templates(c.templates.clone());
    }

    // 切换调度后端时，需要清理或同步所有 Windows 计划任务。
    if old_use_task_scheduler != new_use_task_scheduler || old_wake_computer != new_wake_computer {
        let task_scheduler = WindowsTaskScheduler::new();
        let directories = core.config.lock().unwrap().directories.clone();
        if new_use_task_scheduler {
            task_scheduler.sync_all(&directories, new_wake_computer);
        } else if old_use_task_scheduler {
            task_scheduler.remove_all(&directories);
        }
    }

    core.refresh_schedule_table();
    Ok(())
}

#[tauri::command]
pub fn add_directory(core: Core, path: String) -> AppResult<()> {
    let (sync_task, wake_to_run) = {
        let mut c = core.config.lock().unwrap();
        c.add_directory(&path);
        if !c.save() { return Err(AppError::new("保存配置失败")); }
        (c.use_windows_task_scheduler, c.wake_computer_for_scheduled_tasks)
    };
    if sync_task {
        let cfg = DirectoryConfig::load(&path);
        WindowsTaskScheduler::new().sync_directory(&path, cfg.schedule_time.as_deref(), true, wake_to_run)
            .map_err(|e| AppError::new(&e))?;
    }
    core.refresh_schedule_table();
    Ok(())
}

#[tauri::command]
pub fn remove_directory(core: Core, path: String, force: bool) -> AppResult<()> {
    if core.compressor_running.load(std::sync::atomic::Ordering::SeqCst) && !force {
        return Err(AppError::new("正在压缩,需确认强制移除"));
    }
    let sync_task = {
        let mut c = core.config.lock().unwrap();
        if let Some(i) = c.directories.iter().position(|d| d.path == path) {
            c.remove_directory(i);
        }
        if !c.save() { return Err(AppError::new("保存配置失败")); }
        c.use_windows_task_scheduler
    };
    if sync_task {
        let _ = WindowsTaskScheduler::new().remove_task(&path);
    }
    core.refresh_schedule_table();
    Ok(())
}

#[tauri::command]
pub fn set_directory_enabled(core: Core, path: String, enabled: bool) -> AppResult<()> {
    let (sync_task, wake_to_run) = {
        let mut c = core.config.lock().unwrap();
        if let Some(i) = c.directories.iter().position(|d| d.path == path) {
            c.set_enabled(i, enabled);
        }
        if !c.save() { return Err(AppError::new("保存配置失败")); }
        (c.use_windows_task_scheduler, c.wake_computer_for_scheduled_tasks)
    };
    if sync_task {
        let cfg = DirectoryConfig::load(&path);
        WindowsTaskScheduler::new().sync_directory(&path, cfg.schedule_time.as_deref(), enabled, wake_to_run)
            .map_err(|e| AppError::new(&e))?;
    }
    core.refresh_schedule_table();
    Ok(())
}

fn to_view(cfg: &DirectoryConfig, exists: bool) -> DirConfigView {
    DirConfigView {
        exists,
        valid: cfg.valid,
        error_message: cfg.error_message.clone(),
        include: cfg.include_patterns.clone(),
        exclude: cfg.exclude_patterns.clone(),
        max_size_mb: cfg.max_size_bytes.map(|b| b as f64 / (1024.0*1024.0)),
        min_size_mb: cfg.min_size_bytes.map(|b| b as f64 / (1024.0*1024.0)),
        max_compress_size_mb: cfg.max_compress_size_bytes.map(|b| b as f64 / (1024.0*1024.0)),
        mtime_after: cfg.mtime_after.clone(),
        mtime_before: cfg.mtime_before.clone(),
        ctime_after: cfg.ctime_after.clone(),
        ctime_before: cfg.ctime_before.clone(),
        rename_rules: cfg.rename_rules.iter().map(|(p,r)|
            RenameRuleView { pattern: p.clone(), replacement: r.clone() }).collect(),
        params: cfg.params.clone(),
        use_custom_params: cfg.use_custom_params,
        schedule_time: cfg.schedule_time.clone(),
    }
}

#[tauri::command]
pub fn get_directory_config(core: Core, path: String) -> DirConfigView {
    let exists = DirectoryConfig::config_path(&path).exists();
    if exists {
        to_view(&DirectoryConfig::load(&path), true)
    } else {
        // default template view, not written to disk yet
        let first_template = core.config.lock().unwrap().templates.first().map(|t| t.0.clone());
        let cfg = DirectoryConfig {
            directory_path: path,
            include_patterns: vec!["*.mp4".into(), "*.mov".into(), "*.avi".into(), "*.mkv".into()],
            exclude_patterns: vec!["*[compress]*".into()],
            rename_rules: vec![("^(.+)(\\.[^.]+)$".into(), "$1[compress]$2".into())],
            params: first_template.unwrap_or_default(),
            ..Default::default()
        };
        to_view(&cfg, false)
    }
}

fn apply_view(path: &str, view: &DirConfigView) -> DirectoryConfig {
    DirectoryConfig {
        directory_path: path.to_string(),
        valid: true,
        include_patterns: view.include.clone(),
        exclude_patterns: view.exclude.clone(),
        max_size_bytes: view.max_size_mb.map(|m| (m * 1024.0 * 1024.0) as u64),
        min_size_bytes: view.min_size_mb.map(|m| (m * 1024.0 * 1024.0) as u64),
        max_compress_size_bytes: view.max_compress_size_mb.map(|m| (m * 1024.0 * 1024.0) as u64),
        mtime_after: view.mtime_after.clone(),
        mtime_before: view.mtime_before.clone(),
        ctime_after: view.ctime_after.clone(),
        ctime_before: view.ctime_before.clone(),
        rename_rules: view.rename_rules.iter().map(|r| (r.pattern.clone(), r.replacement.clone())).collect(),
        params: view.params.clone(),
        use_custom_params: view.use_custom_params,
        schedule_time: view.schedule_time.clone(),
        ..Default::default()
    }
}

#[tauri::command]
pub fn save_directory_config(core: Core, path: String, config: DirConfigView) -> AppResult<()> {
    if config.include.iter().all(|s| s.trim().is_empty()) {
        return Err(AppError::new("白名单(include)不能为空"));
    }
    let cfg = apply_view(&path, &config);
    if !cfg.save() { return Err(AppError::new("写入配置文件失败")); }
    let (sync_task, wake_to_run, directory_enabled) = {
        let config = core.config.lock().unwrap();
        (
            config.use_windows_task_scheduler,
            config.wake_computer_for_scheduled_tasks,
            config
                .directories
                .iter()
                .find(|directory| directory.path == path)
                .map(|directory| directory.enabled)
                .unwrap_or(false),
        )
    };
    if sync_task {
        WindowsTaskScheduler::new().sync_directory(
            &path,
            cfg.schedule_time.as_deref(),
            directory_enabled,
            wake_to_run,
        )
            .map_err(|e| AppError::new(&e))?;
    }
    core.refresh_schedule_table();
    Ok(())
}

#[tauri::command]
pub fn create_directory_config(core: Core, path: String, config: DirConfigView) -> AppResult<()> {
    save_directory_config(core, path, config)
}

#[tauri::command]
pub fn get_config_mtime(path: String) -> u64 {
    let p = DirectoryConfig::config_path(&path);
    std::fs::metadata(&p)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[tauri::command]
pub fn open_config_in_editor(app: tauri::AppHandle, path: String) -> AppResult<()> {
    use tauri_plugin_opener::OpenerExt;
    let p = DirectoryConfig::config_path(&path);
    app.opener().open_path(p.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| AppError::new(e.to_string()))
}

#[tauri::command]
pub fn scan_directory(path: String) -> Vec<FilePreview> {
    let cfg = DirectoryConfig::load(&path);
    let all = scanner::scan(&cfg);
    let (in_run, skipped) = scanner::compute_next_run_set(all, cfg.max_compress_size_bytes);
    let mut out: Vec<FilePreview> = in_run.into_iter().map(|f| FilePreview {
        relative_path: f.relative_path,
        final_name: f.final_name,
        file_size: f.file_size,
        cycle_risk: f.cycle_risk,
        in_next_run: true,
    }).collect();
    // Add skipped files too, with in_next_run = false
    for f in skipped {
        out.push(FilePreview {
            relative_path: f.relative_path,
            final_name: f.final_name,
            file_size: f.file_size,
            cycle_risk: f.cycle_risk,
            in_next_run: false,
        });
    }
    out
}

#[tauri::command]
pub fn list_run_history(path: String) -> Vec<RunSummary> {
    logger::read_history(&path)
}

#[tauri::command]
pub fn compress_directory_now(core: Core, path: String) -> AppResult<()> {
    let core_arc = core.inner().clone();
    if core_arc.start_for_directory(path, false) {
        Ok(())
    } else {
        Err(AppError::new("已有目录正在压缩,请稍后"))
    }
}

#[tauri::command]
pub fn stop_compression(core: Core) -> AppResult<()> {
    let core_arc = core.inner().clone();
    core_arc.stop_compression();
    Ok(())
}

#[tauri::command]
pub fn recheck_ffmpeg(core: Core) -> AppResult<()> {
    let core_arc = core.inner().clone();
    core_arc.check_ffmpeg_async();
    Ok(())
}

#[tauri::command]
pub fn get_ffmpeg_status(core: Core) -> FfmpegStatus {
    core.ffmpeg_status.lock().unwrap().clone()
}
