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

/// Compute which files will be compressed in the next run, respecting max_compress_size.
/// Returns (files_in_next_run, files_exceeding_limit).
/// When max_compress_size_bytes is None, all files are in the next run.
pub fn compute_next_run_set(mut files: Vec<ScanFile>, max_compress_size_bytes: Option<u64>) -> (Vec<ScanFile>, Vec<ScanFile>) {
    // Sort by relative path for deterministic order
    files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));

    let max_bytes = match max_compress_size_bytes {
        Some(b) if b > 0 => b,
        _ => return (files, Vec::new()),
    };

    let mut in_run = Vec::new();
    let mut skipped = Vec::new();
    let mut accumulated: u64 = 0;

    for f in files {
        if accumulated + f.file_size <= max_bytes {
            accumulated += f.file_size;
            in_run.push(f);
        } else {
            skipped.push(f);
        }
    }
    (in_run, skipped)
}

/// Scan a directory per its config. Mirrors FileScanner::scan.
/// Strip the Windows canonicalize `\\?\` prefix — ffmpeg doesn't handle it.
fn strip_verbatim_prefix(p: &str) -> &str {
    if p.starts_with("\\\\?\\") { &p[4..] } else { p }
}

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
        let abs_clean = {
            let s = abs.to_string_lossy().into_owned();
            strip_verbatim_prefix(&s).to_string()
        };
        let relative = match abs.strip_prefix(&root_abs) {
            Ok(r) => r.to_string_lossy().replace('\\', "/"),
            Err(_) => continue,
        };
        if !config.passes_filters(&relative, fi.size, fi.modified, fi.created) {
            continue;
        }
        // temp_name is just the filename (last component) with _tmp inserted
        let fname = Path::new(&relative)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(&relative);

        out.push(ScanFile {
            temp_name: insert_temp_suffix(fname),
            final_name: config.matcher.apply_rename(&relative),
            cycle_risk: config.matcher.has_cycle_risk(&relative),
            relative_path: relative,
            absolute_path: abs_clean,
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
