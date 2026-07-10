use crate::types::FfmpegStatus;
use crate::util::fs_util::safe_delete_retry;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Parameters for a single compression. Mirrors CompressParams.
pub struct CompressParams {
    pub ffmpeg_path: String,
    pub arguments: String,
    pub input_path: String,
    pub output_path: String,
    pub timeout_seconds: i64,
}

/// Result of a compression. Mirrors CompressResult.
#[derive(Default)]
pub struct CompressResult {
    pub success: bool,
    pub exit_code: i32,
    pub duration_ms: i32,
    pub error_message: String,
    pub cancelled: bool,
}

fn base_command(program: &str) -> Command {
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

/// Run a process with a timeout. Returns (exit_code, timed_out, captured_output).
/// exit_code is -1 on spawn failure, -2 on timeout, -3 on cancellation.
pub fn run_process(program: &str, args: &[&str], timeout_secs: i64, cancel: &AtomicBool) -> (i32, bool, String) {
    let mut cmd = base_command(program);
    cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(_) => return (-1, false, String::new()),
    };
    let start = Instant::now();
    loop {
        // Check cancellation before each poll
        if cancel.load(Ordering::SeqCst) {
            let _ = child.kill();
            let _ = child.wait(); // reap to avoid zombie
            return (-3, false, String::new());
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                let mut out = String::new();
                use std::io::Read;
                if let Some(mut s) = child.stdout.take() { let _ = s.read_to_string(&mut out); }
                if let Some(mut e) = child.stderr.take() { let _ = e.read_to_string(&mut out); }
                return (status.code().unwrap_or(-1), false, out);
            }
            Ok(None) => {
                if start.elapsed() >= Duration::from_secs(timeout_secs.max(0) as u64) {
                    let _ = child.kill();
                    let _ = child.wait();
                    return (-2, true, String::new());
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(_) => return (-1, false, String::new()),
        }
    }
}

/// Compress one file. Mirrors CompressionEngine::compress.
pub fn compress(params: &CompressParams, cancel: &AtomicBool) -> CompressResult {
    let mut result = CompressResult::default();
    let start = Instant::now();

    let mut args: Vec<String> = vec!["-i".into(), params.input_path.clone()];
    if !params.arguments.is_empty() {
        // split ffmpeg args on whitespace (mirrors passing a raw arg string)
        args.extend(params.arguments.split_whitespace().map(String::from));
    }
    args.push("-y".into());
    args.push(params.output_path.clone());
    let arg_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

    let (code, timed_out, _out) = run_process(&params.ffmpeg_path, &arg_refs, params.timeout_seconds, cancel);
    result.duration_ms = start.elapsed().as_millis() as i32;

    if code == -3 {
        result.cancelled = true;
        result.error_message = "压缩已取消".into();
        // Delete partial output. On Windows, right after ffmpeg is killed the
        // file handle may still be held by the OS briefly (or by Defender/indexer
        // scanning the file it just released) — so retry a few times.
        safe_delete_retry(Path::new(&params.output_path), 5);
    } else if timed_out {
        result.error_message = format!("压缩超时 ({}s)", params.timeout_seconds);
        // Clean up partial output on timeout (same retry rationale as above).
        safe_delete_retry(Path::new(&params.output_path), 5);
    } else if code == -1 {
        result.error_message = "无法启动 ffmpeg 进程".into();
    } else {
        result.exit_code = code;
        result.success = code == 0;
        if !result.success {
            result.error_message = format!("ffmpeg 退出码: {}", code);
        }
    }
    result
}

/// Probe ffmpeg availability + version. Mirrors CompressionEngine::probeFfmpeg.
pub fn probe_ffmpeg(path: &str) -> FfmpegStatus {
    let mut s = FfmpegStatus::default();
    if path.is_empty() {
        s.error = "未配置 ffmpeg 路径".into();
        return s;
    }
    let is_bare = !path.contains('/') && !path.contains('\\');
    if !is_bare && !Path::new(path).exists() {
        s.error = format!("ffmpeg 路径不存在: {}", path);
        return s;
    }
    let no_cancel = AtomicBool::new(false);
    let (code, timed_out, out) = run_process(path, &["-version"], 5, &no_cancel);
    if code == -1 {
        s.error = "无法启动 ffmpeg".into();
        return s;
    }
    if timed_out || code != 0 {
        s.error = format!("ffmpeg 退出码: {}", code);
        return s;
    }
    match out.find("version") {
        None => { s.error = "无法解析 version".into(); }
        Some(pos) => {
            let after = &out[pos + "version".len()..];
            let trimmed = after.trim_start();
            let ver: String = trimmed
                .chars()
                .take_while(|c| !c.is_whitespace())
                .collect();
            if ver.is_empty() {
                s.error = "version 为空".into();
            } else {
                s.version = ver;
                s.ready = true;
            }
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_empty_path_errors() {
        let s = probe_ffmpeg("");
        assert!(!s.ready);
        assert!(s.error.contains("未配置"));
    }

    #[test]
    fn probe_nonexistent_path_errors() {
        let s = probe_ffmpeg("C:/definitely/not/here/ffmpeg.exe");
        assert!(!s.ready);
        assert!(s.error.contains("不存在") || s.error.contains("启动"));
    }

    #[test]
    fn compress_reports_exit_code() {
        let cancel = AtomicBool::new(false);
        let out = run_process("cmd", &["/C", "exit", "0"], 5, &cancel);
        assert_eq!(out.0, 0); // exit code
    }
}
