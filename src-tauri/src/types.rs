use serde::{Deserialize, Serialize};

/// Per-file compression status. Mirrors FileStatus (C++).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileStatus {
    Success,
    SkippedLarger,
    Failed,
    SkippedOther,
}

/// Per-file result. Mirrors FileResult (C++).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileResult {
    pub name: String,
    pub path: String,
    pub final_name: String,
    pub final_path: String,
    pub status: FileStatus,
    pub original_size: u64,
    pub compressed_size: u64,
    pub saved_bytes: i64,
    pub ffmpeg_exit_code: i32,
    pub ffmpeg_duration_ms: i32,
    pub cycle_risk: bool,
    pub error_message: String,
}

impl Default for FileResult {
    fn default() -> Self {
        FileResult {
            name: String::new(), path: String::new(),
            final_name: String::new(), final_path: String::new(),
            status: FileStatus::SkippedOther,
            original_size: 0, compressed_size: 0, saved_bytes: 0,
            ffmpeg_exit_code: -1, ffmpeg_duration_ms: 0, cycle_risk: false,
            error_message: String::new(),
        }
    }
}

/// Directory summary within a run. Mirrors DirectoryResult (C++).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryResult {
    pub path: String,
    pub config_valid: bool,
    pub files_total: i32,
    pub files_processed: i32,
}

/// Full run summary. Mirrors RunSummary (C++).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunSummary {
    pub run_id: String,
    pub start_time: String,
    pub end_time: String,
    pub duration_seconds: i32,
    pub directories: Vec<DirectoryResult>,
    pub files: Vec<FileResult>,
    pub success_count: i32,
    pub skipped_larger_count: i32,
    pub failed_count: i32,
    pub skipped_other_count: i32,
    pub total_saved_bytes: i64,
    pub cycle_risk_count: i32,
}

impl RunSummary {
    /// Mirrors RunSummary::computeTotals.
    pub fn compute_totals(&mut self) {
        self.success_count = 0;
        self.skipped_larger_count = 0;
        self.failed_count = 0;
        self.skipped_other_count = 0;
        self.total_saved_bytes = 0;
        self.cycle_risk_count = 0;
        for f in &self.files {
            match f.status {
                FileStatus::Success => self.success_count += 1,
                FileStatus::SkippedLarger => self.skipped_larger_count += 1,
                FileStatus::Failed => self.failed_count += 1,
                FileStatus::SkippedOther => self.skipped_other_count += 1,
            }
            self.total_saved_bytes += f.saved_bytes;
            if f.cycle_risk { self.cycle_risk_count += 1; }
        }
    }
}

/// Runtime stage of a directory. Mirrors DirRuntimeState::Stage.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Stage { Idle, Scanning, Compressing, Completed }

/// Per-directory runtime state pushed to the UI. Mirrors DirRuntimeState.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirRuntimeState {
    pub dir_path: String,
    pub stage: Stage,
    pub status_text: String,
    pub current_file: String,
    pub completed_files: i32,
    pub total_files: i32,
    pub last_run_time: String,
    pub last_run_result: String,
    pub next_run_time: String,
}

impl DirRuntimeState {
    pub fn new(dir_path: String) -> Self {
        DirRuntimeState {
            dir_path, stage: Stage::Idle, status_text: String::new(),
            current_file: String::new(), completed_files: 0, total_files: 0,
            last_run_time: String::new(), last_run_result: String::new(),
            next_run_time: String::new(),
        }
    }
}

/// Card info for the directory list (level 1 UI aggregate).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirCardInfo {
    pub path: String,
    pub enabled: bool,
    pub badge: String,
    pub badge_detail: String,
    pub file_count: i32,
    pub total_size: u64,
    pub params_name: String,
    pub cycle_risk_count: i32,
    pub last_run_time: String,
    pub last_run_result: String,
    pub next_run_time: String,
    pub next_run_count: i32,
    pub next_run_size: u64,
}

/// A single file in the preview table.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilePreview {
    pub relative_path: String,
    pub final_name: String,
    pub file_size: u64,
    pub cycle_risk: bool,
    pub in_next_run: bool,
}

/// ffmpeg availability result. Mirrors FfmpegProbe.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FfmpegStatus {
    pub ready: bool,
    pub version: String,
    pub error: String,
}

/// Directory-level config as a form view (for TabConfig).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirConfigView {
    pub exists: bool,
    pub valid: bool,
    pub error_message: String,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub max_size_mb: Option<f64>,
    pub min_size_mb: Option<f64>,
    pub mtime_after: Option<String>,
    pub mtime_before: Option<String>,
    pub ctime_after: Option<String>,
    pub ctime_before: Option<String>,
    pub max_compress_size_mb: Option<f64>,
    pub rename_rules: Vec<RenameRuleView>,
    pub params: String,
    pub use_custom_params: bool,
    pub schedule_time: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameRuleView {
    pub pattern: String,
    pub replacement: String,
}
