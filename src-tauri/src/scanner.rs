use crate::config::directory_config::DirectoryConfig;
use crate::util::fs_util::list_files_recursive;
use crate::util::string_util::insert_temp_suffix;
use std::path::Path;
use std::time::SystemTime;

/// One scanned file with derived names. Mirrors ScanFile (C++).
#[derive(Debug, Clone)]
pub struct ScanFile {
    pub relative_path: String,
    pub absolute_path: String,
    pub file_size: u64,
    pub modified: SystemTime,
    pub created: SystemTime,
    pub temp_name: String,
    pub final_name: String,
    pub cycle_risk: bool,
}

/// Scan a directory per its config. Mirrors FileScanner::scan.
pub fn scan(config: &DirectoryConfig) -> Vec<ScanFile> {
    let mut out = Vec::new();
    if !config.valid {
        return out;
    }
    let root = Path::new(&config.directory_path);
    if !root.is_dir() {
        return out;
    }
    let root_abs = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());

    for fi in list_files_recursive(root) {
        let abs = std::fs::canonicalize(&fi.path).unwrap_or_else(|_| fi.path.clone());
        let relative = match abs.strip_prefix(&root_abs) {
            Ok(r) => r.to_string_lossy().replace('\\', "/"),
            Err(_) => continue,
        };
        if !config.passes_filters(&relative, fi.size, fi.modified, fi.created) {
            continue;
        }
        out.push(ScanFile {
            temp_name: insert_temp_suffix(&relative),
            final_name: config.matcher.apply_rename(&relative),
            cycle_risk: config.matcher.has_cycle_risk(&relative),
            relative_path: relative,
            absolute_path: abs.to_string_lossy().to_string(),
            file_size: fi.size,
            modified: fi.modified,
            created: fi.created,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::directory_config::DirectoryConfig;

    #[test]
    fn scan_matches_include_and_computes_names() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_str().unwrap();
        std::fs::write(tmp.path().join("a.mp4"), b"data").unwrap();
        std::fs::write(tmp.path().join("b.txt"), b"data").unwrap();
        DirectoryConfig::create_default(dir, "");
        let cfg = DirectoryConfig::load(dir);

        let files = scan(&cfg);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].relative_path, "a.mp4");
        assert_eq!(files[0].temp_name, "a_tmp.mp4");
        assert_eq!(files[0].final_name, "a[compress].mp4");
    }

    #[test]
    fn invalid_config_scans_nothing() {
        let tmp = tempfile::tempdir().unwrap();
        let cfg = DirectoryConfig::load(tmp.path().to_str().unwrap());
        assert!(scan(&cfg).is_empty());
    }
}
