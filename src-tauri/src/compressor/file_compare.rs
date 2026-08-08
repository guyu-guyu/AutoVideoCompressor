use crate::types::{FileResult, FileStatus};
use crate::util::fs_util::{safe_delete, safe_rename};
use crate::util::string_util::remove_temp_suffix;
use std::path::Path;

/// Compare original vs compressed, keep the smaller, clean up temp file.
/// Mirrors FileCompare::compareAndCleanup.
///
/// `final_path` is the already-computed final destination (relative-path based,
/// computed by the caller via `matcher.apply_rename`), so renaming is consistent
/// with the rest of the pipeline and not recomputed here from just the file name.
pub fn compare_and_cleanup(
    original_path: &str,
    original_size: u64,
    compressed_path: &str,
    compressed_size: u64,
    final_path: &str,
    ffmpeg_exit_code: i32,
    ffmpeg_duration_ms: i32,
) -> FileResult {
    let orig = Path::new(original_path);
    let name = orig.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string();
    let final_name = Path::new(final_path)
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or(&name)
        .to_string();

    let mut result = FileResult {
        name: name.clone(),
        path: original_path.to_string(),
        final_name: final_name.clone(),
        final_path: final_path.to_string(),
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

    // Defense-in-depth: a "compressed" output that is missing or 0 bytes can
    // never be a valid result, even if ffmpeg reported exit 0 (e.g. disk full,
    // encoder produced nothing). Treat it as failure and keep the original —
    // deleting the original here was the "→ 0.0 B" data-loss bug.
    if compressed_size == 0 || !Path::new(compressed_path).exists() {
        result.status = FileStatus::Failed;
        result.error_message = "压缩输出为空或不存在".into();
        safe_delete(Path::new(compressed_path));
        return result;
    }

    if compressed_size < original_size {
        result.status = FileStatus::Success;
        result.saved_bytes = original_size as i64 - compressed_size as i64;
        safe_delete(orig);
        let clean_path = remove_temp_suffix(compressed_path);
        safe_rename(Path::new(compressed_path), Path::new(&clean_path));
        let target = Path::new(final_path);
        if clean_path != target.to_string_lossy() {
            safe_rename(Path::new(&clean_path), target);
        }
        result.final_path = target.to_string_lossy().to_string();
    } else {
        result.status = FileStatus::SkippedLarger;
        result.saved_bytes = -((compressed_size - original_size) as i64);
        safe_delete(Path::new(compressed_path));
        // Compressed file was not smaller — keep the original, but still apply
        // the rename rule to it so the kept file no longer matches include and
        // won't be recompressed (and discarded) on every subsequent run.
        let target = Path::new(final_path);
        if target != orig {
            safe_rename(orig, target);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::FileStatus;

    #[test]
    fn smaller_compressed_replaces_original() {
        let tmp = tempfile::tempdir().unwrap();
        let orig = tmp.path().join("a.mp4");
        let comp = tmp.path().join("a_tmp.mp4");
        std::fs::write(&orig, vec![0u8; 100]).unwrap();
        std::fs::write(&comp, vec![0u8; 40]).unwrap();

        let final_path = tmp.path().join("a[compress].mp4").to_string_lossy().to_string();
        let r = compare_and_cleanup(
            orig.to_str().unwrap(), 100, comp.to_str().unwrap(), 40,
            &final_path, 0, 500);
        assert_eq!(r.status, FileStatus::Success);
        assert_eq!(r.saved_bytes, 60);
        assert_eq!(r.final_name, "a[compress].mp4");
        assert!(!orig.exists());
        assert!(tmp.path().join("a[compress].mp4").exists());
    }

    #[test]
    fn larger_compressed_discarded_and_original_renamed() {
        let tmp = tempfile::tempdir().unwrap();
        let orig = tmp.path().join("a.mp4");
        let comp = tmp.path().join("a_tmp.mp4");
        std::fs::write(&orig, vec![0u8; 40]).unwrap();
        std::fs::write(&comp, vec![0u8; 100]).unwrap();

        let final_path = tmp.path().join("a[compress].mp4").to_string_lossy().to_string();
        let r = compare_and_cleanup(
            orig.to_str().unwrap(), 40, comp.to_str().unwrap(), 100,
            &final_path, 0, 500);
        assert_eq!(r.status, FileStatus::SkippedLarger);
        // Compressed was discarded...
        assert!(!comp.exists());
        // ...and the original was kept, but renamed per the rename rule so it
        // won't match include again on the next run.
        assert!(!orig.exists());
        assert!(tmp.path().join("a[compress].mp4").exists());
        assert_eq!(r.final_name, "a[compress].mp4");
    }

    #[test]
    fn larger_compressed_keeps_original_when_rename_is_noop() {
        let tmp = tempfile::tempdir().unwrap();
        let orig = tmp.path().join("a.mp4");
        let comp = tmp.path().join("a_tmp.mp4");
        std::fs::write(&orig, vec![0u8; 40]).unwrap();
        std::fs::write(&comp, vec![0u8; 100]).unwrap();

        // final_path identical to the original (e.g. no rename rule matched):
        // the rename is a no-op and the original stays untouched.
        let final_path = orig.to_string_lossy().to_string();
        let r = compare_and_cleanup(
            orig.to_str().unwrap(), 40, comp.to_str().unwrap(), 100,
            &final_path, 0, 500);
        assert_eq!(r.status, FileStatus::SkippedLarger);
        assert!(orig.exists());
        assert!(!comp.exists());
    }

    /// Regression test for the "→ 0.0 B" data-loss bug: when ffmpeg timed out /
    /// was killed, the partial temp file was already deleted and exit_code was
    /// left at 0, so compare_and_cleanup treated a 0-byte "compressed" output
    /// as Success and deleted the original. A 0-byte output must NEVER delete
    /// the original.
    #[test]
    fn empty_compressed_output_never_deletes_original() {
        let tmp = tempfile::tempdir().unwrap();
        let orig = tmp.path().join("a.mp4");
        let comp = tmp.path().join("a_tmp.mp4");
        std::fs::write(&orig, vec![0u8; 100]).unwrap();
        // compressed file missing entirely (already cleaned up after timeout)

        let final_path = tmp.path().join("a[compress].mp4").to_string_lossy().to_string();
        let r = compare_and_cleanup(
            orig.to_str().unwrap(), 100, comp.to_str().unwrap(), 0,
            &final_path, 0, 500);
        assert_ne!(r.status, FileStatus::Success, "0 字节输出绝不能判成功");
        assert!(orig.exists(), "0 字节输出时原文件必须保留");
        assert!(!comp.exists());
    }

    #[test]
    fn ffmpeg_failure_keeps_original() {
        let tmp = tempfile::tempdir().unwrap();
        let orig = tmp.path().join("a.mp4");
        let comp = tmp.path().join("a_tmp.mp4");
        std::fs::write(&orig, vec![0u8; 40]).unwrap();
        std::fs::write(&comp, vec![0u8; 10]).unwrap();

        let final_path = tmp.path().join("a[compress].mp4").to_string_lossy().to_string();
        let r = compare_and_cleanup(
            orig.to_str().unwrap(), 40, comp.to_str().unwrap(), 10,
            &final_path, 1, 500);
        assert_eq!(r.status, FileStatus::Failed);
        assert!(orig.exists());
        assert!(!comp.exists());
    }
}
