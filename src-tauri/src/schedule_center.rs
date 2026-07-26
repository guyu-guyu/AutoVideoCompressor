//! ScheduleCenter 调度后端。
//!
//! 封装 ScheduleCenter CLI 调用，将目录级定时任务同步为 Windows 计划任务
//!（存放于 `\ScheduleCenter\` 文件夹下），实现"应用未运行时也能按时启动压缩"。
//!
//! # 命名规范
//! 每个目录对应一个任务，任务名 = `AutoCompress\` + 目录路径经安全转写：
//!   `D:\Videos\Movies` → `AutoCompress\D-Videos-Movies`
//!
//! # CLI 依赖
//! 需要 `ScheduleCenter.exe` 在 PATH 中（已预装于此设备）。

use crate::config::directory_config::DirectoryConfig;
use crate::config::global_config::DirEntry;
use serde::Deserialize;
use std::path::PathBuf;
use std::process::Command;

// ---------------------------------------------------------------------------
// ScheduleCenter JSON 输出反序列化（只用到的字段）
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct ListOutput {
    #[allow(dead_code)]
    success: bool,
    #[serde(default)]
    tasks: Vec<TaskInfo>,
}

#[derive(Deserialize)]
struct TaskInfo {
    #[serde(default)]
    name: String,
}

// ---------------------------------------------------------------------------
// ScheduleCenter 管理器
// ---------------------------------------------------------------------------

pub struct ScheduleCenter {
    /// 当前可执行文件路径（ScheduleCenter 用它创建计划任务的目标程序）
    app_exe: PathBuf,
}

impl ScheduleCenter {
    /// 创建管理器，自动获取当前 exe 路径。
    pub fn new() -> Self {
        let app_exe = std::env::current_exe()
            .unwrap_or_else(|_| PathBuf::from("autovideocompressor.exe"));
        ScheduleCenter { app_exe }
    }

    // ------------------------------------------------------------------
    // 任务名工具
    // ------------------------------------------------------------------

    /// 将目录路径转写为安全的 ScheduleCenter 任务名。
    /// `D:\Videos\Movies` → `AutoCompress\D-Videos-Movies`
    pub fn task_name(dir: &str) -> String {
        let safe = dir
            .replace('\\', "/")     // 统一正斜杠
            .replace(":/", "-")     // D:/ → D-
            .replace('/', "-")      // 分隔符 → -
            .replace(':', "-");     // 残留冒号
        format!("AutoCompress\\{safe}")
    }

    // ------------------------------------------------------------------
    // 核心操作
    // ------------------------------------------------------------------

    /// 为一个目录创建每日定时任务。
    fn add_daily_task(&self, dir: &str, time: &str) -> Result<(), String> {
        let name = Self::task_name(dir);
        let exe = self.app_exe.to_string_lossy().to_string();
        // 路径含空格时需引号包裹
        let args = format!("--run-once --directory \"{}\"", dir);

        let output = Command::new("ScheduleCenter")
            .args([
                "add",
                "--name", &name,
                "--path", &exe,
                "--args", &args,
                "--trigger", "daily",
                "--time", time,
            ])
            .output()
            .map_err(|e| format!("无法启动 ScheduleCenter: {e}"))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let msg = extract_error_message(&stderr);
            // TASK_EXISTS (exit 5) 视为成功——任务已存在，无需重复创建
            if output.status.code() == Some(5) {
                return Ok(());
            }
            Err(format!("ScheduleCenter add 失败: {msg}"))
        }
    }

    /// 删除一个目录对应的计划任务。
    pub fn remove_task(&self, dir: &str) -> Result<(), String> {
        let name = Self::task_name(dir);
        let output = Command::new("ScheduleCenter")
            .args(["delete", "--name", &name, "--force"])
            .output()
            .map_err(|e| format!("无法启动 ScheduleCenter: {e}"))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let msg = extract_error_message(&stderr);
            // TASK_NOT_FOUND (exit 4) 视为成功——任务本来就不存在
            if output.status.code() == Some(4) {
                return Ok(());
            }
            Err(format!("ScheduleCenter delete 失败: {msg}"))
        }
    }

    /// 更新已有任务：先删后加（ScheduleCenter 不支持 `update` 切换触发类型）。
    fn upsert_task(&self, dir: &str, time: &str) -> Result<(), String> {
        // 忽略删除错误（可能不存在）
        let _ = self.remove_task(dir);
        self.add_daily_task(dir, time)
    }

    /// 根据目录的调度配置同步单个任务。
    /// - `schedule_time = Some("HH:MM")` 且 `enabled = true` → 创建/更新每日定时任务
    /// - 其他情况 → 删除已有任务
    pub fn sync_directory(
        &self,
        dir: &str,
        schedule_time: Option<&str>,
        enabled: bool,
    ) -> Result<(), String> {
        match (schedule_time, enabled) {
            (Some(time), true) if !time.is_empty() => self.upsert_task(dir, time),
            _ => self.remove_task(dir),
        }
    }

    /// 同步所有目录的调度状态到 ScheduleCenter。
    /// 遍历 `directories`，对每个目录加载其配置并执行 sync_directory。
    pub fn sync_all(&self, directories: &[DirEntry]) {
        for d in directories {
            let cfg = DirectoryConfig::load(&d.path);
            if let Err(e) = self.sync_directory(
                &d.path,
                cfg.schedule_time.as_deref(),
                d.enabled,
            ) {
                eprintln!(
                    "[schedule_center] 同步目录失败 '{}': {e}",
                    d.path
                );
            }
        }
    }

    /// 删除所有 `AutoCompress\` 前缀的任务（用于完全清理）。
    pub fn remove_all(&self) {
        let Ok(tasks) = self.list_tasks() else { return };
        for name in &tasks {
            let result = Command::new("ScheduleCenter")
                .args(["delete", "--name", name, "--force"])
                .output();
            match result {
                Ok(out) if !out.status.success() => {
                    let e = String::from_utf8_lossy(&out.stderr);
                    eprintln!("[schedule_center] 删除任务 '{name}' 失败: {e}");
                }
                Err(e) => {
                    eprintln!("[schedule_center] 无法运行 ScheduleCenter: {e}");
                }
                _ => {}
            }
        }
    }

    /// 列出 `AutoCompress\` 前缀的所有任务名。
    pub fn list_tasks(&self) -> Result<Vec<String>, String> {
        let output = Command::new("ScheduleCenter")
            .args(["list", "--filter", "AutoCompress\\*"])
            .output()
            .map_err(|e| format!("无法启动 ScheduleCenter: {e}"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("ScheduleCenter list 失败: {}", extract_error_message(&stderr)));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let parsed: ListOutput =
            serde_json::from_str(&stdout).map_err(|e| format!("JSON 解析失败: {e}"))?;

        Ok(parsed.tasks.into_iter().map(|t| t.name).collect())
    }

    /// 立即运行一个目录的 ScheduleCenter 任务（用于测试/手动触发）。
    pub fn run_task(&self, dir: &str) -> Result<(), String> {
        let name = Self::task_name(dir);
        let output = Command::new("ScheduleCenter")
            .args(["run", "--name", &name])
            .output()
            .map_err(|e| format!("无法启动 ScheduleCenter: {e}"))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("ScheduleCenter run 失败: {}", extract_error_message(&stderr)))
        }
    }
}

impl Default for ScheduleCenter {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// 辅助函数
// ---------------------------------------------------------------------------

/// 从 ScheduleCenter 的 stderr JSON 中提取人类可读的错误消息。
fn extract_error_message(stderr: &str) -> String {
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(stderr) {
        if let Some(msg) = v.get("error")
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
        {
            return msg.to_string();
        }
    }
    // fallback：按行取最后一段非空文本
    stderr.lines().filter(|l| !l.is_empty()).last().unwrap_or(stderr).to_string()
}

// ---------------------------------------------------------------------------
// 测试
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_name_simple() {
        assert_eq!(
            ScheduleCenter::task_name(r"D:\Videos\Movies"),
            "AutoCompress\\D-Videos-Movies"
        );
    }

    #[test]
    fn task_name_with_spaces() {
        // 空格保持原样（任务名允许空格）
        assert_eq!(
            ScheduleCenter::task_name(r"E:\My Videos\test footage"),
            "AutoCompress\\E-My Videos-test footage"
        );
    }

    #[test]
    fn task_name_unix_style() {
        // 前导 / 会变成空字符串后的 -，合理
        assert_eq!(
            ScheduleCenter::task_name("/mnt/videos/movies"),
            "AutoCompress\\-mnt-videos-movies"
        );
    }

    #[test]
    fn task_name_drive_letter_only() {
        assert_eq!(
            ScheduleCenter::task_name(r"D:\"),
            "AutoCompress\\D-"
        );
    }

    #[test]
    fn extract_error_from_json() {
        let json = r#"{"success":false,"error":{"code":"INVALID_ARGUMENTS","message":"未知选项 --help"}}"#;
        assert_eq!(extract_error_message(json), "未知选项 --help");
    }

    #[test]
    fn extract_error_from_plain_text() {
        let text = "Some raw error\nline 2";
        assert_eq!(extract_error_message(text), "line 2");
    }
}
