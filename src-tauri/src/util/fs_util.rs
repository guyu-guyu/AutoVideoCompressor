use std::path::{Path, PathBuf};
use std::time::SystemTime;
use walkdir::WalkDir;

/// File metadata gathered during a scan.
pub struct FileInfo {
    pub path: PathBuf,
    pub size: u64,
    pub modified: SystemTime,
    pub created: SystemTime,
}

/// Recursively list regular files, skipping unreadable entries.
/// Mirrors FileUtils::listFilesRecursive.
pub fn list_files_recursive(dir: &Path) -> Vec<FileInfo> {
    let mut out = Vec::new();
    if !dir.is_dir() {
        return out;
    }
    for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        if !entry.file_type().is_file() {
            continue;
        }
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
        let created = meta.created().unwrap_or(modified);
        out.push(FileInfo {
            path: entry.path().to_path_buf(),
            size: meta.len(),
            modified,
            created,
        });
    }
    out
}

/// Delete a file, returning whether it succeeded. Mirrors safeDelete.
pub fn safe_delete(path: &Path) -> bool {
    std::fs::remove_file(path).is_ok()
}

/// Delete a file with retries. Windows may briefly hold a lock on a file that
/// was just released by a killed child process (Defender/indexer scanning it),
/// so a single remove_file can fail even when the child is gone. Retries up to
/// `attempts` times with a short pause between tries. Returns true iff the file
/// is gone after the attempts (either successfully removed, or already absent).
pub fn safe_delete_retry(path: &Path, attempts: u32) -> bool {
    for i in 0..attempts.max(1) {
        if !path.exists() {
            return true;
        }
        match std::fs::remove_file(path) {
            Ok(_) => return true,
            Err(e) => {
                eprintln!(
                    "[autocompress] remove_file failed (attempt {}/{}) for {}: {}",
                    i + 1, attempts, path.display(), e
                );
                if i + 1 < attempts {
                    std::thread::sleep(std::time::Duration::from_millis(120));
                }
            }
        }
    }
    !path.exists()
}

/// Rename a file, returning whether it succeeded. Mirrors safeRename.
pub fn safe_rename(from: &Path, to: &Path) -> bool {
    std::fs::rename(from, to).is_ok()
}

/// Whether the volume holding `path` has at least `needed` free bytes.
/// Fail-closed: false when it can't be determined. Mirrors hasEnoughSpace.
pub fn has_enough_space(path: &Path, needed: u64) -> bool {
    match fs4::available_space(path) {
        Ok(free) => free >= needed,
        Err(_) => false,
    }
}

/// Global config/log base dir: %APPDATA%/AutoCompress, fallback %TEMP%.
pub fn config_base_dir() -> PathBuf {
    if let Ok(appdata) = std::env::var("APPDATA") {
        let d = PathBuf::from(appdata).join("AutoCompress");
        let _ = std::fs::create_dir_all(&d);
        return d;
    }
    let tmp = std::env::var("TEMP").unwrap_or_else(|_| "C:\\Temp".into());
    let d = PathBuf::from(tmp).join("AutoCompress");
    let _ = std::fs::create_dir_all(&d);
    d
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_files_recursive_finds_nested() {
        let tmp = tempfile::tempdir().unwrap();
        let sub = tmp.path().join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(sub.join("a.mp4"), b"hello").unwrap();
        let files = list_files_recursive(tmp.path());
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].size, 5);
    }

    #[test]
    fn safe_delete_and_rename() {
        let tmp = tempfile::tempdir().unwrap();
        let p = tmp.path().join("x.txt");
        std::fs::write(&p, b"1").unwrap();
        let q = tmp.path().join("y.txt");
        assert!(safe_rename(&p, &q));
        assert!(q.exists());
        assert!(safe_delete(&q));
        assert!(!q.exists());
    }
}
