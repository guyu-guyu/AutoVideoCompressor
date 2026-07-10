use crate::types::{FileStatus, RunSummary};
use crate::util::string_util::format_file_size;
use chrono::Local;
use serde_json::Value;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

/// Per-directory logger writing <dir>/.autocompress/logs/run_*.log. Mirrors Logger.
pub struct Logger {
    log_dir: PathBuf,
    run_id: String,
    current_log_path: PathBuf,
    pending_lines: Vec<String>,
}

/// ISO8601 local timestamp. Mirrors StringUtils::nowToString.
fn now_iso() -> String {
    Local::now().format("%Y-%m-%dT%H:%M:%S%z").to_string()
}

/// Filename-safe timestamp. Mirrors StringUtils::nowForFilename.
pub fn now_for_filename() -> String {
    Local::now().format("%Y-%m-%d_%H-%M-%S").to_string()
}

impl Logger {
    pub fn new(dir: &str) -> Self {
        Logger {
            log_dir: PathBuf::from(dir).join(".autocompress").join("logs"),
            run_id: String::new(),
            current_log_path: PathBuf::new(),
            pending_lines: Vec::new(),
        }
    }

    pub fn begin_run(&mut self, initial: &RunSummary) -> bool {
        let _ = std::fs::create_dir_all(&self.log_dir);
        self.run_id = now_for_filename();
        self.current_log_path = self.log_dir.join(format!("run_{}.log", self.run_id));
        let mut file = match std::fs::File::create(&self.current_log_path) {
            Ok(f) => f,
            Err(_) => return false,
        };
        let mut header = String::new();
        header.push_str("========================================\n");
        header.push_str("AutoCompress Run Log\n");
        header.push_str(&format!("Start time: {}\n", now_iso()));
        header.push_str("Directories:\n");
        for d in &initial.directories {
            let suffix = if d.config_valid { "" } else { " (无效配置)" };
            header.push_str(&format!("  - {}{}\n", d.path, suffix));
        }
        header.push_str("========================================\n\n");
        file.write_all(header.as_bytes()).is_ok()
    }

    pub fn log_file_result(&mut self, r: &crate::types::FileResult) {
        let line = match r.status {
            FileStatus::Success => format!(
                "✅ {} ({} → {}, 节省 {})",
                r.name, format_file_size(r.original_size as i64),
                format_file_size(r.compressed_size as i64),
                format_file_size(r.saved_bytes)
            ),
            FileStatus::SkippedLarger => format!(
                "⏭ {} 压缩后更大 ({} → {})，已丢弃",
                r.name, format_file_size(r.original_size as i64),
                format_file_size(r.compressed_size as i64)
            ),
            FileStatus::Failed => format!(
                "❌ {} 压缩失败 (退出码: {}){}",
                r.name, r.ffmpeg_exit_code,
                if r.error_message.is_empty() {
                    String::new()
                } else {
                    format!(" 原因: {}", r.error_message)
                }
            ),
            FileStatus::SkippedOther => format!("ℹ {} 跳过", r.name),
        };
        self.pending_lines.push(line);
    }

    pub fn finalize_run(&mut self, summary: &RunSummary) {
        if self.current_log_path.as_os_str().is_empty() {
            return;
        }
        let mut file = match OpenOptions::new().append(true).open(&self.current_log_path) {
            Ok(f) => f,
            Err(_) => return,
        };
        let mut body = String::new();
        for line in &self.pending_lines {
            body.push_str(line);
            body.push('\n');
        }
        self.pending_lines.clear();
        body.push_str("\n--- Summary ---\n");
        body.push_str(&format!("Total files: {}\n", summary.files.len()));
        body.push_str(&format!("Successful: {}\n", summary.success_count));
        body.push_str(&format!("Skipped (larger): {}\n", summary.skipped_larger_count));
        body.push_str(&format!("Failed: {}\n", summary.failed_count));
        body.push_str(&format!("Skipped (other): {}\n", summary.skipped_other_count));
        body.push_str(&format!("Total saved: {}\n", format_file_size(summary.total_saved_bytes)));
        body.push_str(&format!("Cycle risk files: {}\n", summary.cycle_risk_count));
        body.push_str(&format!("Duration: {} seconds\n", summary.duration_seconds));
        body.push_str("\n--- JSON ---\n");
        body.push_str(&summary_to_json_block(&self.run_id, summary));
        body.push_str("\n--- JSON END ---\n");
        let _ = file.write_all(body.as_bytes());
    }

    pub fn clean_old_logs(&self, retention_days: i64) {
        if retention_days <= 0 { return; }
        let entries = match std::fs::read_dir(&self.log_dir) {
            Ok(e) => e,
            Err(_) => return,
        };
        let now = std::time::SystemTime::now();
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if !name.starts_with("run_") || path.extension().and_then(|s| s.to_str()) != Some("log") {
                continue;
            }
            if let Ok(meta) = entry.metadata() {
                if let Ok(modified) = meta.modified() {
                    if let Ok(age) = now.duration_since(modified) {
                        if age.as_secs() as i64 >= retention_days * 24 * 3600 {
                            let _ = std::fs::remove_file(&path);
                        }
                    }
                }
            }
        }
    }

}

/// Machine-readable JSON block. Mirrors Logger::summaryToJsonBlock (snake_case keys).
fn summary_to_json_block(run_id: &str, summary: &RunSummary) -> String {
    let mut j = serde_json::Map::new();
    j.insert("version".into(), Value::from(1));
    j.insert("run_id".into(), Value::from(run_id.to_string()));
    j.insert("start_time".into(), Value::from(summary.start_time.clone()));
    j.insert("end_time".into(), Value::from(summary.end_time.clone()));
    j.insert("duration_seconds".into(), Value::from(summary.duration_seconds));
    j.insert("directory_count".into(), Value::from(summary.directories.len() as i64));

    let mut dirs = serde_json::Map::new();
    for d in &summary.directories {
        let mut dm = serde_json::Map::new();
        dm.insert("path".into(), Value::from(d.path.clone()));
        dm.insert("config_valid".into(), Value::from(d.config_valid));
        dm.insert("files_total".into(), Value::from(d.files_total));
        dm.insert("files_processed".into(), Value::from(d.files_processed));
        dirs.insert(d.path.clone(), Value::Object(dm));
    }
    j.insert("directories".into(), Value::Object(dirs));

    let files: Vec<Value> = summary.files.iter().map(|f| {
        let mut fm = serde_json::Map::new();
        fm.insert("name".into(), Value::from(f.name.clone()));
        fm.insert("path".into(), Value::from(f.path.clone()));
        fm.insert("final_name".into(), Value::from(f.final_name.clone()));
        fm.insert("final_path".into(), Value::from(f.final_path.clone()));
        fm.insert("original_size".into(), Value::from(f.original_size));
        fm.insert("compressed_size".into(), Value::from(f.compressed_size));
        fm.insert("saved_bytes".into(), Value::from(f.saved_bytes));
        fm.insert("cycle_risk".into(), Value::from(f.cycle_risk));
        if !f.error_message.is_empty() {
            fm.insert("error_message".into(), Value::from(f.error_message.clone()));
        }
        let status = match f.status {
            FileStatus::Success => "success",
            FileStatus::SkippedLarger => "skipped_larger",
            FileStatus::Failed => "failed",
            FileStatus::SkippedOther => "skipped_other",
        };
        fm.insert("status".into(), Value::from(status));
        if f.status == FileStatus::Failed {
            fm.insert("ffmpeg_exit_code".into(), Value::from(f.ffmpeg_exit_code));
        }
        Value::Object(fm)
    }).collect();
    j.insert("files".into(), Value::from(files));

    let mut s = serde_json::Map::new();
    s.insert("success".into(), Value::from(summary.success_count));
    s.insert("skipped_larger".into(), Value::from(summary.skipped_larger_count));
    s.insert("failed".into(), Value::from(summary.failed_count));
    s.insert("skipped_other".into(), Value::from(summary.skipped_other_count));
    s.insert("total_saved_bytes".into(), Value::from(summary.total_saved_bytes));
    s.insert("cycle_risk_count".into(), Value::from(summary.cycle_risk_count));
    j.insert("summary".into(), Value::Object(s));

    serde_json::to_string_pretty(&Value::Object(j)).unwrap_or_default()
}

/// Parse all run logs in <dir>, newest first. Extracts each JSON block into a RunSummary.
pub fn read_history(dir: &str) -> Vec<RunSummary> {
    let log_dir = PathBuf::from(dir).join(".autocompress").join("logs");
    let mut names: Vec<PathBuf> = match std::fs::read_dir(&log_dir) {
        Ok(e) => e.filter_map(|x| x.ok()).map(|x| x.path())
            .filter(|p| {
                let n = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
                n.starts_with("run_") && p.extension().and_then(|s| s.to_str()) == Some("log")
            }).collect(),
        Err(_) => return Vec::new(),
    };
    names.sort();
    names.reverse(); // newest first (filename timestamp sorts chronologically)

    let mut out = Vec::new();
    for path in names {
        let text = match std::fs::read_to_string(&path) { Ok(t) => t, Err(_) => continue };
        if let Some(json) = extract_json_block(&text) {
            if let Some(summary) = parse_summary_json(&json) {
                out.push(summary);
            }
        }
    }
    out
}

fn extract_json_block(text: &str) -> Option<String> {
    let start = text.find("--- JSON ---")? + "--- JSON ---".len();
    let end = text[start..].find("--- JSON END ---")? + start;
    Some(text[start..end].trim().to_string())
}

fn parse_summary_json(json: &str) -> Option<RunSummary> {
    let v: Value = serde_json::from_str(json).ok()?;
    let get_str = |obj: &Value, k: &str| obj.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
    let get_i64 = |obj: &Value, k: &str| obj.get(k).and_then(|x| x.as_i64()).unwrap_or(0);
    let get_u64 = |obj: &Value, k: &str| obj.get(k).and_then(|x| x.as_u64()).unwrap_or(0);
    let get_bool = |obj: &Value, k: &str| obj.get(k).and_then(|x| x.as_bool()).unwrap_or(false);

    let mut s = RunSummary {
        run_id: get_str(&v, "run_id"),
        start_time: get_str(&v, "start_time"),
        end_time: get_str(&v, "end_time"),
        duration_seconds: get_i64(&v, "duration_seconds") as i32,
        ..Default::default()
    };
    if let Some(sum) = v.get("summary") {
        s.success_count = get_i64(sum, "success") as i32;
        s.skipped_larger_count = get_i64(sum, "skipped_larger") as i32;
        s.failed_count = get_i64(sum, "failed") as i32;
        s.skipped_other_count = get_i64(sum, "skipped_other") as i32;
        s.total_saved_bytes = get_i64(sum, "total_saved_bytes");
        s.cycle_risk_count = get_i64(sum, "cycle_risk_count") as i32;
    }
    // files (for expandable detail in history tab)
    if let Some(Value::Array(arr)) = v.get("files") {
        for f in arr {
            let fr = crate::types::FileResult {
                name: get_str(f, "name"),
                final_name: get_str(f, "final_name"),
                original_size: get_u64(f, "original_size"),
                compressed_size: get_u64(f, "compressed_size"),
                saved_bytes: get_i64(f, "saved_bytes"),
                cycle_risk: get_bool(f, "cycle_risk"),
                status: match get_str(f, "status").as_str() {
                    "success" => FileStatus::Success,
                    "skipped_larger" => FileStatus::SkippedLarger,
                    "failed" => FileStatus::Failed,
                    _ => FileStatus::SkippedOther,
                },
                ..Default::default()
            };
            s.files.push(fr);
        }
    }
    Some(s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{RunSummary, DirectoryResult, FileResult, FileStatus};

    fn sample_summary() -> RunSummary {
        let mut s = RunSummary::default();
        s.run_id = "2026-01-01_00-00-00".into();
        s.start_time = "2026-01-01T00:00:00+0800".into();
        s.end_time = s.start_time.clone();
        s.directories.push(DirectoryResult {
            path: "D:/x".into(), config_valid: true, files_total: 1, files_processed: 1,
        });
        s.files.push(FileResult {
            name: "a.mp4".into(), status: FileStatus::Success,
            original_size: 100, compressed_size: 40, saved_bytes: 60, ..Default::default()
        });
        s.compute_totals();
        s
    }

    #[test]
    fn write_and_read_history_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_str().unwrap();
        let mut logger = Logger::new(dir);
        let s = sample_summary();
        assert!(logger.begin_run(&s));
        logger.log_file_result(&s.files[0]);
        logger.finalize_run(&s);

        let hist = read_history(dir);
        assert_eq!(hist.len(), 1);
        assert_eq!(hist[0].success_count, 1);
        assert_eq!(hist[0].total_saved_bytes, 60);
    }

    #[test]
    fn clean_old_logs_keeps_recent() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_str().unwrap();
        let mut logger = Logger::new(dir);
        let s = sample_summary();
        logger.begin_run(&s);
        logger.finalize_run(&s);
        logger.clean_old_logs(90); // recent file, retention 90d → kept
        assert_eq!(read_history(dir).len(), 1);
    }
}
