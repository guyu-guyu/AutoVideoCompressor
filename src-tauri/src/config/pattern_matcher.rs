use crate::util::string_util::glob_to_regex;
use regex::{Regex, RegexBuilder};

/// Compiled include/exclude/rename engine. Mirrors PatternMatcher (C++).
#[derive(Default, Clone)]
pub struct PatternMatcher {
    includes: Vec<Regex>,
    excludes: Vec<Regex>,
    renames: Vec<(Regex, String)>,
}

/// Compile a rule: try as regex (icase), fall back to glob→regex (icase).
/// Anchored with ^...$ to match std::regex_match's whole-string semantics.
fn compile_icase(pat: &str) -> Regex {
    let anchored = format!("^(?:{})$", pat);
    if let Ok(re) = RegexBuilder::new(&anchored).case_insensitive(true).build() {
        return re;
    }
    let glob = glob_to_regex(pat);
    RegexBuilder::new(&glob)
        .case_insensitive(true)
        .build()
        .unwrap_or_else(|_| Regex::new("^\\z").unwrap()) // never-match fallback
}

impl PatternMatcher {
    pub fn new() -> Self {
        PatternMatcher::default()
    }

    fn compile_list(patterns: &[String]) -> Vec<Regex> {
        patterns
            .iter()
            .filter(|p| !p.is_empty() && !p.starts_with('#'))
            .map(|p| compile_icase(p))
            .collect()
    }

    pub fn set_include(&mut self, patterns: &[String]) {
        self.includes = Self::compile_list(patterns);
    }

    pub fn set_exclude(&mut self, patterns: &[String]) {
        self.excludes = Self::compile_list(patterns);
    }

    /// Rename pattern is compiled WITHOUT icase (mirrors std::regex(pattern)).
    pub fn add_rename_rule(&mut self, pattern: &str, replacement: &str) {
        if let Ok(re) = Regex::new(pattern) {
            self.renames.push((re, replacement.to_string()));
        }
    }

    pub fn is_included(&self, path: &str) -> bool {
        self.includes.iter().any(|r| r.is_match(path))
    }

    pub fn is_excluded(&self, path: &str) -> bool {
        self.excludes.iter().any(|r| r.is_match(path))
    }

    pub fn apply_rename(&self, path: &str) -> String {
        let mut result = path.to_string();
        for (re, repl) in &self.renames {
            result = re.replace_all(&result, repl.as_str()).into_owned();
        }
        result
    }

    /// Mirrors PatternMatcher::hasCycleRisk.
    pub fn has_cycle_risk(&self, original: &str) -> bool {
        if !self.is_included(original) {
            return false;
        }
        let final_name = self.apply_rename(original);
        if final_name == original {
            return true;
        }
        if self.is_excluded(&final_name) {
            return false;
        }
        self.is_included(&final_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matcher() -> PatternMatcher {
        let mut m = PatternMatcher::new();
        m.set_include(&["*.mp4".into(), "*.mov".into()]);
        m.set_exclude(&["*[compress]*".into()]);
        m.add_rename_rule("^(.+)(\\.[^.]+)$", "$1[compress]$2");
        m
    }

    #[test]
    fn include_exclude_basic() {
        let m = matcher();
        assert!(m.is_included("a.mp4"));
        assert!(!m.is_included("a.txt"));
        assert!(m.is_excluded("a[compress].mp4"));
    }

    #[test]
    fn apply_rename_inserts_tag() {
        let m = matcher();
        assert_eq!(m.apply_rename("a.mp4"), "a[compress].mp4");
    }

    #[test]
    fn cycle_risk_detection() {
        let m = matcher();
        // renamed output is excluded → no cycle risk
        assert!(!m.has_cycle_risk("a.mp4"));

        // matcher with no rename → same name → cycle risk
        let mut m2 = PatternMatcher::new();
        m2.set_include(&["*.mp4".into()]);
        assert!(m2.has_cycle_risk("a.mp4"));

        // non-included file → no risk
        assert!(!m2.has_cycle_risk("a.txt"));
    }

    #[test]
    fn comment_and_blank_lines_skipped() {
        let mut m = PatternMatcher::new();
        m.set_include(&["".into(), "# comment".into(), "*.mp4".into()]);
        assert!(m.is_included("a.mp4"));
        assert!(!m.is_included("# comment"));
    }
}
