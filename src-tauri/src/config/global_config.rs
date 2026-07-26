use crate::util::fs_util::config_base_dir;
use serde_json::Value;
use std::path::{Path, PathBuf};

#[derive(Clone)]
pub struct DirEntry {
    pub path: String,
    pub enabled: bool,
}

/// Global config, flat on-disk format matching ConfigManager (C++).
#[derive(Clone)]
pub struct GlobalConfig {
    pub directories: Vec<DirEntry>,
    pub ffmpeg_path: String,
    pub ffmpeg_timeout_seconds: i64,
    pub minimize_to_tray: bool,
    pub start_with_windows: bool,
    pub log_retention_days: i64,
    pub language: String,
    pub templates: Vec<(String, String)>,
    /// 调度后端：true = ScheduleCenter（Windows 计划任务），false = 应用内轮询（默认）。
    pub use_schedule_center: bool,
}

fn normalize(p: &str) -> String {
    let mut s = p.replace('\\', "/").to_lowercase();
    while s.ends_with('/') && s.len() > 1 {
        s.pop();
    }
    s
}

fn is_parent_of(parent: &str, child: &str) -> bool {
    if parent.len() >= child.len() { return false; }
    if !child.starts_with(parent) { return false; }
    child.as_bytes()[parent.len()] == b'/'
}

impl GlobalConfig {
    pub fn new_defaults() -> Self {
        GlobalConfig {
            directories: Vec::new(),
            ffmpeg_path: String::new(),
            ffmpeg_timeout_seconds: 3600,
            minimize_to_tray: true,
            start_with_windows: false,
            log_retention_days: 90,
            language: "zh-CN".into(),
            templates: vec![
                ("H.265 高质量".into(), "-c:v libx265 -crf 18 -preset slow -c:a aac -b:a 192k".into()),
                ("H.264 平衡".into(), "-c:v libx264 -crf 23 -preset medium -c:a aac -b:a 192k".into()),
                ("H.264 快速".into(), "-c:v libx264 -crf 28 -preset fast -c:a aac -b:a 128k".into()),
            ],
            use_schedule_center: false,
        }
    }

    pub fn config_file_path() -> PathBuf {
        config_base_dir().join("config.json")
    }

    /// Load from the default location; create defaults if missing.
    pub fn load() -> Self {
        let path = Self::config_file_path();
        match Self::load_from(&path) {
            Some(c) => c,
            None => {
                let c = Self::new_defaults();
                let _ = c.save_to(&path);
                c
            }
        }
    }

    pub fn load_from(path: &Path) -> Option<Self> {
        let text = std::fs::read_to_string(path).ok()?;
        let j: Value = serde_json::from_str(&text).ok()?;
        let mut c = Self::new_defaults();

        c.directories.clear();
        if let Some(Value::Array(arr)) = j.get("directories") {
            for e in arr {
                let path = e.get("path").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let enabled = e.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);
                if !path.is_empty() {
                    c.directories.push(DirEntry { path, enabled });
                }
            }
        }
        c.ffmpeg_path = j.get("ffmpeg_path").and_then(|v| v.as_str()).unwrap_or("").to_string();
        c.ffmpeg_timeout_seconds = j.get("ffmpeg_timeout_seconds").and_then(|v| v.as_i64()).unwrap_or(3600);
        c.minimize_to_tray = j.get("minimize_to_tray").and_then(|v| v.as_bool()).unwrap_or(true);
        c.start_with_windows = j.get("start_with_windows").and_then(|v| v.as_bool()).unwrap_or(false);
        c.log_retention_days = j.get("log_retention_days").and_then(|v| v.as_i64()).unwrap_or(90);
        c.language = j.get("language").and_then(|v| v.as_str()).unwrap_or("zh-CN").to_string();
        c.use_schedule_center = j.get("use_schedule_center").and_then(|v| v.as_bool()).unwrap_or(false);

        if let Some(Value::Array(arr)) = j.get("templates") {
            c.templates.clear();
            for t in arr {
                let name = t.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                let params = t.get("params").and_then(|v| v.as_str()).unwrap_or("").to_string();
                if !name.is_empty() {
                    c.templates.push((name, params));
                }
            }
        }
        Some(c)
    }

    pub fn save(&self) -> bool {
        self.save_to(&Self::config_file_path())
    }

    pub fn save_to(&self, path: &Path) -> bool {
        let mut j = serde_json::Map::new();
        let dirs: Vec<Value> = self.directories.iter().map(|d| {
            let mut m = serde_json::Map::new();
            m.insert("path".into(), Value::from(d.path.clone()));
            m.insert("enabled".into(), Value::from(d.enabled));
            Value::Object(m)
        }).collect();
        j.insert("directories".into(), Value::from(dirs));
        j.insert("ffmpeg_path".into(), Value::from(self.ffmpeg_path.clone()));
        j.insert("ffmpeg_timeout_seconds".into(), Value::from(self.ffmpeg_timeout_seconds));
        j.insert("minimize_to_tray".into(), Value::from(self.minimize_to_tray));
        j.insert("start_with_windows".into(), Value::from(self.start_with_windows));
        j.insert("log_retention_days".into(), Value::from(self.log_retention_days));
        j.insert("language".into(), Value::from(self.language.clone()));
        j.insert("use_schedule_center".into(), Value::from(self.use_schedule_center));
        let tmpls: Vec<Value> = self.templates.iter().map(|(n, p)| {
            let mut m = serde_json::Map::new();
            m.insert("name".into(), Value::from(n.clone()));
            m.insert("params".into(), Value::from(p.clone()));
            Value::Object(m)
        }).collect();
        j.insert("templates".into(), Value::from(tmpls));

        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let text = serde_json::to_string_pretty(&Value::Object(j)).unwrap_or_default();
        std::fs::write(path, format!("{text}\n")).is_ok()
    }

    pub fn add_directory(&mut self, path: &str) {
        if path.is_empty() { return; }
        let norm = normalize(path);
        if self.directories.iter().any(|e| normalize(&e.path) == norm) {
            return;
        }
        self.directories.push(DirEntry { path: path.to_string(), enabled: true });
    }

    pub fn remove_directory(&mut self, index: usize) {
        if index < self.directories.len() {
            self.directories.remove(index);
        }
    }

    pub fn set_enabled(&mut self, index: usize, enabled: bool) {
        if let Some(d) = self.directories.get_mut(index) {
            d.enabled = enabled;
        }
    }

    /// Mark later directories that overlap an earlier one. Mirrors detectOverlaps.
    pub fn detect_overlaps(&self) -> Vec<bool> {
        let n = self.directories.len();
        let mut flags = vec![false; n];
        #[allow(clippy::needless_range_loop)]
        for i in 0..n {
            let a = normalize(&self.directories[i].path);
            for j in (i + 1)..n {
                let b = normalize(&self.directories[j].path);
                if is_parent_of(&a, &b) || is_parent_of(&b, &a) {
                    flags[j] = true;
                }
            }
        }
        flags
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_dedup_and_remove() {
        let mut c = GlobalConfig::new_defaults();
        c.add_directory("D:/Videos");
        c.add_directory("D:/videos"); // dup (case-insensitive)
        assert_eq!(c.directories.len(), 1);
        c.add_directory("D:/Other");
        assert_eq!(c.directories.len(), 2);
        c.remove_directory(0);
        assert_eq!(c.directories.len(), 1);
        assert_eq!(c.directories[0].path, "D:/Other");
    }

    #[test]
    fn overlap_marks_later_child() {
        let mut c = GlobalConfig::new_defaults();
        c.add_directory("D:/Videos");
        c.add_directory("D:/Videos/Sub");
        let ov = c.detect_overlaps();
        assert_eq!(ov, vec![false, true]);
    }

    #[test]
    fn roundtrip_flat_format() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("config.json");
        let mut c = GlobalConfig::new_defaults();
        c.ffmpeg_path = "C:/ffmpeg.exe".into();
        c.add_directory("D:/Videos");
        assert!(c.save_to(&path));
        let loaded = GlobalConfig::load_from(&path).unwrap();
        assert_eq!(loaded.ffmpeg_path, "C:/ffmpeg.exe");
        assert_eq!(loaded.directories.len(), 1);
        assert_eq!(loaded.templates.len(), 3);
    }

    #[test]
    fn defaults_have_three_templates() {
        let c = GlobalConfig::new_defaults();
        assert_eq!(c.templates.len(), 3);
        assert_eq!(c.templates[0].0, "H.265 高质量");
        assert_eq!(c.ffmpeg_timeout_seconds, 3600);
        assert_eq!(c.log_retention_days, 90);
    }
}
