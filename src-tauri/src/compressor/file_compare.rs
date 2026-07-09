use crate::config::pattern_matcher::PatternMatcher;
use crate::types::{FileResult, FileStatus};
use crate::util::fs_util::{safe_delete, safe_rename};
use crate::util::string_util::remove_temp_suffix;
use std::path::Path;

/// Compare original vs compressed, keep the smaller, clean up temp file.
/// Mirrors FileCompare::compareAndCleanup.
pub fn compare_and_cleanup(
    original_path: &str,
    original_size: u64,
    compressed_path: &str,
    compressed_size: u64,
    matcher: &PatternMatcher,
    ffmpeg_exit_code: i32,
    ffmpeg_duration_ms: i32,
) -> FileResult {
    let orig = Path::new(original_path);
    let name = orig.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string();
    let final_name = matcher.apply_rename(&name);
    let parent = orig.parent().map(|p| p.to_path_buf()).unwrap_or_default();

    let mut result = FileResult {
        name: name.clone(),
        path: original_path.to_string(),
        final_name: final_name.clone(),
        final_path: parent.join(&final_name).to_string_lossy().to_string(),
        original_size,
        compressed_size,
        ffmpeg_exit_code,
        ffmpeg_duration_ms,
        ..Default::default()
    };

    if ffmpeg_exit_code != 0 {
        result.status = FileStatus::Failed;
        safe_delete(Path::new(compressed_path));
        return result;
    }

    if compressed_size < original_size {
        result.status = FileStatus::Success;
        result.saved_bytes = original_size as i64 - compressed_size as i64;
        safe_delete(orig);
        let clean_path = remove_temp_suffix(compressed_path);
        safe_rename(Path::new(compressed_path), Path::new(&clean_path));
        let target = parent.join(&final_name);
        if clean_path != target.to_string_lossy() {
            safe_rename(Path::new(&clean_path), &target);
        }
        result.final_path = target.to_string_lossy().to_string();
    } else {
        result.status = FileStatus::SkippedLarger;
        result.saved_bytes = -((compressed_size - original_size) as i64);
        safe_delete(Path::new(compressed_path));
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::pattern_matcher::PatternMatcher;
    use crate::types::FileStatus;

    fn matcher() -> PatternMatcher {
        let mut m = PatternMatcher::new();
        m.set_include(&["*.mp4".into()]);
        m.add_rename_rule("^(.+)(\\.[^.]+)$", "$1[compress]$2");
        m
    }

    #[test]
    fn smaller_compressed_replaces_original() {
        let tmp = tempfile::tempdir().unwrap();
        let orig = tmp.path().join("a.mp4");
        let comp = tmp.path().join("a_tmp.mp4");
        std::fs::write(&orig, vec![0u8; 100]).unwrap();
        std::fs::write(&comp, vec![0u8; 40]).unwrap();

        let r = compare_and_cleanup(
            orig.to_str().unwrap(), 100, comp.to_str().unwrap(), 40, &matcher(), 0, 500);
        assert_eq!(r.status, FileStatus::Success);
        assert_eq!(r.saved_bytes, 60);
        assert!(!orig.exists());
        assert!(tmp.path().join("a[compress].mp4").exists());
    }

    #[test]
    fn larger_compressed_discarded() {
        let tmp = tempfile::tempdir().unwrap();
        let orig = tmp.path().join("a.mp4");
        let comp = tmp.path().join("a_tmp.mp4");
        std::fs::write(&orig, vec![0u8; 40]).unwrap();
        std::fs::write(&comp, vec![0u8; 100]).unwrap();

        let r = compare_and_cleanup(
            orig.to_str().unwrap(), 40, comp.to_str().unwrap(), 100, &matcher(), 0, 500);
        assert_eq!(r.status, FileStatus::SkippedLarger);
        assert!(orig.exists());
        assert!(!comp.exists());
    }

    #[test]
    fn ffmpeg_failure_keeps_original() {
        let tmp = tempfile::tempdir().unwrap();
        let orig = tmp.path().join("a.mp4");
        let comp = tmp.path().join("a_tmp.mp4");
        std::fs::write(&orig, vec![0u8; 40]).unwrap();
        std::fs::write(&comp, vec![0u8; 10]).unwrap();

        let r = compare_and_cleanup(
            orig.to_str().unwrap(), 40, comp.to_str().unwrap(), 10, &matcher(), 1, 500);
        assert_eq!(r.status, FileStatus::Failed);
        assert!(orig.exists());
        assert!(!comp.exists());
    }
}
