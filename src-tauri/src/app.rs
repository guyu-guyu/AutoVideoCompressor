use crate::compressor::{engine, file_compare};
use crate::config::directory_config::DirectoryConfig;
use crate::config::global_config::GlobalConfig;
use crate::config::template_manager::TemplateManager;
use crate::logger::{Logger, now_for_filename};
use crate::scanner;
use crate::scheduler::{compute_next_run, DirSchedule, Scheduler};
use crate::types::*;
use crate::util::fs_util::{config_base_dir, has_enough_space, safe_delete_retry};
use crate::util::string_util::format_file_size;
use chrono::{Datelike, Local, Timelike};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

/// IPC pending jobs directory path.
fn pending_jobs_dir() -> std::path::PathBuf {
    config_base_dir().join("pending")
}

/// Singleton lock file path.
pub fn singleton_lock_path() -> std::path::PathBuf {
    config_base_dir().join("singleton.lock")
}

pub struct AppCore {
    pub config: Mutex<GlobalConfig>,
    pub scheduler: Scheduler,
    pub template_manager: Mutex<TemplateManager>,
    pub compressor_running: AtomicBool,
    pub cancel_flag: AtomicBool,
    pub ffmpeg_status: Mutex<FfmpegStatus>,
    pub last_runs: Mutex<std::collections::HashMap<String, (String, String)>>,
    pub app_handle: Mutex<Option<AppHandle>>,
}

/// Compute the status badge for a directory. Pure given (dir, overlap flag).
/// Returns (badge, detail). badge ∈ {valid, unscheduled, invalid, config_error, overlap}.
pub fn compute_badge(dir: &str, overlap: bool) -> (String, String) {
    if overlap {
        return ("overlap".into(), String::new());
    }
    let cfg = DirectoryConfig::load(dir);
    if !cfg.valid {
        if cfg.error_message.contains("not found") {
            return ("invalid".into(), String::new());
        }
        return ("config_error".into(), cfg.error_message);
    }
    match &cfg.schedule_time {
        Some(t) if !t.is_empty() => ("valid".into(), String::new()),
        _ => ("unscheduled".into(), String::new()),
    }
}

/// Build a FileResult for a file that was NOT compressed (skipped or failed
/// before ffmpeg ran). Original file is untouched, so `final_*` == original.
fn make_untouched_result(sf: &scanner::ScanFile, status: FileStatus, error_message: &str) -> FileResult {
    FileResult {
        name: sf.relative_path.clone(),
        path: sf.absolute_path.clone(),
        final_name: sf.relative_path.clone(),
        final_path: sf.absolute_path.clone(),
        original_size: sf.file_size,
        status,
        error_message: error_message.to_string(),
        ..Default::default()
    }
}

impl AppCore {
    pub fn new() -> Arc<Self> {
        let config = GlobalConfig::load();
        let mut tm = TemplateManager::new();
        tm.set_templates(config.templates.clone());
        Arc::new(AppCore {
            config: Mutex::new(config),
            scheduler: Scheduler::new(),
            template_manager: Mutex::new(tm),
            compressor_running: AtomicBool::new(false),
            cancel_flag: AtomicBool::new(false),
            ffmpeg_status: Mutex::new(FfmpegStatus::default()),
            last_runs: Mutex::new(std::collections::HashMap::new()),
            app_handle: Mutex::new(None),
        })
    }

    pub fn set_app_handle(&self, handle: AppHandle) {
        *self.app_handle.lock().unwrap() = Some(handle);
    }

    fn emit<S: serde::Serialize + Clone>(&self, event: &str, payload: S) {
        if let Some(h) = self.app_handle.lock().unwrap().as_ref() {
            let _ = h.emit(event, payload);
        }
    }

    /// Rebuild the scheduler timetable from per-directory configs. Mirrors refreshScheduleTable.
    pub fn refresh_schedule_table(&self) {
        let dirs = self.config.lock().unwrap().directories.clone();
        let mut schedules = Vec::new();
        for d in &dirs {
            let cfg = DirectoryConfig::load(&d.path);
            let (enabled, next_run) = match (&cfg.valid, &cfg.schedule_time) {
                (true, Some(t)) if !t.is_empty() => {
                    let parts: Vec<&str> = t.split(':').collect();
                    if parts.len() == 2 {
                        if let (Ok(hh), Ok(mm)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                            (d.enabled, compute_next_run(Local::now(), hh, mm))
                        } else {
                            (false, Local::now())
                        }
                    } else {
                        (false, Local::now())
                    }
                }
                _ => (false, Local::now()),
            };
            schedules.push(DirSchedule { dir_path: d.path.clone(), enabled, next_run });
        }
        self.scheduler.set_directories(schedules);
    }

    /// Human-readable next-run label. Mirrors App::formatNextRun.
    pub fn format_next_run(&self, dir: &str) -> String {
        match self.scheduler.next_run_time(dir) {
            None => "未配置".into(),
            Some(next) => {
                let now = Local::now();
                if next.year() == now.year() && next.ordinal() == now.ordinal() {
                    format!("今天 {:02}:{:02}", next.hour(), next.minute())
                } else if next.year() == now.year() && next.ordinal() == now.ordinal() + 1 {
                    format!("明天 {:02}:{:02}", next.hour(), next.minute())
                } else {
                    format!("{:02}-{:02} {:02}:{:02}",
                        next.month(), next.day(), next.hour(), next.minute())
                }
            }
        }
    }

    /// Aggregate card info for the level-1 list. Used by list_directories command.
    pub fn build_card_infos(&self) -> Vec<DirCardInfo> {
        let (dirs, overlaps) = {
            let c = self.config.lock().unwrap();
            (c.directories.clone(), c.detect_overlaps())
        };
        let mut out = Vec::new();
        for (i, d) in dirs.iter().enumerate() {
            let overlap = overlaps.get(i).copied().unwrap_or(false);
            let (badge, detail) = compute_badge(&d.path, overlap);
            let cfg = DirectoryConfig::load(&d.path);
            let (file_count, total_size, cycle_risk, next_run_count, next_run_size) = if cfg.valid {
                let files = scanner::scan(&cfg);
                let fcount = files.len();
                let size: u64 = files.iter().map(|f| f.file_size).sum();
                let risk = files.iter().filter(|f| f.cycle_risk).count() as i32;
                let (in_run, _) = scanner::compute_next_run_set(files, cfg.max_compress_size_bytes);
                let nr_size: u64 = in_run.iter().map(|f| f.file_size).sum();
                (fcount as i32, size, risk, in_run.len() as i32, nr_size)
            } else {
                (0, 0, 0, 0, 0)
            };
            let (last_time, last_result) = {
                let mut cache = self.last_runs.lock().unwrap();
                if let Some(v) = cache.get(&d.path).cloned() {
                    v
                } else {
                    // Fallback: read from log history (handles app restart)
                    let history = crate::logger::read_history(&d.path);
                    let val = history.first().map(|r| {
                        let size_s = crate::util::string_util::format_file_size(r.total_saved_bytes);
                        let result = format!("成功{}·节省{}", r.success_count, size_s);
                        (r.start_time.clone(), result)
                    }).unwrap_or_default();
                    cache.insert(d.path.clone(), val.clone());
                    val
                }
            };
            out.push(DirCardInfo {
                path: d.path.clone(),
                enabled: d.enabled,
                badge,
                badge_detail: detail,
                file_count,
                total_size,
                params_name: cfg.params.clone(),
                cycle_risk_count: cycle_risk,
                last_run_time: last_time,
                last_run_result: last_result,
                next_run_time: self.format_next_run(&d.path),
                next_run_count,
                next_run_size,
            });
        }
        out
    }

    /// Launch the per-directory pipeline on a background thread (serial-locked).
    /// Mirrors App::startForDirectory. Returns false if busy.
    pub fn start_for_directory(self: &Arc<Self>, dir: String, advance: bool) -> bool {
        if self.compressor_running.swap(true, Ordering::SeqCst) {
            return false;
        }
        // Reset cancel flag for this run
        self.cancel_flag.store(false, Ordering::SeqCst);
        let me = Arc::clone(self);
        std::thread::spawn(move || {
            me.execute_for_directory(&dir, advance);
            me.compressor_running.store(false, Ordering::SeqCst);
            // Notify frontend that compression stopped
            me.emit_dir_state(&dir, Stage::Idle, "就绪");
        });
        true
    }

    /// Stop an ongoing compression. Kills ffmpeg and cleans up temp files.
    pub fn stop_compression(&self) {
        self.cancel_flag.store(true, Ordering::SeqCst);
    }

    /// Core per-directory pipeline. Mirrors App::executeForDirectory.
    pub fn execute_for_directory(&self, dir: &str, advance: bool) {
        let dir_config = DirectoryConfig::load(dir);
        {
            let templates = self.config.lock().unwrap().templates.clone();
            self.template_manager.lock().unwrap().set_templates(templates);
        }
        if !dir_config.valid {
            if advance { self.scheduler.mark_completed(dir); }
            return;
        }
        // overlap check
        let overlap = {
            let c = self.config.lock().unwrap();
            let flags = c.detect_overlaps();
            c.directories.iter().enumerate()
                .find(|(_, d)| d.path == dir)
                .map(|(i, _)| flags.get(i).copied().unwrap_or(false))
                .unwrap_or(false)
        };
        if overlap {
            if advance { self.scheduler.mark_completed(dir); }
            return;
        }

        let dir_params = dir_config.params.clone();
        let matcher = dir_config.matcher.clone();
        let all_files = scanner::scan(&dir_config);
        let (files, _skipped) = scanner::compute_next_run_set(all_files, dir_config.max_compress_size_bytes);

        let (ffmpeg_path, timeout) = {
            let c = self.config.lock().unwrap();
            let p = if c.ffmpeg_path.is_empty() { "ffmpeg".to_string() } else { c.ffmpeg_path.clone() };
            (p, c.ffmpeg_timeout_seconds)
        };

        let mut logger = Logger::new(dir);
        let mut summary = RunSummary {
            run_id: now_for_filename(),
            start_time: crate::util::string_util::now_iso_public(),
            directories: vec![DirectoryResult {
                path: dir.to_string(),
                config_valid: true,
                files_total: files.len() as i32,
                files_processed: 0,
            }],
            ..Default::default()
        };
        logger.begin_run(&summary);

        if !files.is_empty() {
            self.emit_dir_state(dir, Stage::Compressing,
                &format!("压缩中: {}", Path::new(dir).file_name()
                    .and_then(|s| s.to_str()).unwrap_or("")));
        }

        let mut all_results = Vec::new();
        let mut was_cancelled = false;
        // Tracks the single temp file of the file currently being compressed (at most
        // one exists at a time in this serial loop). Used to clean it up on cancel.
        let mut current_temp: Option<std::path::PathBuf>;
        for (fi, sf) in files.iter().enumerate() {
            // Check cancellation before processing each file
            if self.cancel_flag.load(Ordering::SeqCst) {
                was_cancelled = true;
                break;
            }

            self.emit("compress-progress", serde_json::json!({
                "dirPath": dir, "currentFile": sf.relative_path,
                "completed": fi + 1, "total": files.len()
            }));

            let resolved = self.template_manager.lock().unwrap().resolve(&dir_params);
            if resolved.is_empty() {
                // No ffmpeg params resolved (empty template/param name): abort this
                // file and record a failed result with the reason.
                let fail = make_untouched_result(
                    sf,
                    FileStatus::Failed,
                    "未解析到压缩参数（参数模板为空或不存在），跳过该文件",
                );
                logger.log_file_result(&fail);
                all_results.push(fail);
                continue;
            }
            let params = resolved;

            let orig = Path::new(&sf.absolute_path);
            let parent = orig.parent().map(|p| p.to_path_buf()).unwrap_or_default();
            let temp_path = parent.join(&sf.temp_name);
            current_temp = Some(temp_path.clone());

            if !has_enough_space(&parent, sf.file_size / 2) {
                let skip = make_untouched_result(sf, FileStatus::SkippedOther, "");
                logger.log_file_result(&skip);
                all_results.push(skip);
                continue;
            }

            let cparams = engine::CompressParams {
                ffmpeg_path: ffmpeg_path.clone(),
                arguments: params,
                input_path: sf.absolute_path.clone(),
                output_path: temp_path.to_string_lossy().to_string(),
                timeout_seconds: timeout,
            };
            eprintln!(
                "[avc] compressing {}: ffmpeg={}, timeout={}s, args={}",
                sf.relative_path, ffmpeg_path, timeout, cparams.arguments
            );
            let cres = engine::compress(&cparams, &self.cancel_flag);
            eprintln!(
                "[avc] compress result: success={}, exit_code={}, cancelled={}, dur_ms={}, err={}",
                cres.success, cres.exit_code, cres.cancelled, cres.duration_ms, cres.error_message
            );

            // If cancelled during compression, delete the temp file we just created
            // for this file and break. Use a retrying delete: on Windows the OS may
            // briefly hold the file lock after ffmpeg is killed, so a naive
            // remove_file often fails silently (this was the root cause of the
            // "tmp remains after stop" bug).
            if cres.cancelled {
                was_cancelled = true;
                if let Some(p) = current_temp.take() {
                    if !safe_delete_retry(&p, 5) {
                        eprintln!(
                            "[avc] failed to remove temp file after cancel: {}",
                            p.display()
                        );
                    }
                }
                break;
            }

            if !cres.success {
                self.update_ffmpeg_status(FfmpegStatus {
                    ready: false, version: String::new(), error: cres.error_message.clone(),
                });
            }
            let compressed_size = if cres.success && temp_path.exists() {
                std::fs::metadata(&temp_path).map(|m| m.len()).unwrap_or(0)
            } else { 0 };

            // Compute the final (renamed) path from just the filename (not the
            // full relative path) — otherwise parent.join(rename("subdir/a.mp4"))
            // duplicates the directory component.
            let orig_name = orig
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or(&sf.relative_path);
            let final_name = matcher.apply_rename(orig_name);
            let final_path = parent.join(&final_name).to_string_lossy().to_string();

            let mut fr = file_compare::compare_and_cleanup(
                &sf.absolute_path, sf.file_size,
                &temp_path.to_string_lossy(), compressed_size,
                &final_path, cres.exit_code, cres.duration_ms);
            fr.cycle_risk = sf.cycle_risk;
            logger.log_file_result(&fr);
            all_results.push(fr);
        }

        if let Some(d0) = summary.directories.get_mut(0) {
            d0.files_processed = all_results.len() as i32;
        }
        summary.files = all_results;
        summary.end_time = crate::util::string_util::now_iso_public();
        summary.duration_seconds = 0;
        summary.compute_totals();

        if was_cancelled {
            // If cancelled, don't write the finalize log (partial run not saved)
            // But still record what we did
        } else {
            logger.finalize_run(&summary);
        }

        if advance { self.scheduler.mark_completed(dir); }

        // record last-run for cards (only if we did some work)
        if !was_cancelled {
            let result_str = format!("成功{}·节省{}",
                summary.success_count, format_file_size(summary.total_saved_bytes));
            self.last_runs.lock().unwrap()
                .insert(dir.to_string(), (summary.start_time.clone(), result_str));
        } else {
            self.last_runs.lock().unwrap()
                .insert(dir.to_string(), (summary.start_time.clone(), "已停止".into()));
        }

        self.emit_dir_state(dir, Stage::Idle, if was_cancelled { "已停止" } else { "就绪" });
    }

    fn emit_dir_state(&self, dir: &str, stage: Stage, status: &str) {
        let mut st = DirRuntimeState::new(dir.to_string());
        st.stage = stage;
        st.status_text = status.to_string();
        st.next_run_time = self.format_next_run(dir);
        if let Some((t, r)) = self.last_runs.lock().unwrap().get(dir) {
            st.last_run_time = t.clone();
            st.last_run_result = r.clone();
        }
        self.emit("dir-state-changed", st);
    }

    pub fn check_ffmpeg_async(self: &Arc<Self>) {
        let me = Arc::clone(self);
        let path = {
            let c = me.config.lock().unwrap();
            if c.ffmpeg_path.is_empty() { "ffmpeg".to_string() } else { c.ffmpeg_path.clone() }
        };
        std::thread::spawn(move || {
            let status = engine::probe_ffmpeg(&path);
            me.update_ffmpeg_status(status);
        });
    }

    fn update_ffmpeg_status(&self, status: FfmpegStatus) {
        *self.ffmpeg_status.lock().unwrap() = status.clone();
        self.emit("ffmpeg-status-changed", status);
    }

    // ------------------------------------------------------------------
    // IPC: 接收 --run-once 委托的压缩请求
    // ------------------------------------------------------------------

    /// 收集并清理 IPC 请求文件，返回请求压缩的目录列表。
    pub fn collect_pending_requests() -> Vec<String> {
        let dir = pending_jobs_dir();
        if !dir.exists() {
            return Vec::new();
        }
        let entries: Vec<_> = match std::fs::read_dir(&dir) {
            Ok(e) => e.filter_map(|e| e.ok()).collect(),
            Err(_) => return Vec::new(),
        };
        let mut result = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for entry in &entries {
            let path = entry.path();
            if path.extension().map(|e| e == "pending").unwrap_or(false) {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(j) = serde_json::from_str::<serde_json::Value>(&content) {
                        if let Some(d) = j.get("dir").and_then(|d| d.as_str()).map(String::from) {
                            if seen.insert(d.clone()) {
                                result.push(d);
                            }
                        }
                    }
                }
                let _ = std::fs::remove_file(&path);
            }
        }
        result
    }

    /// 启动后台线程，定期检查并处理 IPC 请求。
    pub fn start_pending_jobs_monitor(self: &Arc<Self>) {
        let me = Arc::clone(self);
        std::thread::spawn(move || {
            loop {
                let dirs = AppCore::collect_pending_requests();
                for dir in dirs {
                    if me.cancel_flag.load(Ordering::SeqCst) {
                        // 正在停止，跳过本次请求
                        continue;
                    }
                    let _ = me.start_for_directory(dir, true);
                }
                std::thread::sleep(std::time::Duration::from_secs(2));
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn badge_for_missing_config_is_invalid() {
        let tmp = tempfile::tempdir().unwrap();
        let (badge, _detail) = compute_badge(tmp.path().to_str().unwrap(), false);
        assert_eq!(badge, "invalid");
    }

    #[test]
    fn badge_for_valid_scheduled_and_unscheduled() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_str().unwrap();
        crate::config::directory_config::DirectoryConfig::create_default(dir, "");
        // no schedule → unscheduled
        let (b1, _) = compute_badge(dir, false);
        assert_eq!(b1, "unscheduled");
        // overlap flag → overlap wins
        let (b2, _) = compute_badge(dir, true);
        assert_eq!(b2, "overlap");
    }
}
