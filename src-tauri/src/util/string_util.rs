/// Convert a glob pattern to a regex string. Mirrors StringUtils::globToRegex.
pub fn glob_to_regex(glob: &str) -> String {
    let mut re = String::with_capacity(glob.len() + 4);
    re.push('^');
    for c in glob.chars() {
        match c {
            '*' => re.push_str(".*"),
            '?' => re.push('.'),
            '.' => re.push_str("\\."),
            '\\' => re.push_str("\\\\"),
            '+' => re.push_str("\\+"),
            '(' | ')' | '[' | ']' | '{' | '}' | '^' | '$' | '|' => {
                re.push('\\');
                re.push(c);
            }
            other => re.push(other),
        }
    }
    re.push('$');
    re
}

/// Insert "_tmp" before the final extension. Mirrors insertTempSuffix.
pub fn insert_temp_suffix(path: &str) -> String {
    match path.rfind('.') {
        None => format!("{path}_tmp"),
        Some(dot) => format!("{}_tmp{}", &path[..dot], &path[dot..]),
    }
}

/// Remove "_tmp" preceding the extension. Mirrors removeTempSuffix.
pub fn remove_temp_suffix(path: &str) -> String {
    match path.rfind("_tmp.") {
        None => path.to_string(),
        Some(pos) => format!("{}{}", &path[..pos], &path[pos + 4..]),
    }
}

/// Human-readable size (1 decimal, B..TB). Mirrors formatFileSize.
pub fn format_file_size(bytes: i64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit = 0usize;
    while size >= 1024.0 && unit < 4 {
        size /= 1024.0;
        unit += 1;
    }
    format!("{:.1} {}", size, UNITS[unit])
}

/// ISO8601 local timestamp (for run summaries). Mirrors StringUtils::nowToString.
pub fn now_iso_public() -> String {
    chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%z").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glob_to_regex_basic() {
        assert_eq!(glob_to_regex("*.mp4"), r"^.*\.mp4$");
        assert_eq!(glob_to_regex("a?b"), "^a.b$");
        assert_eq!(glob_to_regex("v(1).mov"), r"^v\(1\)\.mov$");
    }

    #[test]
    fn insert_temp_suffix_works() {
        assert_eq!(insert_temp_suffix("a.mp4"), "a_tmp.mp4");
        assert_eq!(insert_temp_suffix("dir/x.MOV"), "dir/x_tmp.MOV");
        assert_eq!(insert_temp_suffix("noext"), "noext_tmp");
    }

    #[test]
    fn remove_temp_suffix_works() {
        assert_eq!(remove_temp_suffix("a_tmp.mp4"), "a.mp4");
        assert_eq!(remove_temp_suffix("a.mp4"), "a.mp4");
    }

    #[test]
    fn format_file_size_works() {
        assert_eq!(format_file_size(0), "0.0 B");
        assert_eq!(format_file_size(1536), "1.5 KB");
        assert_eq!(format_file_size(1024 * 1024), "1.0 MB");
    }
}
