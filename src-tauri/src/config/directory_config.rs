use crate::config::pattern_matcher::PatternMatcher;
use chrono::{DateTime, Local, NaiveDate, TimeZone, Duration};
use serde_json::Value;
use std::path::PathBuf;
use std::time::SystemTime;

/// Parsed <dir>/.autocompress/config.json. Mirrors DirectoryConfig (C++).
#[derive(Default, Clone)]
pub struct DirectoryConfig {
    pub directory_path: String,
    pub valid: bool,
    pub error_message: String,
    pub include_patterns: Vec<String>,
    pub exclude_patterns: Vec<String>,
    pub rename_rules: Vec<(String, String)>,
    pub max_size_bytes: Option<u64>,
    pub min_size_bytes: Option<u64>,
    pub max_compress_size_bytes: Option<u64>,
    pub mtime_after: Option<String>,
    pub mtime_before: Option<String>,
    pub ctime_after: Option<String>,
    pub ctime_before: Option<String>,
    pub params: String,
    pub use_custom_params: bool,
    pub schedule_time: Option<String>,
    pub matcher: PatternMatcher,
}


fn parse_date_local_midnight(s: &str) -> Option<DateTime<Local>> {
    let d = NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()?;
    let naive = d.and_hms_opt(0, 0, 0)?;
    Local.from_local_datetime(&naive).single()
}

impl DirectoryConfig {
    pub fn config_path(dir: &str) -> PathBuf {
        PathBuf::from(dir).join(".autocompress").join("config.json")
    }

    pub fn load(dir: &str) -> Self {
        let mut cfg = DirectoryConfig {
            directory_path: dir.to_string(),
            ..Default::default()
        };
        let path = Self::config_path(dir);
        if !path.exists() {
            cfg.valid = false;
            cfg.error_message = format!("Config file not found: {}", path.display());
            return cfg;
        }
        let text = match std::fs::read_to_string(&path) {
            Ok(t) => t,
            Err(e) => {
                cfg.error_message = format!("Cannot open config file: {e}");
                return cfg;
            }
        };
        let j: Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(e) => {
                cfg.error_message = format!("JSON parse error: {e}");
                return cfg;
            }
        };

        // include (required, non-empty array)
        match j.get("include") {
            Some(Value::Array(arr)) if !arr.is_empty() => {
                cfg.include_patterns =
                    arr.iter().filter_map(|v| v.as_str().map(String::from)).collect();
            }
            _ => {
                cfg.error_message = "'include' must be a non-empty array".into();
                return cfg;
            }
        }

        if let Some(Value::Array(arr)) = j.get("exclude") {
            cfg.exclude_patterns =
                arr.iter().filter_map(|v| v.as_str().map(String::from)).collect();
        }

        if let Some(Value::Object(f)) = j.get("filters") {
            if let Some(mb) = f.get("max_size_mb").and_then(|v| v.as_f64()) {
                cfg.max_size_bytes = Some((mb * 1024.0 * 1024.0) as u64);
            }
            if let Some(mb) = f.get("min_size_mb").and_then(|v| v.as_f64()) {
                cfg.min_size_bytes = Some((mb * 1024.0 * 1024.0) as u64);
            }
            if let Some(mb) = f.get("max_compress_size_mb").and_then(|v| v.as_f64()) {
                cfg.max_compress_size_bytes = Some((mb * 1024.0 * 1024.0) as u64);
            }
            cfg.mtime_after = f.get("mtime_after").and_then(|v| v.as_str()).map(String::from);
            cfg.mtime_before = f.get("mtime_before").and_then(|v| v.as_str()).map(String::from);
            cfg.ctime_after = f.get("ctime_after").and_then(|v| v.as_str()).map(String::from);
            cfg.ctime_before = f.get("ctime_before").and_then(|v| v.as_str()).map(String::from);
        }

        if let Some(Value::Array(arr)) = j.get("rename_rules") {
            for rule in arr {
                if let (Some(p), Some(r)) = (
                    rule.get("pattern").and_then(|v| v.as_str()),
                    rule.get("replacement").and_then(|v| v.as_str()),
                ) {
                    cfg.rename_rules.push((p.to_string(), r.to_string()));
                }
            }
        }

        if let Some(p) = j.get("params").and_then(|v| v.as_str()) {
            cfg.params = p.to_string();
        }

        cfg.use_custom_params = j.get("use_custom_params").and_then(|v| v.as_bool()).unwrap_or(false);

        if let Some(Value::Object(s)) = j.get("schedule") {
            cfg.schedule_time = s.get("time").and_then(|v| v.as_str()).map(String::from);
        }

        cfg.valid = true;
        cfg.compile_matcher();
        cfg
    }

    pub fn create_default(dir: &str, params: &str) -> Self {
        let mut cfg = DirectoryConfig {
            directory_path: dir.to_string(),
            valid: true,
            include_patterns: vec![
                "*.mp4".into(), "*.mov".into(), "*.avi".into(), "*.mkv".into(),
            ],
            exclude_patterns: vec!["*[compress]*".into()],
            rename_rules: vec![("^(.+)(\\.[^.]+)$".into(), "$1[compress]$2".into())],
            ..Default::default()
        };
        if !params.is_empty() {
            cfg.params = params.to_string();
        }
        cfg.compile_matcher();
        cfg.save();
        cfg
    }

    pub fn save(&self) -> bool {
        let mut j = serde_json::Map::new();
        j.insert("include".into(), Value::from(self.include_patterns.clone()));
        if !self.exclude_patterns.is_empty() {
            j.insert("exclude".into(), Value::from(self.exclude_patterns.clone()));
        }
        let mut filters = serde_json::Map::new();
        if let Some(b) = self.max_size_bytes {
            filters.insert("max_size_mb".into(), Value::from(b as f64 / (1024.0 * 1024.0)));
        }
        if let Some(b) = self.min_size_bytes {
            filters.insert("min_size_mb".into(), Value::from(b as f64 / (1024.0 * 1024.0)));
        }
        if let Some(b) = self.max_compress_size_bytes {
            filters.insert("max_compress_size_mb".into(), Value::from(b as f64 / (1024.0 * 1024.0)));
        }
        if let Some(v) = &self.mtime_after { filters.insert("mtime_after".into(), Value::from(v.clone())); }
        if let Some(v) = &self.mtime_before { filters.insert("mtime_before".into(), Value::from(v.clone())); }
        if let Some(v) = &self.ctime_after { filters.insert("ctime_after".into(), Value::from(v.clone())); }
        if let Some(v) = &self.ctime_before { filters.insert("ctime_before".into(), Value::from(v.clone())); }
        if !filters.is_empty() {
            j.insert("filters".into(), Value::Object(filters));
        }
        if !self.rename_rules.is_empty() {
            let arr: Vec<Value> = self.rename_rules.iter().map(|(p, r)| {
                let mut m = serde_json::Map::new();
                m.insert("pattern".into(), Value::from(p.clone()));
                m.insert("replacement".into(), Value::from(r.clone()));
                Value::Object(m)
            }).collect();
            j.insert("rename_rules".into(), Value::from(arr));
        }
        if !self.params.is_empty() {
            j.insert("params".into(), Value::from(self.params.clone()));
        }
        j.insert("use_custom_params".into(), Value::from(self.use_custom_params));
        if let Some(t) = &self.schedule_time {
            let mut s = serde_json::Map::new();
            s.insert("time".into(), Value::from(t.clone()));
            j.insert("schedule".into(), Value::Object(s));
        }

        let path = Self::config_path(&self.directory_path);
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let text = serde_json::to_string_pretty(&Value::Object(j)).unwrap_or_default();
        std::fs::write(&path, format!("{text}\n")).is_ok()
    }

    fn compile_matcher(&mut self) {
        let mut m = PatternMatcher::new();
        m.set_include(&self.include_patterns);
        m.set_exclude(&self.exclude_patterns);
        for (p, r) in &self.rename_rules {
            m.add_rename_rule(p, r);
        }
        self.matcher = m;
    }

    /// Mirrors DirectoryConfig::passesFilters.
    pub fn passes_filters(
        &self,
        rel: &str,
        size: u64,
        mtime: SystemTime,
        ctime: SystemTime,
    ) -> bool {
        if !self.matcher.is_included(rel) { return false; }
        if self.matcher.is_excluded(rel) { return false; }
        if let Some(min) = self.min_size_bytes { if size < min { return false; } }
        if let Some(max) = self.max_size_bytes { if size > max { return false; } }

        let sys_m: DateTime<Local> = mtime.into();
        let sys_c: DateTime<Local> = ctime.into();

        if let Some(s) = &self.mtime_after {
            match parse_date_local_midnight(s) { Some(b) if sys_m >= b => {}, _ => return false }
        }
        if let Some(s) = &self.mtime_before {
            match parse_date_local_midnight(s) {
                Some(b) if sys_m < b + Duration::hours(24) => {}, _ => return false
            }
        }
        if let Some(s) = &self.ctime_after {
            match parse_date_local_midnight(s) { Some(b) if sys_c >= b => {}, _ => return false }
        }
        if let Some(s) = &self.ctime_before {
            match parse_date_local_midnight(s) {
                Some(b) if sys_c < b + Duration::hours(24) => {}, _ => return false
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_missing_is_invalid() {
        let tmp = tempfile::tempdir().unwrap();
        let cfg = DirectoryConfig::load(tmp.path().to_str().unwrap());
        assert!(!cfg.valid);
        assert!(cfg.error_message.contains("not found"));
    }

    #[test]
    fn create_default_then_load_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_str().unwrap();
        let created = DirectoryConfig::create_default(dir, "H.265 高质量");
        assert!(created.valid);
        assert!(DirectoryConfig::config_path(dir).exists());

        let loaded = DirectoryConfig::load(dir);
        assert!(loaded.valid);
        assert_eq!(loaded.include_patterns, vec!["*.mp4","*.mov","*.avi","*.mkv"]);
        assert_eq!(loaded.exclude_patterns, vec!["*[compress]*"]);
        assert_eq!(loaded.params, "H.265 高质量");
        assert!(loaded.schedule_time.is_none());
        assert_eq!(loaded.rename_rules.len(), 1);
    }

    #[test]
    fn empty_include_is_invalid() {
        let tmp = tempfile::tempdir().unwrap();
        let acdir = tmp.path().join(".autocompress");
        std::fs::create_dir_all(&acdir).unwrap();
        std::fs::write(acdir.join("config.json"), r#"{"include":[]}"#).unwrap();
        let cfg = DirectoryConfig::load(tmp.path().to_str().unwrap());
        assert!(!cfg.valid);
        assert!(cfg.error_message.contains("include"));
    }

    #[test]
    fn schedule_time_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_str().unwrap();
        let mut cfg = DirectoryConfig::create_default(dir, "");
        cfg.schedule_time = Some("03:00".into());
        assert!(cfg.save());
        let loaded = DirectoryConfig::load(dir);
        assert_eq!(loaded.schedule_time, Some("03:00".to_string()));
    }

    #[test]
    fn parse_error_is_invalid() {
        let tmp = tempfile::tempdir().unwrap();
        let acdir = tmp.path().join(".autocompress");
        std::fs::create_dir_all(&acdir).unwrap();
        std::fs::write(acdir.join("config.json"), "{ not json").unwrap();
        let cfg = DirectoryConfig::load(tmp.path().to_str().unwrap());
        assert!(!cfg.valid);
    }
}
