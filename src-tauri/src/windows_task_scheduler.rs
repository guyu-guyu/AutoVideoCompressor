//! Windows Task Scheduler backend implemented with the built-in `schtasks.exe`.
//!
//! Each configured directory maps to one daily task. The task starts this
//! executable with `--scheduled --directory <path>`, so the GUI opens for the
//! requested directory even after the desktop application exits.

use crate::config::directory_config::DirectoryConfig;
use crate::config::global_config::DirEntry;
use std::path::PathBuf;
use std::process::{Command, Output};

const TASK_PREFIX: &str = "AutoVideoCompressor-";

pub struct WindowsTaskScheduler {
    app_exe: PathBuf,
}

impl WindowsTaskScheduler {
    /// Creates a scheduler for the current executable.
    pub fn new() -> Self {
        let app_exe =
            std::env::current_exe().unwrap_or_else(|_| PathBuf::from("autovideocompressor.exe"));
        Self { app_exe }
    }

    /// Converts a directory path to a Task Scheduler-safe, root-level name.
    pub fn task_name(dir: &str) -> String {
        let safe = dir
            .replace('\\', "/")
            .replace(":/", "-")
            .replace('/', "-")
            .replace(':', "-");
        format!("{TASK_PREFIX}{safe}")
    }

    fn add_daily_task(&self, dir: &str, time: &str) -> Result<(), String> {
        validate_time(time)?;

        let name = Self::task_name(dir);
        let action = self.scheduled_action(dir);
        let output = Command::new("schtasks.exe")
            .args([
                "/Create", "/TN", &name, "/TR", &action, "/SC", "DAILY", "/ST", time, "/IT", "/F",
            ])
            .output()
            .map_err(|e| format!("无法启动 Windows 计划任务工具 schtasks.exe: {e}"))?;

        command_result("创建 Windows 计划任务", output)
    }

    fn scheduled_action(&self, dir: &str) -> String {
        format!(
            "{} --scheduled --directory {}",
            quote_windows_arg(&self.app_exe.to_string_lossy()),
            quote_windows_arg(dir),
        )
    }

    /// Removes the task for a directory. Missing tasks are treated as success.
    pub fn remove_task(&self, dir: &str) -> Result<(), String> {
        let name = Self::task_name(dir);
        let query = Command::new("schtasks.exe")
            .args(["/Query", "/TN", &name])
            .output()
            .map_err(|e| format!("无法启动 Windows 计划任务工具 schtasks.exe: {e}"))?;
        if !query.status.success() {
            return Ok(());
        }

        let output = Command::new("schtasks.exe")
            .args(["/Delete", "/TN", &name, "/F"])
            .output()
            .map_err(|e| format!("无法启动 Windows 计划任务工具 schtasks.exe: {e}"))?;
        command_result("删除 Windows 计划任务", output)
    }

    fn upsert_task(&self, dir: &str, time: &str) -> Result<(), String> {
        // `/F` makes `/Create` replace an existing task with the same name.
        self.add_daily_task(dir, time)
    }

    /// Creates or removes one task according to its directory configuration.
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

    /// Synchronizes every configured directory with Windows Task Scheduler.
    pub fn sync_all(&self, directories: &[DirEntry]) {
        for directory in directories {
            let config = DirectoryConfig::load(&directory.path);
            if let Err(error) = self.sync_directory(
                &directory.path,
                config.schedule_time.as_deref(),
                directory.enabled,
            ) {
                eprintln!(
                    "[windows_task_scheduler] 同步目录失败 '{}': {error}",
                    directory.path
                );
            }
        }
    }

    /// Removes tasks for all known directories.
    pub fn remove_all(&self, directories: &[DirEntry]) {
        for directory in directories {
            if let Err(error) = self.remove_task(&directory.path) {
                eprintln!(
                    "[windows_task_scheduler] 删除目录任务失败 '{}': {error}",
                    directory.path
                );
            }
        }
    }

    /// Starts a directory task immediately.
    pub fn run_task(&self, dir: &str) -> Result<(), String> {
        let name = Self::task_name(dir);
        let output = Command::new("schtasks.exe")
            .args(["/Run", "/TN", &name])
            .output()
            .map_err(|e| format!("无法启动 Windows 计划任务工具 schtasks.exe: {e}"))?;
        command_result("运行 Windows 计划任务", output)
    }
}

impl Default for WindowsTaskScheduler {
    fn default() -> Self {
        Self::new()
    }
}

fn validate_time(time: &str) -> Result<(), String> {
    let Some((hour, minute)) = time.split_once(':') else {
        return Err(format!("无效的计划任务时间: {time}"));
    };
    if hour.len() != 2 || minute.len() != 2 {
        return Err(format!("无效的计划任务时间: {time}"));
    }
    let hour = hour
        .parse::<u8>()
        .map_err(|_| format!("无效的计划任务时间: {time}"))?;
    let minute = minute
        .parse::<u8>()
        .map_err(|_| format!("无效的计划任务时间: {time}"))?;
    if hour > 23 || minute > 59 {
        return Err(format!("无效的计划任务时间: {time}"));
    }
    Ok(())
}

/// Quotes one argument for the command line that Task Scheduler will parse.
fn quote_windows_arg(arg: &str) -> String {
    if !arg.is_empty() && !arg.chars().any(|c| c.is_whitespace() || c == '"') {
        return arg.to_string();
    }

    let mut quoted = String::from("\"");
    let mut backslashes = 0;
    for ch in arg.chars() {
        match ch {
            '\\' => backslashes += 1,
            '"' => {
                quoted.push_str(&"\\".repeat(backslashes * 2 + 1));
                quoted.push('"');
                backslashes = 0;
            }
            _ => {
                quoted.push_str(&"\\".repeat(backslashes));
                backslashes = 0;
                quoted.push(ch);
            }
        }
    }
    quoted.push_str(&"\\".repeat(backslashes * 2));
    quoted.push('"');
    quoted
}

fn command_result(operation: &str, output: Output) -> Result<(), String> {
    if output.status.success() {
        return Ok(());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let detail = if !stderr.is_empty() { stderr } else { stdout };
    let code = output
        .status
        .code()
        .map(|value| value.to_string())
        .unwrap_or_else(|| "unknown".into());
    Err(format!("{operation}失败 (exit {code}): {detail}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_name_simple() {
        assert_eq!(
            WindowsTaskScheduler::task_name(r"D:\Videos\Movies"),
            "AutoVideoCompressor-D-Videos-Movies"
        );
    }

    #[test]
    fn task_name_with_spaces() {
        assert_eq!(
            WindowsTaskScheduler::task_name(r"E:\My Videos\test footage"),
            "AutoVideoCompressor-E-My Videos-test footage"
        );
    }

    #[test]
    fn validates_daily_time() {
        assert!(validate_time("00:00").is_ok());
        assert!(validate_time("23:59").is_ok());
        assert!(validate_time("24:00").is_err());
        assert!(validate_time("9:30").is_err());
    }

    #[test]
    fn quotes_windows_action_arguments() {
        assert_eq!(
            quote_windows_arg(r"C:\Program Files\app.exe"),
            r#""C:\Program Files\app.exe""#
        );
        assert_eq!(quote_windows_arg(r"D:\Videos"), r"D:\Videos");
        assert_eq!(quote_windows_arg(""), r#""""#);
    }

    #[test]
    fn scheduled_action_targets_exactly_one_directory() {
        let scheduler = WindowsTaskScheduler {
            app_exe: PathBuf::from(r"C:\Program Files\AutoVideoCompressor.exe"),
        };
        assert_eq!(
            scheduler.scheduled_action(r"D:\My Videos"),
            r#""C:\Program Files\AutoVideoCompressor.exe" --scheduled --directory "D:\My Videos""#
        );
    }
}
