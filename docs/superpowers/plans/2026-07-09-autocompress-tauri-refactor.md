# AutoCompress → Tauri (Rust + Vue) 重构实现计划

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 将 `F:\Projects\AutoCompress`(C++20 + Dear ImGui + SDL2)以 Tauri 2.x (Rust) + Vue 3 + TypeScript + Vite 重构,仅 Windows,功能完全对齐原项目。

**Architecture:** 方案 A —— 胖 Rust 后端(全部业务逻辑,与原 C++ 模块 1:1)+ 瘦 Vue 前端(纯 UI)。前端通过 Tauri `command` 调用后端、通过 `event` 接收进度推送。Rust 持有运行态真值 → emit event → Pinia store 缓存 → Vue 响应式渲染。

**Tech Stack:** Rust(serde / serde_json / regex / chrono / walkdir / tempfile),Tauri 2.x(tray / autostart / opener 插件),Vue 3 + TypeScript + Vite + Pinia,Vitest。

**源项目参考(逐模块行为对齐):** `F:\Projects\AutoCompress\src\`

---

## ⚠️ 关键事实:磁盘配置格式以原项目实际实现为准

原项目 **spec 文档** 描述全局 config 为嵌套 `general`/`ffmpeg` 结构,但**实际磁盘格式是扁平 key**(见 `src/config/ConfigManager.cpp` `toJson`/`fromJson`)。为保证"读取现有配置完全相同",本计划采用**实际扁平格式**:

全局 `config.json`(位于 `%APPDATA%/AutoCompress/config.json`,APPDATA 缺失时回退 `%TEMP%/AutoCompress/`):
```json
{
  "directories": [ { "path": "D:/Videos", "enabled": true } ],
  "ffmpeg_path": "",
  "ffmpeg_timeout_seconds": 3600,
  "minimize_to_tray": true,
  "start_with_windows": false,
  "log_retention_days": 90,
  "language": "zh-CN",
  "templates": [ { "name": "H.265 高质量", "params": "-c:v libx265 -crf 18 -preset slow -c:a aac -b:a 192k" } ]
}
```

目录级 `<dir>/.autocompress/config.json`(格式与原项目一致):
```json
{
  "include": ["*.mp4","*.mov","*.avi","*.mkv"],
  "exclude": ["*[compress]*"],
  "filters": { "max_size_mb": 2048, "min_size_mb": 10, "mtime_after": "2025-01-01" },
  "rename_rules": [ { "pattern": "^(.+)(\\.[^.]+)$", "replacement": "$1[compress]$2" } ],
  "params": "H.265 高质量",
  "schedule": { "time": "03:00" }
}
```

原项目默认模板(`ConfigManager` 构造函数,首次无 config 时写出):
```
"H.265 高质量" → "-c:v libx265 -crf 18 -preset slow -c:a aac -b:a 192k"
"H.264 平衡"   → "-c:v libx264 -crf 23 -preset medium -c:a aac -b:a 192k"
"H.264 快速"   → "-c:v libx264 -crf 28 -preset fast -c:a aac -b:a 128k"
```

目录级 createDefault(`DirectoryConfig::createDefault`):
- include=`["*.mp4","*.mov","*.avi","*.mkv"]`,exclude=`["*[compress]*"]`
- rename_rules=`[{"^(.+)(\\.[^.]+)$","$1[compress]$2"}]`,filters 空,schedule 空

fallback ffmpeg 参数(dirConfig.params 与模板都解析为空时):`-c:v libx264 -crf 23 -preset fast -c:a aac -b:a 128k`

---

## 文件结构

**Rust 后端(`src-tauri/src/`)**

| 文件 | 职责 |
|------|------|
| `main.rs` | 入口:CLI(`--run-once`)、tray、autostart、启动 App、注册 commands/state |
| `error.rs` | `AppError`(派生 `Serialize`)统一错误类型 |
| `types.rs` | 全部跨 IPC 结构体(serde camelCase) |
| `util/string_util.rs` | glob→regex、insert/removeTempSuffix、formatFileSize、时间戳格式化 |
| `util/fs_util.rs` | 递归列文件、安全删/改名、磁盘剩余空间 |
| `config/global_config.rs` | 全局 config.json 读写(扁平格式)+ 目录增删/重叠检测 |
| `config/directory_config.rs` | 目录级 `.autocompress/config.json` load/save/createDefault/passesFilters |
| `config/pattern_matcher.rs` | include/exclude/rename/cycleRisk 匹配引擎 |
| `config/template_manager.rs` | 模板 resolve/names |
| `scanner.rs` | 扫描目录 → ScanFile 列表 |
| `compressor/engine.rs` | 起 ffmpeg 子进程 + 超时 + probe |
| `compressor/file_compare.rs` | 比较大小、保留较小者、去 _tmp、应用 rename |
| `scheduler.rs` | 时间表轮询线程 + computeNextRun/markCompleted |
| `logger.rs` | per-directory 日志(文本 + JSON 块)+ cleanOldLogs + 历史解析 |
| `app.rs` | 编排:startForDirectory/executeForDirectory/refreshScheduleTable/ffmpeg 探测/状态推送 |
| `commands.rs` | 全部 `#[tauri::command]` 薄封装 |

**Vue 前端(`src/`)**

| 文件 | 职责 |
|------|------|
| `types.ts` | 与 `types.rs` 对齐的 TS interface |
| `api/tauri.ts` | invoke 封装 + event 监听封装 |
| `stores/app.ts` | Pinia:目录卡片列表、ffmpeg 状态、运行态 |
| `App.vue` | 两级路由切换 |
| `views/DirectoryList.vue` | 一级:状态条 + 添加目录 + 卡片列表 |
| `views/DirectoryDetail.vue` | 二级:返回栏 + 摘要 + 三 tab |
| `components/DirCard.vue` | 卡片 |
| `components/FfmpegStatusBar.vue` | ffmpeg 状态条 |
| `components/ConfigForm.vue` | 配置表单 |
| `components/tabs/TabPreview.vue` | 预览表格 |
| `components/tabs/TabConfig.vue` | 配置 tab(含 mtime 轮询) |
| `components/tabs/TabHistory.vue` | 历史 tab |
| `components/GlobalSettings.vue` | 全局设置弹窗 |

---

## Task 1: 脚手架 — Tauri 2 + Vue 3 + TS 项目初始化

**Files:**
- Create: Tauri 项目骨架(`package.json`, `vite.config.ts`, `index.html`, `src/main.ts`, `src/App.vue`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `src-tauri/src/main.rs`, `src-tauri/build.rs`)

- [ ] **Step 1: 用官方脚手架创建项目**

工作目录 `F:\Projects\webviewApp` 已是空 git 仓库(仅 `docs/`)。因目录非空,先生成到临时目录再合并:
```bash
cd /f/Projects && npm create tauri-app@latest tmp-scaffold -- --template vue-ts --manager npm
```
然后把 `tmp-scaffold` 内容(除其 `.git`)移动到 `F:\Projects\webviewApp\` 根目录,保留已有 `docs/`,删除 `tmp-scaffold`。

- [ ] **Step 2: 安装依赖并加 Rust crates**

Run:
```bash
cd /f/Projects/webviewApp && npm install && npm install pinia
```
编辑 `src-tauri/Cargo.toml`,`[dependencies]` 加:
```toml
serde = { version = "1", features = ["derive"] }
serde_json = "1"
regex = "1"
chrono = "0.4"
walkdir = "2"
```
`[dev-dependencies]` 加:
```toml
tempfile = "3"
```

- [ ] **Step 3: 验证脚手架编译**

Run: `cd /f/Projects/webviewApp/src-tauri && cargo build 2>&1 | tail -20`
Expected: `Finished`(首次拉依赖较慢)。

- [ ] **Step 4: 提交**

```bash
cd /f/Projects/webviewApp && git add -A && git commit -m "chore: scaffold Tauri 2 + Vue 3 + TS project"
```

---

## Task 2: error.rs — 统一错误类型

**Files:**
- Create: `src-tauri/src/error.rs`
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: 写实现**

`src-tauri/src/error.rs`:
```rust
use serde::Serialize;

/// Application-wide error type. Serializable so Tauri commands can return it
/// straight to the frontend (invoke() rejects with this shape).
#[derive(Debug, Serialize)]
pub struct AppError {
    pub message: String,
}

impl AppError {
    pub fn new(msg: impl Into<String>) -> Self {
        AppError { message: msg.into() }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for AppError {}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self { AppError::new(e.to_string()) }
}
impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self { AppError::new(e.to_string()) }
}

pub type AppResult<T> = Result<T, AppError>;
```

- [ ] **Step 2: 声明模块并编译**

`main.rs` 顶部加 `mod error;`。
Run: `cd /f/Projects/webviewApp/src-tauri && cargo build 2>&1 | tail -5`
Expected: `Finished`(未使用告警可忽略)。

- [ ] **Step 3: 提交**

```bash
git add -A && git commit -m "feat(rust): add AppError type"
```

---

## Task 3: util/string_util.rs — 字符串/时间工具(对齐 StringUtils.cpp)

**Files:**
- Create: `src-tauri/src/util/mod.rs`, `src-tauri/src/util/string_util.rs`
- Modify: `src-tauri/src/main.rs`

行为对齐 `StringUtils`:`globToRegex`(`*`→`.*`,`?`→`.`,`.`→`\.`,`\`→`\\`,`+`→`\+`,`()[]{}^$|`→前加 `\`,首尾 `^$`);`insertTempSuffix`(最后 `.` 前插 `_tmp`,无 `.` 末尾加);`removeTempSuffix`(找 `_tmp.` 删 `_tmp`);`formatFileSize`(B..TB,`>=1024` 进位,1 位小数)。

- [ ] **Step 1: 写失败测试**

`src-tauri/src/util/string_util.rs`(仅测试):
```rust
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
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd /f/Projects/webviewApp/src-tauri && cargo test string_util 2>&1 | tail -20`
Expected: 编译错误(函数未定义)。

- [ ] **Step 3: 写实现**

在测试之上加:
```rust
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
```
`src-tauri/src/util/mod.rs`:
```rust
pub mod string_util;
pub mod fs_util;
```
(若此刻 `fs_util` 未建导致编译失败,先只写 `pub mod string_util;`,Task 4 再补。)
`main.rs` 加 `mod util;`。

- [ ] **Step 4: 运行测试确认通过**

Run: `cd /f/Projects/webviewApp/src-tauri && cargo test string_util 2>&1 | tail -20`
Expected: `test result: ok. 4 passed`.

- [ ] **Step 5: 提交**

```bash
git add -A && git commit -m "feat(rust): string/size utils mirroring StringUtils"
```

---

## Task 4: util/fs_util.rs — 文件系统工具(对齐 FileUtils.cpp)

**Files:**
- Create: `src-tauri/src/util/fs_util.rs`
- Modify: `src-tauri/src/util/mod.rs`

- [ ] **Step 1: 写失败测试**

`src-tauri/src/util/fs_util.rs`:
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn list_files_recursive_finds_nested() {
        let tmp = tempfile::tempdir().unwrap();
        let sub = tmp.path().join("sub");
        std::fs::create_dir_all(&sub).unwrap();
        let mut f = std::fs::File::create(sub.join("a.mp4")).unwrap();
        f.write_all(b"hello").unwrap();
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
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd /f/Projects/webviewApp/src-tauri && cargo test fs_util 2>&1 | tail -20`
Expected: 编译错误。

- [ ] **Step 3: 写实现**

在测试之上加:
```rust
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

/// Rename a file, returning whether it succeeded. Mirrors safeRename.
pub fn safe_rename(from: &Path, to: &Path) -> bool {
    std::fs::rename(from, to).is_ok()
}

/// Whether the volume holding `path` has at least `needed` free bytes.
/// V1 stub: see Task 9 for precise fs4-based implementation.
pub fn has_enough_space(_path: &Path, _needed: u64) -> bool {
    true
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
```
确保 `mod.rs` 含 `pub mod fs_util;`。

- [ ] **Step 4: 运行测试确认通过**

Run: `cd /f/Projects/webviewApp/src-tauri && cargo test fs_util 2>&1 | tail -20`
Expected: `test result: ok. 2 passed`.

- [ ] **Step 5: 提交**

```bash
git add -A && git commit -m "feat(rust): filesystem utils mirroring FileUtils"
```

---

## Task 5: config/pattern_matcher.rs — 匹配引擎(对齐 PatternMatcher.cpp)

**Files:**
- Create: `src-tauri/src/config/mod.rs`, `src-tauri/src/config/pattern_matcher.rs`
- Modify: `src-tauri/src/main.rs`

行为对齐 `PatternMatcher`:
- include/exclude 每条 pattern:先当正则(icase)编译,失败则 `glob_to_regex` 再编译(icase);跳过空行与 `#` 开头行。
- rename rule:`pattern` 当正则编译(**非** icase,与原 `std::regex(pattern)` 一致),失败跳过;`apply_rename` 顺序 `regex_replace`(替换全部匹配)。替换语法用 `$1`/`$2`(regex crate 原生支持,与原一致)。
- `is_included`/`is_excluded`:任一规则 `regex_match`(整串匹配,用 `^...$` 锚定或 `is_match` 全匹配)。**注意**:原用 `std::regex_match`(整串匹配)。regex crate 的 `is_match` 是部分匹配,故需用锚定 `^(?:pat)$` 包裹后 `is_match`。
- `has_cycle_risk`:若原文件不 included → false;final=applyRename(orig);final==orig → true;final 被 excluded → false;否则 final 是否 included。

- [ ] **Step 1: 写失败测试**

`src-tauri/src/config/pattern_matcher.rs`(仅测试):
```rust
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
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd /f/Projects/webviewApp/src-tauri && cargo test pattern_matcher 2>&1 | tail -20`
Expected: 编译错误。

- [ ] **Step 3: 写实现**

在测试之上加:
```rust
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
```
`src-tauri/src/config/mod.rs`:
```rust
pub mod pattern_matcher;
pub mod directory_config;
pub mod template_manager;
pub mod global_config;
```
(若后续模块未建导致编译失败,先只声明 `pattern_matcher`,后续任务补。)
`main.rs` 加 `mod config;`。

- [ ] **Step 4: 运行测试确认通过**

Run: `cd /f/Projects/webviewApp/src-tauri && cargo test pattern_matcher 2>&1 | tail -20`
Expected: `test result: ok. 4 passed`.

- [ ] **Step 5: 提交**

```bash
git add -A && git commit -m "feat(rust): pattern matcher mirroring PatternMatcher"
```

---

## Task 6: config/template_manager.rs — 模板解析(对齐 TemplateManager.cpp)

**Files:**
- Create: `src-tauri/src/config/template_manager.rs`
- Modify: `src-tauri/src/config/mod.rs`

行为:`resolve(nameOrParams)` — 空→空;匹配模板 name→其 params;否则原样返回(即当作裸参数)。`names()` 返回全部模板名。模板存为 `Vec<(String,String)>`(name,params),保序。

- [ ] **Step 1: 写失败测试**

`src-tauri/src/config/template_manager.rs`(仅测试):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_by_name_or_passthrough() {
        let mut tm = TemplateManager::new();
        tm.set_templates(vec![("H.265".into(), "-c:v libx265".into())]);
        assert_eq!(tm.resolve("H.265"), "-c:v libx265");
        assert_eq!(tm.resolve("-c:v libx264"), "-c:v libx264");
        assert_eq!(tm.resolve(""), "");
    }

    #[test]
    fn names_are_ordered() {
        let mut tm = TemplateManager::new();
        tm.set_templates(vec![("a".into(), "1".into()), ("b".into(), "2".into())]);
        assert_eq!(tm.names(), vec!["a".to_string(), "b".to_string()]);
    }
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd /f/Projects/webviewApp/src-tauri && cargo test template_manager 2>&1 | tail -20`
Expected: 编译错误。

- [ ] **Step 3: 写实现**

在测试之上加:
```rust
/// Resolves template names to ffmpeg params. Mirrors TemplateManager (C++).
#[derive(Default, Clone)]
pub struct TemplateManager {
    templates: Vec<(String, String)>, // (name, params), order-preserving
}

impl TemplateManager {
    pub fn new() -> Self {
        TemplateManager::default()
    }

    pub fn set_templates(&mut self, tmpls: Vec<(String, String)>) {
        self.templates = tmpls;
    }

    /// Empty → empty; matching name → its params; otherwise return input verbatim.
    pub fn resolve(&self, name_or_params: &str) -> String {
        if name_or_params.is_empty() {
            return String::new();
        }
        for (name, params) in &self.templates {
            if name == name_or_params {
                return params.clone();
            }
        }
        name_or_params.to_string()
    }

    pub fn names(&self) -> Vec<String> {
        self.templates.iter().map(|(n, _)| n.clone()).collect()
    }
}
```
确保 `mod.rs` 含 `pub mod template_manager;`。

- [ ] **Step 4: 运行测试确认通过**

Run: `cd /f/Projects/webviewApp/src-tauri && cargo test template_manager 2>&1 | tail -20`
Expected: `test result: ok. 2 passed`.

- [ ] **Step 5: 提交**

```bash
git add -A && git commit -m "feat(rust): template manager mirroring TemplateManager"
```

---

## Task 7: config/directory_config.rs — 目录级配置(对齐 DirectoryConfig.cpp)

**Files:**
- Create: `src-tauri/src/config/directory_config.rs`
- Modify: `src-tauri/src/config/mod.rs`

行为对齐 `DirectoryConfig`。要点:
- `CONFIG_FILENAME = ".autocompress/config.json"`;`config_path(dir) = dir/.autocompress/config.json`。
- `load`:文件不存在 → `valid=false`,error "Config file not found: ..."。存在则解析:
  - `include` 必须是非空数组,否则 `valid=false` error `"'include' must be a non-empty array"`。
  - `exclude` 可选数组;`filters` 可选(`max_size_mb`/`min_size_mb` 为 MB 浮点 → 存字节 `u64`;`mtime_after`/`mtime_before`/`ctime_after`/`ctime_before` 为字符串)。
  - `rename_rules` 可选数组 of `{pattern,replacement}`。
  - `params` 可选字符串。
  - `schedule.time` 可选字符串。
  - 成功 → `valid=true`,编译 matcher。JSON 解析异常 → `valid=false` error `"JSON parse error: ..."`。
- `create_default(dir, params)`:见计划头默认值,写盘,返回。
- `save`:仅写非空字段(exclude 空则不写;filters 空对象则不写;等)。size 字节→MB 浮点写回。缩进 4 空格 + 结尾换行(与原 `dump(4)+endl` 一致)。
- `passes_filters(rel, size, mtime, ctime)`:matcher.is_included && !is_excluded;min/max size;mtime/ctime after/before。日期解析 `YYYY-MM-DD`(本地时区午夜)。`*_before` 语义:`sysTime >= bound + 24h` → 排除(即含当天)。

日期与时间用 `chrono::NaiveDate` 解析,转本地时区 `Local.from_local_datetime`。文件时间 `SystemTime` → `chrono::DateTime<Local>` 比较。

- [ ] **Step 1: 写失败测试**

`src-tauri/src/config/directory_config.rs`(仅测试):
```rust
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
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd /f/Projects/webviewApp/src-tauri && cargo test directory_config 2>&1 | tail -20`
Expected: 编译错误。

- [ ] **Step 3: 写实现**

在测试之上加:
```rust
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
    pub mtime_after: Option<String>,
    pub mtime_before: Option<String>,
    pub ctime_after: Option<String>,
    pub ctime_before: Option<String>,
    pub params: String,
    pub schedule_time: Option<String>,
    pub matcher: PatternMatcher,
}

const CONFIG_FILENAME_REL: &str = ".autocompress/config.json";

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
```
确保 `mod.rs` 含 `pub mod directory_config;`。

- [ ] **Step 4: 运行测试确认通过**

Run: `cd /f/Projects/webviewApp/src-tauri && cargo test directory_config 2>&1 | tail -20`
Expected: `test result: ok. 5 passed`.

- [ ] **Step 5: 提交**

```bash
git add -A && git commit -m "feat(rust): directory config mirroring DirectoryConfig"
```

---

## Task 8: config/global_config.rs — 全局配置(对齐 ConfigManager.cpp,扁平格式)

**Files:**
- Create: `src-tauri/src/config/global_config.rs`
- Modify: `src-tauri/src/config/mod.rs`

行为对齐 `ConfigManager`(**扁平磁盘格式**,见计划头):
- 默认模板 3 条(见计划头)。
- `load`:读 `%APPDATA%/AutoCompress/config.json`;不存在 → 写默认并返回 true;解析失败 → false(不覆盖内存默认)。字段用 `.value(key, default)` 语义(缺失用默认)。
- `save`:写扁平 JSON,缩进 4。
- `add_directory(path)`:空→忽略;规范化路径去重(不区分大小写);默认 enabled=true。
- `remove_directory(index)` / `set_enabled(index,bool)`。
- `detect_overlaps() -> Vec<bool>`:后出现的目录若与前面任一构成父子包含 → 标记 true。
- 路径规范化:`std::fs::canonicalize` 不适合(要求存在且返回 `\\?\` 前缀);改用手写规范化:转小写 + 统一 `/` 分隔 + 去尾斜杠。父子判断:child 以 parent 开头且下一个字符是分隔符。

- [ ] **Step 1: 写失败测试**

`src-tauri/src/config/global_config.rs`(仅测试):
```rust
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
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd /f/Projects/webviewApp/src-tauri && cargo test global_config 2>&1 | tail -20`
Expected: 编译错误。

- [ ] **Step 3: 写实现**

在测试之上加:
```rust
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
```
确保 `mod.rs` 含 `pub mod global_config;`。

- [ ] **Step 4: 运行测试确认通过**

Run: `cd /f/Projects/webviewApp/src-tauri && cargo test global_config 2>&1 | tail -20`
Expected: `test result: ok. 4 passed`.

- [ ] **Step 5: 提交**

```bash
git add -A && git commit -m "feat(rust): global config mirroring ConfigManager (flat format)"
```

---

## Task 9: types.rs — 共享类型(对齐 Types.h)+ 磁盘空间精确化

**Files:**
- Create: `src-tauri/src/types.rs`
- Modify: `src-tauri/src/main.rs`, `src-tauri/src/util/fs_util.rs`, `src-tauri/Cargo.toml`

- [ ] **Step 1: 加 fs4 crate 精确磁盘空间(替换 Task 4 stub)**

`src-tauri/Cargo.toml` `[dependencies]` 加:
```toml
fs4 = "0.8"
```
把 `fs_util.rs` 的 `has_enough_space` 替换为:
```rust
/// Whether the volume holding `path` has at least `needed` free bytes.
/// Fail-closed: false when it can't be determined. Mirrors hasEnoughSpace.
pub fn has_enough_space(path: &Path, needed: u64) -> bool {
    match fs4::available_space(path) {
        Ok(free) => free >= needed,
        Err(_) => false,
    }
}
```

- [ ] **Step 2: 写 types.rs(无独立测试;编译即验证)**

`src-tauri/src/types.rs`:
```rust
use serde::{Deserialize, Serialize};

/// Per-file compression status. Mirrors FileStatus (C++).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileStatus {
    Success,
    SkippedLarger,
    Failed,
    SkippedOther,
}

/// Per-file result. Mirrors FileResult (C++).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileResult {
    pub name: String,
    pub path: String,
    pub final_name: String,
    pub final_path: String,
    pub status: FileStatus,
    pub original_size: u64,
    pub compressed_size: u64,
    pub saved_bytes: i64,
    pub ffmpeg_exit_code: i32,
    pub ffmpeg_duration_ms: i32,
    pub cycle_risk: bool,
}

impl Default for FileResult {
    fn default() -> Self {
        FileResult {
            name: String::new(), path: String::new(),
            final_name: String::new(), final_path: String::new(),
            status: FileStatus::SkippedOther,
            original_size: 0, compressed_size: 0, saved_bytes: 0,
            ffmpeg_exit_code: -1, ffmpeg_duration_ms: 0, cycle_risk: false,
        }
    }
}

/// Directory summary within a run. Mirrors DirectoryResult (C++).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryResult {
    pub path: String,
    pub config_valid: bool,
    pub files_total: i32,
    pub files_processed: i32,
}

/// Full run summary. Mirrors RunSummary (C++).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunSummary {
    pub run_id: String,
    pub start_time: String,
    pub end_time: String,
    pub duration_seconds: i32,
    pub directories: Vec<DirectoryResult>,
    pub files: Vec<FileResult>,
    pub success_count: i32,
    pub skipped_larger_count: i32,
    pub failed_count: i32,
    pub skipped_other_count: i32,
    pub total_saved_bytes: i64,
    pub cycle_risk_count: i32,
}

impl RunSummary {
    /// Mirrors RunSummary::computeTotals.
    pub fn compute_totals(&mut self) {
        self.success_count = 0;
        self.skipped_larger_count = 0;
        self.failed_count = 0;
        self.skipped_other_count = 0;
        self.total_saved_bytes = 0;
        self.cycle_risk_count = 0;
        for f in &self.files {
            match f.status {
                FileStatus::Success => self.success_count += 1,
                FileStatus::SkippedLarger => self.skipped_larger_count += 1,
                FileStatus::Failed => self.failed_count += 1,
                FileStatus::SkippedOther => self.skipped_other_count += 1,
            }
            self.total_saved_bytes += f.saved_bytes;
            if f.cycle_risk { self.cycle_risk_count += 1; }
        }
    }
}

/// Runtime stage of a directory. Mirrors DirRuntimeState::Stage.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Stage { Idle, Scanning, Compressing, Completed }

/// Per-directory runtime state pushed to the UI. Mirrors DirRuntimeState.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirRuntimeState {
    pub dir_path: String,
    pub stage: Stage,
    pub status_text: String,
    pub current_file: String,
    pub completed_files: i32,
    pub total_files: i32,
    pub last_run_time: String,
    pub last_run_result: String,
    pub next_run_time: String,
}

impl DirRuntimeState {
    pub fn new(dir_path: String) -> Self {
        DirRuntimeState {
            dir_path, stage: Stage::Idle, status_text: String::new(),
            current_file: String::new(), completed_files: 0, total_files: 0,
            last_run_time: String::new(), last_run_result: String::new(),
            next_run_time: String::new(),
        }
    }
}

/// Card info for the directory list (level 1 UI aggregate).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirCardInfo {
    pub path: String,
    pub enabled: bool,
    pub badge: String,          // "valid" | "unscheduled" | "invalid" | "overlap" | "config_error"
    pub badge_detail: String,   // e.g. overlap owner path, or error line
    pub file_count: i32,
    pub total_size: u64,
    pub params_name: String,
    pub cycle_risk_count: i32,
    pub last_run_time: String,
    pub last_run_result: String,
    pub next_run_time: String,
}

/// A single file in the preview table.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilePreview {
    pub relative_path: String,
    pub final_name: String,
    pub file_size: u64,
    pub cycle_risk: bool,
}

/// ffmpeg availability result. Mirrors FfmpegProbe.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FfmpegStatus {
    pub ready: bool,
    pub version: String,
    pub error: String,
}

/// Directory-level config as a form view (for TabConfig).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirConfigView {
    pub exists: bool,
    pub valid: bool,
    pub error_message: String,
    pub include: Vec<String>,
    pub exclude: Vec<String>,
    pub max_size_mb: Option<f64>,
    pub min_size_mb: Option<f64>,
    pub mtime_after: Option<String>,
    pub mtime_before: Option<String>,
    pub ctime_after: Option<String>,
    pub ctime_before: Option<String>,
    pub rename_rules: Vec<RenameRuleView>,
    pub params: String,
    pub schedule_time: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenameRuleView {
    pub pattern: String,
    pub replacement: String,
}
```
`main.rs` 加 `mod types;`。

- [ ] **Step 3: 编译**

Run: `cd /f/Projects/webviewApp/src-tauri && cargo build 2>&1 | tail -10`
Expected: `Finished`.

- [ ] **Step 4: 提交**

```bash
git add -A && git commit -m "feat(rust): shared types mirroring Types.h + precise disk space check"
```

---

## Task 10: scanner.rs — 文件扫描(对齐 FileScanner.cpp)

**Files:**
- Create: `src-tauri/src/scanner.rs`
- Modify: `src-tauri/src/main.rs`

行为对齐 `FileScanner`:config 无效或非目录 → 空。对每个文件计算相对路径(相对根目录,`\`→`/`),`passes_filters` 过滤;通过则产出 `ScanFile`(relativePath, absolutePath, fileSize, mtime, ctime, tempName=insert_temp_suffix(rel), finalName=matcher.apply_rename(rel), cycleRisk=matcher.has_cycle_risk(rel))。

- [ ] **Step 1: 写失败测试**

`src-tauri/src/scanner.rs`(仅测试):
```rust
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
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd /f/Projects/webviewApp/src-tauri && cargo test scanner 2>&1 | tail -20`
Expected: 编译错误。

- [ ] **Step 3: 写实现**

在测试之上加:
```rust
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
```
`main.rs` 加 `mod scanner;`。

- [ ] **Step 4: 运行测试确认通过**

Run: `cd /f/Projects/webviewApp/src-tauri && cargo test scanner 2>&1 | tail -20`
Expected: `test result: ok. 2 passed`.

- [ ] **Step 5: 提交**

```bash
git add -A && git commit -m "feat(rust): file scanner mirroring FileScanner"
```

---

## Task 11: scheduler.rs — 时间表调度(对齐 Scheduler.cpp)

**Files:**
- Create: `src-tauri/src/scheduler.rs`
- Modify: `src-tauri/src/main.rs`

行为对齐 `Scheduler`:
- `compute_next_run(now, hour, minute)`(纯函数):今天 HH:MM;若 `<= now` 则明天同点。用 `chrono::Local`。
- 内部 `Vec<DirSchedule>{dir_path, enabled, next_run}`,`Mutex` 保护。`Arc` 共享给后台线程。
- `set_directories(Vec<DirSchedule>)` 替换时间表。
- `seconds_until_next_run(path)`:未排程/未找到 → -1;否则 `max(0, next-now)` 秒。
- `next_run_time(path)`:未排程/未找到 → None。
- `mark_completed(path)`:该目录 next_run += 24h。
- `start(callback)`:后台线程每秒扫描,找第一个 `enabled && now>=next_run` 调 `callback(dir)`(不在此推进 next_run,由 App 侧 mark_completed 推进)。`stop()` 结束线程。

因纯函数 `compute_next_run` 可测,轮询线程行为用集成方式验证(见测试)。

- [ ] **Step 1: 写失败测试**

`src-tauri/src/scheduler.rs`(仅测试):
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Local, Timelike, Duration};

    #[test]
    fn compute_next_run_future_today() {
        let now = Local::now();
        // pick a minute one hour ahead → still today
        let target = now + Duration::hours(1);
        let next = compute_next_run(now, target.hour(), target.minute());
        assert!(next > now);
        // within ~1 day
        assert!(next <= now + Duration::hours(25));
    }

    #[test]
    fn compute_next_run_past_rolls_tomorrow() {
        let now = Local::now();
        let past = now - Duration::hours(1);
        let next = compute_next_run(now, past.hour(), past.minute());
        assert!(next > now);
    }

    #[test]
    fn seconds_and_mark_completed() {
        let sched = Scheduler::new();
        let now = Local::now();
        let future = now + Duration::hours(2);
        sched.set_directories(vec![DirSchedule {
            dir_path: "D:/x".into(),
            enabled: true,
            next_run: future,
        }]);
        let secs = sched.seconds_until_next_run("D:/x");
        assert!(secs > 0);
        assert_eq!(sched.seconds_until_next_run("D:/missing"), -1);
        sched.mark_completed("D:/x");
        let after = sched.next_run_time("D:/x").unwrap();
        assert!(after > future);
    }
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd /f/Projects/webviewApp/src-tauri && cargo test scheduler 2>&1 | tail -20`
Expected: 编译错误。

- [ ] **Step 3: 写实现**

在测试之上加:
```rust
use chrono::{DateTime, Duration, Local, TimeZone, Timelike};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

/// One directory's schedule entry. Mirrors DirSchedule (C++).
#[derive(Clone)]
pub struct DirSchedule {
    pub dir_path: String,
    pub enabled: bool,
    pub next_run: DateTime<Local>,
}

/// Per-directory timetable poller. Mirrors Scheduler (C++).
pub struct Scheduler {
    schedules: Arc<Mutex<Vec<DirSchedule>>>,
    running: Arc<AtomicBool>,
    handle: Mutex<Option<JoinHandle<()>>>,
}

/// Pure: next run for HH:MM relative to `now`. Mirrors computeNextRun.
pub fn compute_next_run(now: DateTime<Local>, hour: u32, minute: u32) -> DateTime<Local> {
    let today = now.date_naive().and_hms_opt(hour, minute, 0).unwrap();
    let mut target = Local.from_local_datetime(&today).single().unwrap_or(now);
    if target <= now {
        target = target + Duration::hours(24);
    }
    target
}

impl Scheduler {
    pub fn new() -> Self {
        Scheduler {
            schedules: Arc::new(Mutex::new(Vec::new())),
            running: Arc::new(AtomicBool::new(false)),
            handle: Mutex::new(None),
        }
    }

    pub fn set_directories(&self, dirs: Vec<DirSchedule>) {
        *self.schedules.lock().unwrap() = dirs;
    }

    pub fn seconds_until_next_run(&self, dir_path: &str) -> i64 {
        let guard = self.schedules.lock().unwrap();
        for s in guard.iter() {
            if s.dir_path == dir_path {
                if !s.enabled { return -1; }
                let diff = (s.next_run - Local::now()).num_seconds();
                return diff.max(0);
            }
        }
        -1
    }

    pub fn next_run_time(&self, dir_path: &str) -> Option<DateTime<Local>> {
        let guard = self.schedules.lock().unwrap();
        for s in guard.iter() {
            if s.dir_path == dir_path {
                if !s.enabled { return None; }
                return Some(s.next_run);
            }
        }
        None
    }

    pub fn mark_completed(&self, dir_path: &str) {
        let mut guard = self.schedules.lock().unwrap();
        for s in guard.iter_mut() {
            if s.dir_path == dir_path {
                s.next_run = s.next_run + Duration::hours(24);
                break;
            }
        }
    }

    /// Start the background poller. `cb` is called with the directory to run.
    pub fn start<F>(&self, cb: F)
    where
        F: Fn(String) + Send + 'static,
    {
        self.running.store(true, Ordering::SeqCst);
        let schedules = Arc::clone(&self.schedules);
        let running = Arc::clone(&self.running);
        let handle = thread::spawn(move || {
            while running.load(Ordering::SeqCst) {
                let mut to_run: Option<String> = None;
                {
                    let now = Local::now();
                    let guard = schedules.lock().unwrap();
                    for s in guard.iter() {
                        if s.enabled && now >= s.next_run {
                            to_run = Some(s.dir_path.clone());
                            break;
                        }
                    }
                }
                if let Some(dir) = to_run {
                    cb(dir);
                }
                thread::sleep(std::time::Duration::from_secs(1));
            }
        });
        *self.handle.lock().unwrap() = Some(handle);
    }

    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(h) = self.handle.lock().unwrap().take() {
            let _ = h.join();
        }
    }
}

impl Default for Scheduler {
    fn default() -> Self { Scheduler::new() }
}
```
`main.rs` 加 `mod scheduler;`。

- [ ] **Step 4: 运行测试确认通过**

Run: `cd /f/Projects/webviewApp/src-tauri && cargo test scheduler 2>&1 | tail -20`
Expected: `test result: ok. 3 passed`.

- [ ] **Step 5: 提交**

```bash
git add -A && git commit -m "feat(rust): scheduler mirroring Scheduler"
```

---

## Task 12: logger.rs — per-directory 日志(对齐 Logger.cpp)

**Files:**
- Create: `src-tauri/src/logger.rs`
- Modify: `src-tauri/src/main.rs`

行为对齐 `Logger`(per-directory,写 `<dir>/.autocompress/logs/run_*.log`):
- `Logger::new(dir)` → log_dir = `<dir>/.autocompress/logs`。
- `begin_run(&initial_summary)`:建目录;run_id=`now_for_filename()`(`%Y-%m-%d_%H-%M-%S`,本地);写 header(分隔线、Start time=ISO、Directories 列表,无效配置标 `(无效配置)`)。
- `log_file_result(&r)`:缓冲一行(✅/⏭/❌/ℹ,文案见原文)。
- `finalize_run(&summary)`:刷缓冲行 + `--- Summary ---` 文本块 + `--- JSON ---`/`--- JSON END ---` 包住 JSON(缩进 2)。JSON key 为 snake_case:`version,run_id,start_time,end_time,duration_seconds,directory_count,directories(map by path),files[],summary`。file 的 status 字符串 `success/skipped_larger/failed/skipped_other`,failed 时含 `ffmpeg_exit_code`。
- `clean_old_logs(retention_days)`:删 `run_*.log` 中 mtime 超 `retention_days*24h` 的。retention<=0 不清。
- 历史解析 `read_history(dir) -> Vec<RunSummary>`:遍历 `run_*.log`,提取 `--- JSON ---`..`--- JSON END ---` 之间内容解析为 RunSummary(前端历史 tab 用),按文件名倒序。

ISO 时间戳格式 `%Y-%m-%dT%H:%M:%S%z`(对齐 `nowToString`)。

- [ ] **Step 1: 写失败测试**

`src-tauri/src/logger.rs`(仅测试):
```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{RunSummary, DirectoryResult, FileResult, FileStatus};

    fn sample_summary() -> RunSummary {
        let mut s = RunSummary::default();
        s.run_id = "2026-01-01_00-00-00".into();
        s.start_time = "2026-01-01T00:00:00+0800".into();
        s.end_time = s.start_time.clone();
        s.directories.push(DirectoryResult {
            path: "D:/x".into(), config_valid: true, files_total: 1, files_processed: 1,
        });
        s.files.push(FileResult {
            name: "a.mp4".into(), status: FileStatus::Success,
            original_size: 100, compressed_size: 40, saved_bytes: 60, ..Default::default()
        });
        s.compute_totals();
        s
    }

    #[test]
    fn write_and_read_history_roundtrip() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_str().unwrap();
        let mut logger = Logger::new(dir);
        let s = sample_summary();
        assert!(logger.begin_run(&s));
        logger.log_file_result(&s.files[0]);
        logger.finalize_run(&s);

        let hist = read_history(dir);
        assert_eq!(hist.len(), 1);
        assert_eq!(hist[0].success_count, 1);
        assert_eq!(hist[0].total_saved_bytes, 60);
    }

    #[test]
    fn clean_old_logs_keeps_recent() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_str().unwrap();
        let mut logger = Logger::new(dir);
        let s = sample_summary();
        logger.begin_run(&s);
        logger.finalize_run(&s);
        logger.clean_old_logs(90); // recent file, retention 90d → kept
        assert_eq!(read_history(dir).len(), 1);
    }
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd /f/Projects/webviewApp/src-tauri && cargo test logger 2>&1 | tail -20`
Expected: 编译错误。

- [ ] **Step 3: 写实现**

在测试之上加:
```rust
use crate::types::{FileStatus, RunSummary};
use crate::util::string_util::format_file_size;
use chrono::Local;
use serde_json::Value;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::PathBuf;

/// Per-directory logger writing <dir>/.autocompress/logs/run_*.log. Mirrors Logger.
pub struct Logger {
    log_dir: PathBuf,
    run_id: String,
    current_log_path: PathBuf,
    pending_lines: Vec<String>,
}

/// ISO8601 local timestamp. Mirrors StringUtils::nowToString.
fn now_iso() -> String {
    Local::now().format("%Y-%m-%dT%H:%M:%S%z").to_string()
}

/// Filename-safe timestamp. Mirrors StringUtils::nowForFilename.
pub fn now_for_filename() -> String {
    Local::now().format("%Y-%m-%d_%H-%M-%S").to_string()
}

impl Logger {
    pub fn new(dir: &str) -> Self {
        Logger {
            log_dir: PathBuf::from(dir).join(".autocompress").join("logs"),
            run_id: String::new(),
            current_log_path: PathBuf::new(),
            pending_lines: Vec::new(),
        }
    }

    pub fn begin_run(&mut self, initial: &RunSummary) -> bool {
        let _ = std::fs::create_dir_all(&self.log_dir);
        self.run_id = now_for_filename();
        self.current_log_path = self.log_dir.join(format!("run_{}.log", self.run_id));
        let mut file = match std::fs::File::create(&self.current_log_path) {
            Ok(f) => f,
            Err(_) => return false,
        };
        let mut header = String::new();
        header.push_str("========================================\n");
        header.push_str("AutoCompress Run Log\n");
        header.push_str(&format!("Start time: {}\n", now_iso()));
        header.push_str("Directories:\n");
        for d in &initial.directories {
            let suffix = if d.config_valid { "" } else { " (无效配置)" };
            header.push_str(&format!("  - {}{}\n", d.path, suffix));
        }
        header.push_str("========================================\n\n");
        file.write_all(header.as_bytes()).is_ok()
    }

    pub fn log_file_result(&mut self, r: &crate::types::FileResult) {
        let line = match r.status {
            FileStatus::Success => format!(
                "✅ {} ({} → {}, 节省 {})",
                r.name, format_file_size(r.original_size as i64),
                format_file_size(r.compressed_size as i64),
                format_file_size(r.saved_bytes)
            ),
            FileStatus::SkippedLarger => format!(
                "⏭ {} 压缩后更大 ({} → {})，已丢弃",
                r.name, format_file_size(r.original_size as i64),
                format_file_size(r.compressed_size as i64)
            ),
            FileStatus::Failed => format!(
                "❌ {} 压缩失败 (退出码: {})", r.name, r.ffmpeg_exit_code
            ),
            FileStatus::SkippedOther => format!("ℹ {} 跳过", r.name),
        };
        self.pending_lines.push(line);
    }

    pub fn finalize_run(&mut self, summary: &RunSummary) {
        if self.current_log_path.as_os_str().is_empty() {
            return;
        }
        let mut file = match OpenOptions::new().append(true).open(&self.current_log_path) {
            Ok(f) => f,
            Err(_) => return,
        };
        let mut body = String::new();
        for line in &self.pending_lines {
            body.push_str(line);
            body.push('\n');
        }
        self.pending_lines.clear();
        body.push_str("\n--- Summary ---\n");
        body.push_str(&format!("Total files: {}\n", summary.files.len()));
        body.push_str(&format!("Successful: {}\n", summary.success_count));
        body.push_str(&format!("Skipped (larger): {}\n", summary.skipped_larger_count));
        body.push_str(&format!("Failed: {}\n", summary.failed_count));
        body.push_str(&format!("Skipped (other): {}\n", summary.skipped_other_count));
        body.push_str(&format!("Total saved: {}\n", format_file_size(summary.total_saved_bytes)));
        body.push_str(&format!("Cycle risk files: {}\n", summary.cycle_risk_count));
        body.push_str(&format!("Duration: {} seconds\n", summary.duration_seconds));
        body.push_str("\n--- JSON ---\n");
        body.push_str(&summary_to_json_block(&self.run_id, summary));
        body.push_str("\n--- JSON END ---\n");
        let _ = file.write_all(body.as_bytes());
    }

    pub fn clean_old_logs(&self, retention_days: i64) {
        if retention_days <= 0 { return; }
        let entries = match std::fs::read_dir(&self.log_dir) {
            Ok(e) => e,
            Err(_) => return,
        };
        let now = std::time::SystemTime::now();
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if !name.starts_with("run_") || path.extension().and_then(|s| s.to_str()) != Some("log") {
                continue;
            }
            if let Ok(meta) = entry.metadata() {
                if let Ok(modified) = meta.modified() {
                    if let Ok(age) = now.duration_since(modified) {
                        if age.as_secs() as i64 >= retention_days * 24 * 3600 {
                            let _ = std::fs::remove_file(&path);
                        }
                    }
                }
            }
        }
    }

    pub fn log_directory(&self) -> String {
        self.log_dir.to_string_lossy().to_string()
    }
}

/// Machine-readable JSON block. Mirrors Logger::summaryToJsonBlock (snake_case keys).
fn summary_to_json_block(run_id: &str, summary: &RunSummary) -> String {
    let mut j = serde_json::Map::new();
    j.insert("version".into(), Value::from(1));
    j.insert("run_id".into(), Value::from(run_id.to_string()));
    j.insert("start_time".into(), Value::from(summary.start_time.clone()));
    j.insert("end_time".into(), Value::from(summary.end_time.clone()));
    j.insert("duration_seconds".into(), Value::from(summary.duration_seconds));
    j.insert("directory_count".into(), Value::from(summary.directories.len() as i64));

    let mut dirs = serde_json::Map::new();
    for d in &summary.directories {
        let mut dm = serde_json::Map::new();
        dm.insert("path".into(), Value::from(d.path.clone()));
        dm.insert("config_valid".into(), Value::from(d.config_valid));
        dm.insert("files_total".into(), Value::from(d.files_total));
        dm.insert("files_processed".into(), Value::from(d.files_processed));
        dirs.insert(d.path.clone(), Value::Object(dm));
    }
    j.insert("directories".into(), Value::Object(dirs));

    let files: Vec<Value> = summary.files.iter().map(|f| {
        let mut fm = serde_json::Map::new();
        fm.insert("name".into(), Value::from(f.name.clone()));
        fm.insert("path".into(), Value::from(f.path.clone()));
        fm.insert("final_name".into(), Value::from(f.final_name.clone()));
        fm.insert("final_path".into(), Value::from(f.final_path.clone()));
        fm.insert("original_size".into(), Value::from(f.original_size));
        fm.insert("compressed_size".into(), Value::from(f.compressed_size));
        fm.insert("saved_bytes".into(), Value::from(f.saved_bytes));
        fm.insert("cycle_risk".into(), Value::from(f.cycle_risk));
        let status = match f.status {
            FileStatus::Success => "success",
            FileStatus::SkippedLarger => "skipped_larger",
            FileStatus::Failed => "failed",
            FileStatus::SkippedOther => "skipped_other",
        };
        fm.insert("status".into(), Value::from(status));
        if f.status == FileStatus::Failed {
            fm.insert("ffmpeg_exit_code".into(), Value::from(f.ffmpeg_exit_code));
        }
        Value::Object(fm)
    }).collect();
    j.insert("files".into(), Value::from(files));

    let mut s = serde_json::Map::new();
    s.insert("success".into(), Value::from(summary.success_count));
    s.insert("skipped_larger".into(), Value::from(summary.skipped_larger_count));
    s.insert("failed".into(), Value::from(summary.failed_count));
    s.insert("skipped_other".into(), Value::from(summary.skipped_other_count));
    s.insert("total_saved_bytes".into(), Value::from(summary.total_saved_bytes));
    s.insert("cycle_risk_count".into(), Value::from(summary.cycle_risk_count));
    j.insert("summary".into(), Value::Object(s));

    serde_json::to_string_pretty(&Value::Object(j)).unwrap_or_default()
}

/// Parse all run logs in <dir>, newest first. Extracts each JSON block into a RunSummary.
pub fn read_history(dir: &str) -> Vec<RunSummary> {
    let log_dir = PathBuf::from(dir).join(".autocompress").join("logs");
    let mut names: Vec<PathBuf> = match std::fs::read_dir(&log_dir) {
        Ok(e) => e.filter_map(|x| x.ok()).map(|x| x.path())
            .filter(|p| {
                let n = p.file_name().and_then(|s| s.to_str()).unwrap_or("");
                n.starts_with("run_") && p.extension().and_then(|s| s.to_str()) == Some("log")
            }).collect(),
        Err(_) => return Vec::new(),
    };
    names.sort();
    names.reverse(); // newest first (filename timestamp sorts chronologically)

    let mut out = Vec::new();
    for path in names {
        let text = match std::fs::read_to_string(&path) { Ok(t) => t, Err(_) => continue };
        if let Some(json) = extract_json_block(&text) {
            if let Some(summary) = parse_summary_json(&json) {
                out.push(summary);
            }
        }
    }
    out
}

fn extract_json_block(text: &str) -> Option<String> {
    let start = text.find("--- JSON ---")? + "--- JSON ---".len();
    let end = text[start..].find("--- JSON END ---")? + start;
    Some(text[start..end].trim().to_string())
}

fn parse_summary_json(json: &str) -> Option<RunSummary> {
    let v: Value = serde_json::from_str(json).ok()?;
    let mut s = RunSummary::default();
    s.run_id = v.get("run_id").and_then(|x| x.as_str()).unwrap_or("").to_string();
    s.start_time = v.get("start_time").and_then(|x| x.as_str()).unwrap_or("").to_string();
    s.end_time = v.get("end_time").and_then(|x| x.as_str()).unwrap_or("").to_string();
    s.duration_seconds = v.get("duration_seconds").and_then(|x| x.as_i64()).unwrap_or(0) as i32;
    if let Some(sum) = v.get("summary") {
        s.success_count = sum.get("success").and_then(|x| x.as_i64()).unwrap_or(0) as i32;
        s.skipped_larger_count = sum.get("skipped_larger").and_then(|x| x.as_i64()).unwrap_or(0) as i32;
        s.failed_count = sum.get("failed").and_then(|x| x.as_i64()).unwrap_or(0) as i32;
        s.skipped_other_count = sum.get("skipped_other").and_then(|x| x.as_i64()).unwrap_or(0) as i32;
        s.total_saved_bytes = sum.get("total_saved_bytes").and_then(|x| x.as_i64()).unwrap_or(0);
        s.cycle_risk_count = sum.get("cycle_risk_count").and_then(|x| x.as_i64()).unwrap_or(0) as i32;
    }
    // files (for expandable detail in history tab)
    if let Some(Value::Array(arr)) = v.get("files") {
        for f in arr {
            let mut fr = crate::types::FileResult::default();
            fr.name = f.get("name").and_then(|x| x.as_str()).unwrap_or("").to_string();
            fr.final_name = f.get("final_name").and_then(|x| x.as_str()).unwrap_or("").to_string();
            fr.original_size = f.get("original_size").and_then(|x| x.as_u64()).unwrap_or(0);
            fr.compressed_size = f.get("compressed_size").and_then(|x| x.as_u64()).unwrap_or(0);
            fr.saved_bytes = f.get("saved_bytes").and_then(|x| x.as_i64()).unwrap_or(0);
            fr.cycle_risk = f.get("cycle_risk").and_then(|x| x.as_bool()).unwrap_or(false);
            fr.status = match f.get("status").and_then(|x| x.as_str()).unwrap_or("") {
                "success" => FileStatus::Success,
                "skipped_larger" => FileStatus::SkippedLarger,
                "failed" => FileStatus::Failed,
                _ => FileStatus::SkippedOther,
            };
            s.files.push(fr);
        }
    }
    Some(s)
}
```
`main.rs` 加 `mod logger;`。

- [ ] **Step 4: 运行测试确认通过**

Run: `cd /f/Projects/webviewApp/src-tauri && cargo test logger 2>&1 | tail -20`
Expected: `test result: ok. 2 passed`.

- [ ] **Step 5: 提交**

```bash
git add -A && git commit -m "feat(rust): per-directory logger mirroring Logger + history parse"
```

---

## Task 13: compressor/file_compare.rs — 文件比较清理(对齐 FileCompare.cpp)

**Files:**
- Create: `src-tauri/src/compressor/mod.rs`, `src-tauri/src/compressor/file_compare.rs`
- Modify: `src-tauri/src/main.rs`

行为对齐 `FileCompare::compareAndCleanup`:
- 入参:original_path, original_size, compressed_path(tmp), compressed_size, matcher, ffmpeg_exit_code, ffmpeg_duration_ms。
- result.name = 原文件名;final_name = matcher.apply_rename(name);final_path = parent/final_name。
- exit_code != 0 → Failed,删 tmp,返回。
- compressed_size < original_size → Success,saved=orig-comp;删原;tmp 去 `_tmp` 后缀改名;若 clean != target(final_name)再改名到 target;final_path=target。
- 否则 → SkippedLarger,saved= -(comp-orig),删 tmp。

- [ ] **Step 1: 写失败测试**

`src-tauri/src/compressor/file_compare.rs`(仅测试):
```rust
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
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd /f/Projects/webviewApp/src-tauri && cargo test file_compare 2>&1 | tail -20`
Expected: 编译错误。

- [ ] **Step 3: 写实现**

在测试之上加:
```rust
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
```
`src-tauri/src/compressor/mod.rs`:
```rust
pub mod file_compare;
pub mod engine;
```
(engine 在 Task 14 建;若编译失败先只声明 `file_compare`。)
`main.rs` 加 `mod compressor;`。

- [ ] **Step 4: 运行测试确认通过**

Run: `cd /f/Projects/webviewApp/src-tauri && cargo test file_compare 2>&1 | tail -20`
Expected: `test result: ok. 3 passed`.

- [ ] **Step 5: 提交**

```bash
git add -A && git commit -m "feat(rust): file compare/cleanup mirroring FileCompare"
```

---

## Task 14: compressor/engine.rs — ffmpeg 子进程(对齐 CompressionEngine.cpp)

**Files:**
- Create: `src-tauri/src/compressor/engine.rs`
- Modify: `src-tauri/src/compressor/mod.rs`

行为对齐 `CompressionEngine`:
- `compress(params) -> CompressResult{success, exit_code, duration_ms, error_message}`:
  - 命令 `ffmpeg -i <input> <args> -y <output>`。
  - Windows 下 `CREATE_NO_WINDOW`(用 `std::os::windows::process::CommandExt::creation_flags(0x08000000)`)。
  - 超时:spawn child,轮询 `try_wait` 直到 `timeout_seconds`;超时 `kill()`,error "压缩超时 (Ns)"。
  - 正常退出:exit_code=退出码,success=(code==0),非 0 时 error "ffmpeg 退出码: N"。
- `probe_ffmpeg(path) -> FfmpegStatus`:path 空 → error "未配置 ffmpeg 路径";若含分隔符且文件不存在 → error "ffmpeg 路径不存在: ...";否则跑 `<path> -version`(捕获 stdout+stderr,5s 超时),exit!=0 → error;找 "version" 后第一个空白分隔 token 作 version;为空 → error "version 为空";否则 ready=true。

超时轮询用 `child.try_wait()` + `sleep(100ms)` 循环。

- [ ] **Step 1: 写失败测试(用系统命令模拟,不依赖真实 ffmpeg)**

`src-tauri/src/compressor/engine.rs`(仅测试;用 `cmd /C` 构造可控进程):
```rust
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
        // Use cmd.exe as a stand-in "ffmpeg" that exits 0 and writes output.
        // buildCommand wraps: <ffmpeg> -i <in> <args> -y <out>. We instead call
        // run_process directly to validate exit-code capture.
        let out = run_process("cmd", &["/C", "exit", "0"], 5);
        assert_eq!(out.0, 0); // exit code
    }
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd /f/Projects/webviewApp/src-tauri && cargo test engine 2>&1 | tail -20`
Expected: 编译错误。

- [ ] **Step 3: 写实现**

在测试之上加:
```rust
use crate::types::FfmpegStatus;
use std::path::Path;
use std::process::{Command, Stdio};
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
}

fn base_command(program: &str) -> Command {
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);
    cmd
}

/// Run a process with a timeout. Returns (exit_code, timed_out, captured_output).
/// exit_code is -1 on spawn failure, -2 on timeout.
fn run_process(program: &str, args: &[&str], timeout_secs: i64) -> (i32, bool, String) {
    let mut cmd = base_command(program);
    cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(_) => return (-1, false, String::new()),
    };
    let start = Instant::now();
    loop {
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
                    return (-2, true, String::new());
                }
                std::thread::sleep(Duration::from_millis(100));
            }
            Err(_) => return (-1, false, String::new()),
        }
    }
}

/// Compress one file. Mirrors CompressionEngine::compress.
pub fn compress(params: &CompressParams) -> CompressResult {
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

    let (code, timed_out, _out) = run_process(&params.ffmpeg_path, &arg_refs, params.timeout_seconds);
    result.duration_ms = start.elapsed().as_millis() as i32;

    if timed_out {
        result.error_message = format!("压缩超时 ({}s)", params.timeout_seconds);
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
    let (code, timed_out, out) = run_process(path, &["-version"], 5);
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
```
确保 `compressor/mod.rs` 含 `pub mod engine;`。

- [ ] **Step 4: 运行测试确认通过**

Run: `cd /f/Projects/webviewApp/src-tauri && cargo test engine 2>&1 | tail -20`
Expected: `test result: ok. 3 passed`.

- [ ] **Step 5: 提交**

```bash
git add -A && git commit -m "feat(rust): ffmpeg engine + probe mirroring CompressionEngine"
```

---

## Task 15: app.rs — 编排层(对齐 app.cpp)

**Files:**
- Create: `src-tauri/src/app.rs`
- Modify: `src-tauri/src/main.rs`

`AppCore` 持有全局配置、scheduler、template_manager、compressor_running(AtomicBool)、ffmpeg 状态,及可选 `AppHandle`(用于 emit event)。用 `Arc<Mutex<...>>` 包装可变部分,便于 command 与后台线程共享。

关键方法(对齐 app.cpp):
- `execute_for_directory(dir, advance_schedule)`:load dir config;template_manager 同步全局 templates;无效 → (advance 则 mark_completed) 返回。重叠检测:若该目录被标记 overlap → (advance 则 mark_completed) 返回。扫描;begin_run;逐文件:resolve params(空则 fallback 默认);ffmpeg path(空则 "ffmpeg");temp 路径;磁盘空间不足 → SkippedOther;compress;失败则更新 ffmpeg 状态(emit);读 tmp size;compare_and_cleanup;写 final_name/final_path/cycle_risk;记日志。汇总 computeTotals;finalize_run;advance 则 mark_completed;更新 last_run。
- `start_for_directory(dir, advance)`:`compressor_running.swap(true)` 已 true 则返回;detach 线程跑 execute,结束 `store(false)`。
- `refresh_schedule_table()`:遍历目录,load config,有 schedule_time 且可解析 → enabled + compute_next_run;否则 enabled=false;`scheduler.set_directories`。
- `check_ffmpeg_async()`:detach 线程跑 probe,结果存入状态 + emit `ffmpeg-status-changed`。
- `build_card_infos() -> Vec<DirCardInfo>`:给 `list_directories` command 用(状态徽章/文件数/大小/参数名/循环风险/上次/下次)。
- `format_next_run(dir)`:今天/明天 HH:MM 或 MM-DD HH:MM 或 "未配置"(对齐 formatNextRun)。

因 emit 依赖 Tauri 运行时,`execute_for_directory` 的**纯逻辑**(单目录一次压缩,写日志)可用 dummy-ffmpeg 集成测试;此处单测覆盖 `format_next_run` 与 `build_card_infos` 的徽章判定纯逻辑。

- [ ] **Step 1: 写失败测试(徽章判定 + format_next_run 纯逻辑)**

`src-tauri/src/app.rs`(仅测试):
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn badge_for_missing_config_is_invalid() {
        let tmp = tempfile::tempdir().unwrap();
        let (badge, _detail) = compute_badge(tmp.path().to_str().unwrap(), false);
        assert_eq!(badge, "invalid");
    }

    #[test]
    fn badge_for_valid_scheduled_and_unscheduled() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().to_str().unwrap();
        crate::config::directory_config::DirectoryConfig::create_default(dir, "");
        // no schedule → unscheduled
        let (b1, _) = compute_badge(dir, false);
        assert_eq!(b1, "unscheduled");
        // overlap flag → overlap wins
        let (b2, _) = compute_badge(dir, true);
        assert_eq!(b2, "overlap");
    }
}
```

- [ ] **Step 2: 运行测试确认失败**

Run: `cd /f/Projects/webviewApp/src-tauri && cargo test app:: 2>&1 | tail -20`
Expected: 编译错误。

- [ ] **Step 3: 写实现**

`src-tauri/src/app.rs`(核心;emit 用 `Option<AppHandle>` 保护,测试时为 None):
```rust
use crate::compressor::{engine, file_compare};
use crate::config::directory_config::DirectoryConfig;
use crate::config::global_config::GlobalConfig;
use crate::config::template_manager::TemplateManager;
use crate::logger::{self, Logger, now_for_filename};
use crate::scanner;
use crate::scheduler::{compute_next_run, DirSchedule, Scheduler};
use crate::types::*;
use crate::util::fs_util::has_enough_space;
use crate::util::string_util::format_file_size;
use chrono::{Datelike, Local, Timelike};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

const FALLBACK_PARAMS: &str = "-c:v libx264 -crf 23 -preset fast -c:a aac -b:a 128k";

pub struct AppCore {
    pub config: Mutex<GlobalConfig>,
    pub scheduler: Scheduler,
    pub template_manager: Mutex<TemplateManager>,
    pub compressor_running: AtomicBool,
    pub ffmpeg_status: Mutex<FfmpegStatus>,
    pub last_runs: Mutex<std::collections::HashMap<String, (String, String)>>, // dir → (time, result)
    pub app_handle: Mutex<Option<AppHandle>>,
}

/// Compute the status badge for a directory. Pure given (dir, overlap flag).
/// Returns (badge, detail). badge ∈ {valid, unscheduled, invalid, config_error, overlap}.
pub fn compute_badge(dir: &str, overlap: bool) -> (String, String) {
    if overlap {
        return ("overlap".into(), String::new());
    }
    let cfg = DirectoryConfig::load(dir);
    if !cfg.valid {
        // distinguish missing vs parse error
        if cfg.error_message.contains("not found") {
            return ("invalid".into(), String::new());
        }
        return ("config_error".into(), cfg.error_message);
    }
    match &cfg.schedule_time {
        Some(t) if !t.is_empty() => ("valid".into(), String::new()),
        _ => ("unscheduled".into(), String::new()),
    }
}

impl AppCore {
    pub fn new() -> Arc<Self> {
        let config = GlobalConfig::load();
        let mut tm = TemplateManager::new();
        tm.set_templates(config.templates.clone());
        Arc::new(AppCore {
            config: Mutex::new(config),
            scheduler: Scheduler::new(),
            template_manager: Mutex::new(tm),
            compressor_running: AtomicBool::new(false),
            ffmpeg_status: Mutex::new(FfmpegStatus::default()),
            last_runs: Mutex::new(std::collections::HashMap::new()),
            app_handle: Mutex::new(None),
        })
    }

    pub fn set_app_handle(&self, handle: AppHandle) {
        *self.app_handle.lock().unwrap() = Some(handle);
    }

    fn emit<S: serde::Serialize + Clone>(&self, event: &str, payload: S) {
        if let Some(h) = self.app_handle.lock().unwrap().as_ref() {
            let _ = h.emit(event, payload);
        }
    }

    /// Rebuild the scheduler timetable from per-directory configs. Mirrors refreshScheduleTable.
    pub fn refresh_schedule_table(&self) {
        let dirs = self.config.lock().unwrap().directories.clone();
        let mut schedules = Vec::new();
        for d in &dirs {
            let cfg = DirectoryConfig::load(&d.path);
            let (enabled, next_run) = match (&cfg.valid, &cfg.schedule_time) {
                (true, Some(t)) if !t.is_empty() => {
                    let parts: Vec<&str> = t.split(':').collect();
                    if parts.len() == 2 {
                        if let (Ok(hh), Ok(mm)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                            (d.enabled, compute_next_run(Local::now(), hh, mm))
                        } else {
                            (false, Local::now())
                        }
                    } else {
                        (false, Local::now())
                    }
                }
                _ => (false, Local::now()),
            };
            schedules.push(DirSchedule { dir_path: d.path.clone(), enabled, next_run });
        }
        self.scheduler.set_directories(schedules);
    }

    /// Human-readable next-run label. Mirrors App::formatNextRun.
    pub fn format_next_run(&self, dir: &str) -> String {
        match self.scheduler.next_run_time(dir) {
            None => "未配置".into(),
            Some(next) => {
                let now = Local::now();
                if next.year() == now.year() && next.ordinal() == now.ordinal() {
                    format!("今天 {:02}:{:02}", next.hour(), next.minute())
                } else if next.year() == now.year() && next.ordinal() == now.ordinal() + 1 {
                    format!("明天 {:02}:{:02}", next.hour(), next.minute())
                } else {
                    format!("{:02}-{:02} {:02}:{:02}",
                        next.month(), next.day(), next.hour(), next.minute())
                }
            }
        }
    }

    /// Aggregate card info for the level-1 list. Used by list_directories command.
    pub fn build_card_infos(&self) -> Vec<DirCardInfo> {
        let (dirs, overlaps) = {
            let c = self.config.lock().unwrap();
            (c.directories.clone(), c.detect_overlaps())
        };
        let mut out = Vec::new();
        for (i, d) in dirs.iter().enumerate() {
            let overlap = overlaps.get(i).copied().unwrap_or(false);
            let (badge, detail) = compute_badge(&d.path, overlap);
            let cfg = DirectoryConfig::load(&d.path);
            let (file_count, total_size, cycle_risk) = if cfg.valid {
                let files = scanner::scan(&cfg);
                let size: u64 = files.iter().map(|f| f.file_size).sum();
                let risk = files.iter().filter(|f| f.cycle_risk).count() as i32;
                (files.len() as i32, size, risk)
            } else {
                (0, 0, 0)
            };
            let (last_time, last_result) = self.last_runs.lock().unwrap()
                .get(&d.path).cloned().unwrap_or_default();
            out.push(DirCardInfo {
                path: d.path.clone(),
                enabled: d.enabled,
                badge,
                badge_detail: detail,
                file_count,
                total_size,
                params_name: cfg.params.clone(),
                cycle_risk_count: cycle_risk,
                last_run_time: last_time,
                last_run_result: last_result,
                next_run_time: self.format_next_run(&d.path),
            });
        }
        out
    }

    /// Launch the per-directory pipeline on a background thread (serial-locked).
    /// Mirrors App::startForDirectory. Returns false if busy.
    pub fn start_for_directory(self: &Arc<Self>, dir: String, advance: bool) -> bool {
        if self.compressor_running.swap(true, Ordering::SeqCst) {
            return false;
        }
        let me = Arc::clone(self);
        std::thread::spawn(move || {
            me.execute_for_directory(&dir, advance);
            me.compressor_running.store(false, Ordering::SeqCst);
        });
        true
    }

    /// Core per-directory pipeline. Mirrors App::executeForDirectory.
    pub fn execute_for_directory(&self, dir: &str, advance: bool) {
        let dir_config = DirectoryConfig::load(dir);
        {
            let templates = self.config.lock().unwrap().templates.clone();
            self.template_manager.lock().unwrap().set_templates(templates);
        }
        if !dir_config.valid {
            if advance { self.scheduler.mark_completed(dir); }
            return;
        }
        // overlap check
        let overlap = {
            let c = self.config.lock().unwrap();
            let flags = c.detect_overlaps();
            c.directories.iter().enumerate()
                .find(|(_, d)| d.path == dir)
                .map(|(i, _)| flags.get(i).copied().unwrap_or(false))
                .unwrap_or(false)
        };
        if overlap {
            if advance { self.scheduler.mark_completed(dir); }
            return;
        }

        let dir_params = dir_config.params.clone();
        let matcher = dir_config.matcher.clone();
        let files = scanner::scan(&dir_config);

        let (ffmpeg_path, timeout) = {
            let c = self.config.lock().unwrap();
            let p = if c.ffmpeg_path.is_empty() { "ffmpeg".to_string() } else { c.ffmpeg_path.clone() };
            (p, c.ffmpeg_timeout_seconds)
        };

        let mut logger = Logger::new(dir);
        let mut summary = RunSummary::default();
        summary.run_id = now_for_filename();
        summary.start_time = crate::util::string_util::now_iso_public();
        summary.directories.push(DirectoryResult {
            path: dir.to_string(), config_valid: true,
            files_total: files.len() as i32, files_processed: 0,
        });
        logger.begin_run(&summary);

        if !files.is_empty() {
            self.emit_dir_state(dir, Stage::Compressing,
                &format!("压缩中: {}", Path::new(dir).file_name()
                    .and_then(|s| s.to_str()).unwrap_or("")));
        }

        let mut all_results = Vec::new();
        for (fi, sf) in files.iter().enumerate() {
            self.emit("compress-progress", serde_json::json!({
                "dirPath": dir, "currentFile": sf.relative_path,
                "completed": fi + 1, "total": files.len()
            }));

            let resolved = self.template_manager.lock().unwrap().resolve(&dir_params);
            let params = if resolved.is_empty() { FALLBACK_PARAMS.to_string() } else { resolved };

            let orig = Path::new(&sf.absolute_path);
            let parent = orig.parent().map(|p| p.to_path_buf()).unwrap_or_default();
            let temp_path = parent.join(&sf.temp_name);

            if !has_enough_space(&parent, sf.file_size / 2) {
                let mut skip = FileResult::default();
                skip.name = sf.relative_path.clone();
                skip.path = sf.absolute_path.clone();
                skip.final_name = sf.relative_path.clone();
                skip.final_path = sf.absolute_path.clone();
                skip.original_size = sf.file_size;
                skip.status = FileStatus::SkippedOther;
                logger.log_file_result(&skip);
                all_results.push(skip);
                continue;
            }

            let cparams = engine::CompressParams {
                ffmpeg_path: ffmpeg_path.clone(),
                arguments: params,
                input_path: sf.absolute_path.clone(),
                output_path: temp_path.to_string_lossy().to_string(),
                timeout_seconds: timeout,
            };
            let cres = engine::compress(&cparams);
            if !cres.success {
                self.update_ffmpeg_status(FfmpegStatus {
                    ready: false, version: String::new(), error: cres.error_message.clone(),
                });
            }
            let compressed_size = if cres.success && temp_path.exists() {
                std::fs::metadata(&temp_path).map(|m| m.len()).unwrap_or(0)
            } else { 0 };

            let mut fr = file_compare::compare_and_cleanup(
                &sf.absolute_path, sf.file_size,
                &temp_path.to_string_lossy(), compressed_size,
                &matcher, cres.exit_code, cres.duration_ms);
            fr.final_name = matcher.apply_rename(&sf.relative_path);
            fr.final_path = parent.join(&fr.final_name).to_string_lossy().to_string();
            fr.cycle_risk = sf.cycle_risk;
            logger.log_file_result(&fr);
            all_results.push(fr);
        }

        if let Some(d0) = summary.directories.get_mut(0) {
            d0.files_processed = files.len() as i32;
        }
        summary.files = all_results;
        summary.end_time = crate::util::string_util::now_iso_public();
        summary.duration_seconds = 0;
        summary.compute_totals();
        logger.finalize_run(&summary);

        if advance { self.scheduler.mark_completed(dir); }

        // record last-run for cards
        let result_str = format!("成功{}·节省{}",
            summary.success_count, format_file_size(summary.total_saved_bytes));
        self.last_runs.lock().unwrap()
            .insert(dir.to_string(), (summary.start_time.clone(), result_str));

        self.emit_dir_state(dir, Stage::Idle, "就绪");
    }

    fn emit_dir_state(&self, dir: &str, stage: Stage, status: &str) {
        let mut st = DirRuntimeState::new(dir.to_string());
        st.stage = stage;
        st.status_text = status.to_string();
        st.next_run_time = self.format_next_run(dir);
        if let Some((t, r)) = self.last_runs.lock().unwrap().get(dir) {
            st.last_run_time = t.clone();
            st.last_run_result = r.clone();
        }
        self.emit("dir-state-changed", st);
    }

    pub fn check_ffmpeg_async(self: &Arc<Self>) {
        let me = Arc::clone(self);
        let path = {
            let c = me.config.lock().unwrap();
            if c.ffmpeg_path.is_empty() { "ffmpeg".to_string() } else { c.ffmpeg_path.clone() }
        };
        std::thread::spawn(move || {
            let status = engine::probe_ffmpeg(&path);
            me.update_ffmpeg_status(status);
        });
    }

    fn update_ffmpeg_status(&self, status: FfmpegStatus) {
        *self.ffmpeg_status.lock().unwrap() = status.clone();
        self.emit("ffmpeg-status-changed", status);
    }
}
```
在 `util/string_util.rs` 补一个可被 app 复用的 ISO 时间戳(logger 内部的 `now_iso` 是私有的,这里公开一个同实现的函数):
```rust
/// ISO8601 local timestamp (for run summaries). Mirrors StringUtils::nowToString.
pub fn now_iso_public() -> String {
    chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%z").to_string()
}
```
app.rs 中已用完整路径 `crate::util::string_util::now_iso_public()` 调用(见上面 start_time/end_time 两处),无需额外 use 别名。
`main.rs` 加 `mod app;`;`tauri::Emitter` trait 由 tauri 提供(app.rs 顶部已 `use tauri::Emitter;`)。

> 注:`chrono` 需要在 `Cargo.toml` 已引入(Task 1)。`tauri::Emitter` trait 在 Tauri 2 中提供 `emit`。

- [ ] **Step 4: 运行测试确认通过**

Run: `cd /f/Projects/webviewApp/src-tauri && cargo test app:: 2>&1 | tail -20`
Expected: `test result: ok. 2 passed`.

- [ ] **Step 5: 提交**

```bash
git add -A && git commit -m "feat(rust): AppCore orchestration mirroring app.cpp"
```

---

## Task 16: commands.rs — Tauri command 层

**Files:**
- Create: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/main.rs`

全部 command 为薄封装,通过 `State<Arc<AppCore>>` 访问核心。返回 `Result<T, AppError>`。命令清单见 spec §2.1。

- [ ] **Step 1: 写实现(command 层无独立单测;由集成/前端手动验证)**

`src-tauri/src/commands.rs`:
```rust
use crate::app::AppCore;
use crate::config::directory_config::DirectoryConfig;
use crate::error::{AppError, AppResult};
use crate::logger;
use crate::scanner;
use crate::types::*;
use std::sync::Arc;
use tauri::State;

type Core<'a> = State<'a, Arc<AppCore>>;

#[tauri::command]
pub fn list_directories(core: Core) -> Vec<DirCardInfo> {
    core.build_card_infos()
}

#[tauri::command]
pub fn get_global_config(core: Core) -> serde_json::Value {
    let c = core.config.lock().unwrap();
    serde_json::json!({
        "ffmpegPath": c.ffmpeg_path,
        "ffmpegTimeoutSeconds": c.ffmpeg_timeout_seconds,
        "minimizeToTray": c.minimize_to_tray,
        "startWithWindows": c.start_with_windows,
        "logRetentionDays": c.log_retention_days,
        "language": c.language,
        "templates": c.templates.iter().map(|(n,p)|
            serde_json::json!({"name": n, "params": p})).collect::<Vec<_>>(),
    })
}

#[tauri::command]
pub fn save_global_config(core: Core, config: serde_json::Value) -> AppResult<()> {
    {
        let mut c = core.config.lock().unwrap();
        if let Some(v) = config.get("ffmpegPath").and_then(|x| x.as_str()) { c.ffmpeg_path = v.into(); }
        if let Some(v) = config.get("ffmpegTimeoutSeconds").and_then(|x| x.as_i64()) { c.ffmpeg_timeout_seconds = v; }
        if let Some(v) = config.get("minimizeToTray").and_then(|x| x.as_bool()) { c.minimize_to_tray = v; }
        if let Some(v) = config.get("startWithWindows").and_then(|x| x.as_bool()) { c.start_with_windows = v; }
        if let Some(v) = config.get("logRetentionDays").and_then(|x| x.as_i64()) { c.log_retention_days = v; }
        if let Some(v) = config.get("language").and_then(|x| x.as_str()) { c.language = v.into(); }
        if let Some(arr) = config.get("templates").and_then(|x| x.as_array()) {
            c.templates = arr.iter().filter_map(|t| {
                let n = t.get("name").and_then(|x| x.as_str())?.to_string();
                let p = t.get("params").and_then(|x| x.as_str()).unwrap_or("").to_string();
                if n.is_empty() { None } else { Some((n, p)) }
            }).collect();
        }
        if !c.save() { return Err(AppError::new("保存全局配置失败")); }
        core.template_manager.lock().unwrap().set_templates(c.templates.clone());
    }
    core.refresh_schedule_table();
    Ok(())
}

#[tauri::command]
pub fn add_directory(core: Core, path: String) -> AppResult<()> {
    {
        let mut c = core.config.lock().unwrap();
        c.add_directory(&path);
        if !c.save() { return Err(AppError::new("保存配置失败")); }
    }
    core.refresh_schedule_table();
    Ok(())
}

#[tauri::command]
pub fn remove_directory(core: Core, path: String, force: bool) -> AppResult<()> {
    if core.compressor_running.load(std::sync::atomic::Ordering::SeqCst) && !force {
        return Err(AppError::new("正在压缩,需确认强制移除"));
    }
    {
        let mut c = core.config.lock().unwrap();
        if let Some(i) = c.directories.iter().position(|d| d.path == path) {
            c.remove_directory(i);
        }
        if !c.save() { return Err(AppError::new("保存配置失败")); }
    }
    core.refresh_schedule_table();
    Ok(())
}

#[tauri::command]
pub fn set_directory_enabled(core: Core, path: String, enabled: bool) -> AppResult<()> {
    {
        let mut c = core.config.lock().unwrap();
        if let Some(i) = c.directories.iter().position(|d| d.path == path) {
            c.set_enabled(i, enabled);
        }
        if !c.save() { return Err(AppError::new("保存配置失败")); }
    }
    core.refresh_schedule_table();
    Ok(())
}

fn to_view(cfg: &DirectoryConfig, exists: bool) -> DirConfigView {
    DirConfigView {
        exists,
        valid: cfg.valid,
        error_message: cfg.error_message.clone(),
        include: cfg.include_patterns.clone(),
        exclude: cfg.exclude_patterns.clone(),
        max_size_mb: cfg.max_size_bytes.map(|b| b as f64 / (1024.0*1024.0)),
        min_size_mb: cfg.min_size_bytes.map(|b| b as f64 / (1024.0*1024.0)),
        mtime_after: cfg.mtime_after.clone(),
        mtime_before: cfg.mtime_before.clone(),
        ctime_after: cfg.ctime_after.clone(),
        ctime_before: cfg.ctime_before.clone(),
        rename_rules: cfg.rename_rules.iter().map(|(p,r)|
            RenameRuleView { pattern: p.clone(), replacement: r.clone() }).collect(),
        params: cfg.params.clone(),
        schedule_time: cfg.schedule_time.clone(),
    }
}

#[tauri::command]
pub fn get_directory_config(path: String) -> DirConfigView {
    let exists = DirectoryConfig::config_path(&path).exists();
    if exists {
        to_view(&DirectoryConfig::load(&path), true)
    } else {
        // default template view, not written to disk yet
        let mut cfg = DirectoryConfig::default();
        cfg.directory_path = path.clone();
        cfg.include_patterns = vec!["*.mp4".into(),"*.mov".into(),"*.avi".into(),"*.mkv".into()];
        cfg.exclude_patterns = vec!["*[compress]*".into()];
        cfg.rename_rules = vec![("^(.+)(\\.[^.]+)$".into(), "$1[compress]$2".into())];
        to_view(&cfg, false)
    }
}

fn apply_view(path: &str, view: &DirConfigView) -> DirectoryConfig {
    let mut cfg = DirectoryConfig::default();
    cfg.directory_path = path.to_string();
    cfg.valid = true;
    cfg.include_patterns = view.include.clone();
    cfg.exclude_patterns = view.exclude.clone();
    cfg.max_size_bytes = view.max_size_mb.map(|m| (m*1024.0*1024.0) as u64);
    cfg.min_size_bytes = view.min_size_mb.map(|m| (m*1024.0*1024.0) as u64);
    cfg.mtime_after = view.mtime_after.clone();
    cfg.mtime_before = view.mtime_before.clone();
    cfg.ctime_after = view.ctime_after.clone();
    cfg.ctime_before = view.ctime_before.clone();
    cfg.rename_rules = view.rename_rules.iter().map(|r| (r.pattern.clone(), r.replacement.clone())).collect();
    cfg.params = view.params.clone();
    cfg.schedule_time = view.schedule_time.clone();
    cfg
}

#[tauri::command]
pub fn save_directory_config(core: Core, path: String, config: DirConfigView) -> AppResult<()> {
    if config.include.iter().all(|s| s.trim().is_empty()) {
        return Err(AppError::new("白名单(include)不能为空"));
    }
    let cfg = apply_view(&path, &config);
    if !cfg.save() { return Err(AppError::new("写入配置文件失败")); }
    core.refresh_schedule_table();
    Ok(())
}

#[tauri::command]
pub fn create_directory_config(core: Core, path: String, config: DirConfigView) -> AppResult<()> {
    save_directory_config(core, path, config)
}

#[tauri::command]
pub fn get_config_mtime(path: String) -> u64 {
    let p = DirectoryConfig::config_path(&path);
    std::fs::metadata(&p)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[tauri::command]
pub fn open_config_in_editor(app: tauri::AppHandle, path: String) -> AppResult<()> {
    use tauri_plugin_opener::OpenerExt;
    let p = DirectoryConfig::config_path(&path);
    app.opener().open_path(p.to_string_lossy().to_string(), None::<&str>)
        .map_err(|e| AppError::new(e.to_string()))
}

#[tauri::command]
pub fn scan_directory(path: String) -> Vec<FilePreview> {
    let cfg = DirectoryConfig::load(&path);
    scanner::scan(&cfg).into_iter().map(|f| FilePreview {
        relative_path: f.relative_path,
        final_name: f.final_name,
        file_size: f.file_size,
        cycle_risk: f.cycle_risk,
    }).collect()
}

#[tauri::command]
pub fn list_run_history(path: String) -> Vec<RunSummary> {
    logger::read_history(&path)
}

#[tauri::command]
pub fn compress_directory_now(core: Core, path: String) -> AppResult<()> {
    let core_arc = core.inner().clone();
    if core_arc.start_for_directory(path, false) {
        Ok(())
    } else {
        Err(AppError::new("已有目录正在压缩,请稍后"))
    }
}

#[tauri::command]
pub fn recheck_ffmpeg(core: Core) -> AppResult<()> {
    let core_arc = core.inner().clone();
    core_arc.check_ffmpeg_async();
    Ok(())
}

#[tauri::command]
pub fn get_ffmpeg_status(core: Core) -> FfmpegStatus {
    core.ffmpeg_status.lock().unwrap().clone()
}
```
`main.rs` 加 `mod commands;`。

- [ ] **Step 2: 加 opener 插件依赖**

`src-tauri/Cargo.toml` `[dependencies]` 加(若脚手架未含):
```toml
tauri-plugin-opener = "2"
```

- [ ] **Step 3: 编译**

Run: `cd /f/Projects/webviewApp/src-tauri && cargo build 2>&1 | tail -15`
Expected: `Finished`(可能有未接线告警,下一任务解决)。

- [ ] **Step 4: 提交**

```bash
git add -A && git commit -m "feat(rust): Tauri command layer"
```

---

## Task 17: main.rs — 应用接线 + 系统集成(tray / autostart / --run-once)

**Files:**
- Modify: `src-tauri/src/main.rs`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`

- [ ] **Step 1: 加插件依赖**

`src-tauri/Cargo.toml` `[dependencies]` 加:
```toml
tauri-plugin-autostart = "2"
```
`tauri.conf.json` 确保 `app.trayIcon` 存在(用默认 icon)与 `withGlobalTauri` 关系不大;主要在 Rust 侧建 tray。

- [ ] **Step 2: 写 main.rs 接线**

`src-tauri/src/main.rs`(替换 `run`/`main`,保留所有 `mod` 声明):
```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod error;
mod types;
mod util;
mod config;
mod scanner;
mod scheduler;
mod logger;
mod compressor;
mod app;
mod commands;

use app::AppCore;
use std::sync::Arc;
use tauri::{Manager, tray::TrayIconBuilder, menu::{Menu, MenuItem}};

fn run_once(core: &Arc<AppCore>) {
    // Synchronously run one compression cycle over all valid, enabled directories.
    let dirs = core.config.lock().unwrap().directories.clone();
    for d in dirs {
        if d.enabled {
            core.execute_for_directory(&d.path, false);
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let core = AppCore::new();

    // --run-once: headless single cycle for Task Scheduler, then exit.
    if args.iter().any(|a| a == "--run-once") {
        run_once(&core);
        return;
    }

    // Startup: per-directory log cleanup using global retention.
    {
        let retention = core.config.lock().unwrap().log_retention_days;
        let dirs = core.config.lock().unwrap().directories.clone();
        for d in dirs {
            logger::Logger::new(&d.path).clean_old_logs(retention);
        }
    }

    let core_for_setup = Arc::clone(&core);
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent, None))
        .manage(Arc::clone(&core))
        .invoke_handler(tauri::generate_handler![
            commands::list_directories,
            commands::get_global_config,
            commands::save_global_config,
            commands::add_directory,
            commands::remove_directory,
            commands::set_directory_enabled,
            commands::get_directory_config,
            commands::save_directory_config,
            commands::create_directory_config,
            commands::get_config_mtime,
            commands::open_config_in_editor,
            commands::scan_directory,
            commands::list_run_history,
            commands::compress_directory_now,
            commands::recheck_ffmpeg,
            commands::get_ffmpeg_status,
        ])
        .setup(move |app| {
            let handle = app.handle().clone();
            core_for_setup.set_app_handle(handle.clone());

            // Scheduler: trigger a single directory (scheduled → advance).
            let core_cb = Arc::clone(&core_for_setup);
            core_for_setup.scheduler.start(move |dir| {
                core_cb.start_for_directory(dir, true);
            });
            core_for_setup.refresh_schedule_table();
            core_for_setup.check_ffmpeg_async();

            // Tray with 显示 / 立即压缩 / 退出.
            let show = MenuItem::with_id(app, "show", "显示主窗口", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(app, &[&show, &quit])?;
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(move |app, event| match event.id().as_ref() {
                    "show" => {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "quit" => { app.exit(0); }
                    _ => {}
                })
                .build(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let core: tauri::State<Arc<AppCore>> = window.state();
                if core.compressor_running.load(std::sync::atomic::Ordering::SeqCst) {
                    // Ask the frontend to confirm; prevent immediate close.
                    api.prevent_close();
                    let _ = window.emit("close-requested-while-compressing", ());
                } else if core.config.lock().unwrap().minimize_to_tray {
                    // Minimize to tray instead of quitting.
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 3: 更新 capabilities 允许插件命令**

编辑 `src-tauri/capabilities/default.json`,`permissions` 数组加:
```json
"opener:default",
"autostart:default",
"core:window:allow-show",
"core:window:allow-hide",
"core:window:allow-set-focus"
```

- [ ] **Step 4: 编译 + 运行冒烟**

Run: `cd /f/Projects/webviewApp/src-tauri && cargo build 2>&1 | tail -15`
Expected: `Finished`.
Run(可选,需 GUI):`cd /f/Projects/webviewApp && npm run tauri dev`(启动后应出现窗口 + 托盘图标;Ctrl+C 结束)。

- [ ] **Step 5: 提交**

```bash
git add -A && git commit -m "feat(rust): wire app, tray, autostart, --run-once, close handling"
```

---

## Task 18: dummy-ffmpeg 集成测试(engine 端到端,不依赖真实 ffmpeg)

**Files:**
- Create: `src-tauri/tests/dummy_ffmpeg_integration.rs`

用一个 `.bat` 脚本充当 ffmpeg:接受 `-version` 输出 `ffmpeg version fake`;接受压缩调用时把输出文件写成固定小尺寸。验证 `probe_ffmpeg` 解析 version,`compress` 产出文件 + 退出码 0。

> 注:集成测试引用库 crate。需在 `Cargo.toml` 增加 `[lib]`(name = "autocompress_lib", path = "src/lib.rs")并建 `src/lib.rs` re-export 模块,或将集成测试改为 `#[path]` 包含。**采用 lib 方案**:建 `src-tauri/src/lib.rs` 声明所有 `pub mod`,`main.rs` 改为 `use autocompress_lib::*;`。

- [ ] **Step 1: 建 lib.rs 暴露模块**

`src-tauri/src/lib.rs`:
```rust
pub mod error;
pub mod types;
pub mod util;
pub mod config;
pub mod scanner;
pub mod scheduler;
pub mod logger;
pub mod compressor;
pub mod app;
pub mod commands;
```
`Cargo.toml` 加:
```toml
[lib]
name = "autocompress_lib"
path = "src/lib.rs"
```
`main.rs` 顶部的 `mod ...;` 全部删除,改为:
```rust
use autocompress_lib::{app, commands, logger};
```
(其余 main.rs 内容用这些路径;`AppCore` 来自 `app::AppCore`。)

- [ ] **Step 2: 写集成测试**

`src-tauri/tests/dummy_ffmpeg_integration.rs`:
```rust
use autocompress_lib::compressor::engine::{compress, probe_ffmpeg, CompressParams};

fn write_dummy_ffmpeg(dir: &std::path::Path) -> String {
    // A .bat that prints a version for `-version`, else copies input→output.
    let bat = dir.join("ffmpeg.bat");
    let script = r#"@echo off
if "%1"=="-version" (
  echo ffmpeg version fake-1.0 built
  exit /b 0
)
rem crude: last arg is output path; write a small file there
set OUT=%~f0
for %%A in (%*) do set LAST=%%A
echo dummy> "%LAST%"
exit /b 0
"#;
    std::fs::write(&bat, script).unwrap();
    bat.to_string_lossy().to_string()
}

#[test]
fn dummy_probe_reports_version() {
    let tmp = tempfile::tempdir().unwrap();
    let ff = write_dummy_ffmpeg(tmp.path());
    let s = probe_ffmpeg(&ff);
    assert!(s.ready, "probe error: {}", s.error);
    assert!(s.version.starts_with("fake"));
}

#[test]
fn dummy_compress_produces_output() {
    let tmp = tempfile::tempdir().unwrap();
    let ff = write_dummy_ffmpeg(tmp.path());
    let input = tmp.path().join("in.mp4");
    std::fs::write(&input, vec![0u8; 100]).unwrap();
    let output = tmp.path().join("out_tmp.mp4");
    let params = CompressParams {
        ffmpeg_path: ff,
        arguments: "-c:v libx264".into(),
        input_path: input.to_string_lossy().to_string(),
        output_path: output.to_string_lossy().to_string(),
        timeout_seconds: 10,
    };
    let r = compress(&params);
    assert!(r.success, "compress failed: {}", r.error_message);
    assert!(output.exists());
}
```

- [ ] **Step 3: 运行集成测试**

Run: `cd /f/Projects/webviewApp/src-tauri && cargo test --test dummy_ffmpeg_integration 2>&1 | tail -20`
Expected: `test result: ok. 2 passed`(`.bat` 由 cmd 执行,Windows 下有效)。

> 若 `probe_ffmpeg` 因 `.bat` 需经 `cmd /C` 才能运行而失败:在 engine 的 `run_process` 中,当 program 以 `.bat`/`.cmd` 结尾时改用 `cmd /C <program> args`。这属于对齐真实场景的加固;仅在测试失败时加此分支。

- [ ] **Step 4: 全量测试回归**

Run: `cd /f/Projects/webviewApp/src-tauri && cargo test 2>&1 | tail -25`
Expected: 所有单元测试 + 集成测试通过。

- [ ] **Step 5: 提交**

```bash
git add -A && git commit -m "test(rust): dummy-ffmpeg integration + lib crate exposure"
```

---

## Task 19: 前端 types.ts + api/tauri.ts — IPC 封装

**Files:**
- Create: `src/types.ts`, `src/api/tauri.ts`

- [ ] **Step 1: 写 types.ts(与 Rust types.rs camelCase 对齐)**

`src/types.ts`:
```ts
export type FileStatus = "success" | "skipped_larger" | "failed" | "skipped_other";
export type Stage = "idle" | "scanning" | "compressing" | "completed";
export type Badge = "valid" | "unscheduled" | "invalid" | "config_error" | "overlap";

export interface FileResult {
  name: string; path: string; finalName: string; finalPath: string;
  status: FileStatus; originalSize: number; compressedSize: number;
  savedBytes: number; ffmpegExitCode: number; ffmpegDurationMs: number; cycleRisk: boolean;
}

export interface DirectoryResult {
  path: string; configValid: boolean; filesTotal: number; filesProcessed: number;
}

export interface RunSummary {
  runId: string; startTime: string; endTime: string; durationSeconds: number;
  directories: DirectoryResult[]; files: FileResult[];
  successCount: number; skippedLargerCount: number; failedCount: number;
  skippedOtherCount: number; totalSavedBytes: number; cycleRiskCount: number;
}

export interface DirRuntimeState {
  dirPath: string; stage: Stage; statusText: string; currentFile: string;
  completedFiles: number; totalFiles: number;
  lastRunTime: string; lastRunResult: string; nextRunTime: string;
}

export interface DirCardInfo {
  path: string; enabled: boolean; badge: Badge; badgeDetail: string;
  fileCount: number; totalSize: number; paramsName: string; cycleRiskCount: number;
  lastRunTime: string; lastRunResult: string; nextRunTime: string;
}

export interface FilePreview {
  relativePath: string; finalName: string; fileSize: number; cycleRisk: boolean;
}

export interface FfmpegStatus { ready: boolean; version: string; error: string; }

export interface RenameRuleView { pattern: string; replacement: string; }

export interface DirConfigView {
  exists: boolean; valid: boolean; errorMessage: string;
  include: string[]; exclude: string[];
  maxSizeMb: number | null; minSizeMb: number | null;
  mtimeAfter: string | null; mtimeBefore: string | null;
  ctimeAfter: string | null; ctimeBefore: string | null;
  renameRules: RenameRuleView[]; params: string; scheduleTime: string | null;
}

export interface Template { name: string; params: string; }

export interface GlobalConfig {
  ffmpegPath: string; ffmpegTimeoutSeconds: number;
  minimizeToTray: boolean; startWithWindows: boolean;
  logRetentionDays: number; language: string; templates: Template[];
}

export interface CompressProgress {
  dirPath: string; currentFile: string; completed: number; total: number;
}
```

- [ ] **Step 2: 写 api/tauri.ts**

`src/api/tauri.ts`:
```ts
import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  DirCardInfo, DirConfigView, FilePreview, RunSummary,
  FfmpegStatus, GlobalConfig, DirRuntimeState, CompressProgress,
} from "../types";

export const api = {
  listDirectories: () => invoke<DirCardInfo[]>("list_directories"),
  getGlobalConfig: () => invoke<GlobalConfig>("get_global_config"),
  saveGlobalConfig: (config: GlobalConfig) => invoke<void>("save_global_config", { config }),
  addDirectory: (path: string) => invoke<void>("add_directory", { path }),
  removeDirectory: (path: string, force: boolean) => invoke<void>("remove_directory", { path, force }),
  setDirectoryEnabled: (path: string, enabled: boolean) =>
    invoke<void>("set_directory_enabled", { path, enabled }),
  getDirectoryConfig: (path: string) => invoke<DirConfigView>("get_directory_config", { path }),
  saveDirectoryConfig: (path: string, config: DirConfigView) =>
    invoke<void>("save_directory_config", { path, config }),
  createDirectoryConfig: (path: string, config: DirConfigView) =>
    invoke<void>("create_directory_config", { path, config }),
  getConfigMtime: (path: string) => invoke<number>("get_config_mtime", { path }),
  openConfigInEditor: (path: string) => invoke<void>("open_config_in_editor", { path }),
  scanDirectory: (path: string) => invoke<FilePreview[]>("scan_directory", { path }),
  listRunHistory: (path: string) => invoke<RunSummary[]>("list_run_history", { path }),
  compressDirectoryNow: (path: string) => invoke<void>("compress_directory_now", { path }),
  recheckFfmpeg: () => invoke<void>("recheck_ffmpeg"),
  getFfmpegStatus: () => invoke<FfmpegStatus>("get_ffmpeg_status"),
};

export const events = {
  onDirState: (cb: (s: DirRuntimeState) => void): Promise<UnlistenFn> =>
    listen<DirRuntimeState>("dir-state-changed", (e) => cb(e.payload)),
  onProgress: (cb: (p: CompressProgress) => void): Promise<UnlistenFn> =>
    listen<CompressProgress>("compress-progress", (e) => cb(e.payload)),
  onFfmpegStatus: (cb: (s: FfmpegStatus) => void): Promise<UnlistenFn> =>
    listen<FfmpegStatus>("ffmpeg-status-changed", (e) => cb(e.payload)),
  onCloseWhileCompressing: (cb: () => void): Promise<UnlistenFn> =>
    listen("close-requested-while-compressing", () => cb()),
};
```

- [ ] **Step 3: 类型检查**

Run: `cd /f/Projects/webviewApp && npx vue-tsc --noEmit 2>&1 | tail -20`
Expected: 无错误(可能提示未使用,忽略)。

- [ ] **Step 4: 提交**

```bash
git add -A && git commit -m "feat(fe): TS types + Tauri IPC wrappers"
```

---

## Task 20: 前端 Pinia store + 文件大小工具

**Files:**
- Create: `src/stores/app.ts`, `src/util/format.ts`
- Modify: `src/main.ts`(挂载 Pinia)

- [ ] **Step 1: 写 format.ts**

`src/util/format.ts`:
```ts
/** Mirror StringUtils::formatFileSize (1 decimal, B..TB). */
export function formatFileSize(bytes: number): string {
  const units = ["B", "KB", "MB", "GB", "TB"];
  let size = bytes;
  let unit = 0;
  while (size >= 1024 && unit < 4) { size /= 1024; unit++; }
  return `${size.toFixed(1)} ${units[unit]}`;
}
```

- [ ] **Step 2: 写 store**

`src/stores/app.ts`:
```ts
import { defineStore } from "pinia";
import { ref } from "vue";
import { api, events } from "../api/tauri";
import type { DirCardInfo, FfmpegStatus, DirRuntimeState, CompressProgress } from "../types";

export const useAppStore = defineStore("app", () => {
  const cards = ref<DirCardInfo[]>([]);
  const ffmpeg = ref<FfmpegStatus>({ ready: false, version: "", error: "" });
  const runtime = ref<Record<string, DirRuntimeState>>({});
  const progress = ref<Record<string, CompressProgress>>({});
  const compressingWhileClose = ref(false);

  async function refreshCards() { cards.value = await api.listDirectories(); }
  async function refreshFfmpeg() { ffmpeg.value = await api.getFfmpegStatus(); }

  async function init() {
    await refreshCards();
    await refreshFfmpeg();
    await events.onFfmpegStatus((s) => { ffmpeg.value = s; });
    await events.onDirState((s) => {
      runtime.value[s.dirPath] = s;
      refreshCards(); // last/next-run and stage changed
    });
    await events.onProgress((p) => { progress.value[p.dirPath] = p; });
    await events.onCloseWhileCompressing(() => { compressingWhileClose.value = true; });
  }

  return { cards, ffmpeg, runtime, progress, compressingWhileClose,
           refreshCards, refreshFfmpeg, init };
});
```

- [ ] **Step 3: 挂载 Pinia**

`src/main.ts` 改为:
```ts
import { createApp } from "vue";
import { createPinia } from "pinia";
import App from "./App.vue";

createApp(App).use(createPinia()).mount("#app");
```

- [ ] **Step 4: 类型检查**

Run: `cd /f/Projects/webviewApp && npx vue-tsc --noEmit 2>&1 | tail -20`
Expected: 无错误。

- [ ] **Step 5: 提交**

```bash
git add -A && git commit -m "feat(fe): Pinia store + format util"
```

---

## Task 21: 前端 App.vue 两级路由 + FfmpegStatusBar + DirCard

**Files:**
- Modify: `src/App.vue`
- Create: `src/components/FfmpegStatusBar.vue`, `src/components/DirCard.vue`, `src/views/DirectoryList.vue`

- [ ] **Step 1: FfmpegStatusBar.vue**

`src/components/FfmpegStatusBar.vue`:
```vue
<script setup lang="ts">
import { useAppStore } from "../stores/app";
import { api } from "../api/tauri";
const store = useAppStore();
async function recheck() { await api.recheckFfmpeg(); }
</script>

<template>
  <div class="ffmpeg-bar" :class="store.ffmpeg.ready ? 'ok' : 'err'">
    <span v-if="store.ffmpeg.ready">✅ ffmpeg {{ store.ffmpeg.version }} 已就位</span>
    <span v-else>❌ ffmpeg 不可用（{{ store.ffmpeg.error }}）</span>
    <button @click="recheck">↻ 重新检测</button>
  </div>
</template>

<style scoped>
.ffmpeg-bar { display:flex; justify-content:space-between; align-items:center;
  padding:6px 12px; border-radius:6px; margin-bottom:10px; }
.ok { background:#e6f7e6; color:#207520; }
.err { background:#fbe6e6; color:#a11; }
</style>
```

- [ ] **Step 2: DirCard.vue**

`src/components/DirCard.vue`:
```vue
<script setup lang="ts">
import type { DirCardInfo } from "../types";
import { formatFileSize } from "../util/format";
import { api } from "../api/tauri";

const props = defineProps<{ card: DirCardInfo }>();
const emit = defineEmits<{ open: [path: string]; changed: [] }>();

const badgeText: Record<string, string> = {
  valid: "● 有效", unscheduled: "● 有效(未排程)",
  invalid: "● 无效(无配置文件)", config_error: "● 配置错误", overlap: "● 重叠",
};

async function toggle(e: Event) {
  e.stopPropagation();
  await api.setDirectoryEnabled(props.card.path, !props.card.enabled);
  emit("changed");
}
async function compressNow(e: Event) {
  e.stopPropagation();
  try { await api.compressDirectoryNow(props.card.path); }
  catch (err) { alert(String(err)); }
}
</script>

<template>
  <div class="card" @click="emit('open', card.path)">
    <div class="row1">
      <span class="path">📁 {{ card.path }}</span>
      <span class="badge">{{ badgeText[card.badge] }}
        <template v-if="card.badgeDetail">({{ card.badgeDetail }})</template>
      </span>
      <label @click.stop><input type="checkbox" :checked="card.enabled" @change="toggle" /> 启用</label>
      <button @click="compressNow">▶</button>
    </div>
    <div class="row2">
      {{ card.fileCount }} 文件 · {{ formatFileSize(card.totalSize) }}
      <template v-if="card.paramsName"> · 参数 {{ card.paramsName }}</template>
    </div>
    <div v-if="card.cycleRiskCount > 0" class="warn">⚠ {{ card.cycleRiskCount }} 个循环风险</div>
    <div class="row3">
      <span>上次: {{ card.lastRunTime || "—" }} {{ card.lastRunResult }}</span>
      <span class="next">下次: {{ card.nextRunTime }}</span>
    </div>
  </div>
</template>

<style scoped>
.card { border:1px solid #ddd; border-radius:8px; padding:10px; margin-bottom:10px; cursor:pointer; }
.card:hover { background:#fafafa; }
.row1 { display:flex; gap:10px; align-items:center; }
.path { font-weight:600; flex:1; }
.row2 { color:#555; font-size:0.9em; margin-top:4px; }
.warn { color:#c60; font-size:0.9em; margin-top:4px; }
.row3 { display:flex; justify-content:space-between; margin-top:6px;
  border-top:1px dashed #ddd; padding-top:6px; font-size:0.9em; }
.next { color:#06c; font-weight:600; }
</style>
```

- [ ] **Step 3: DirectoryList.vue(一级界面)**

`src/views/DirectoryList.vue`:
```vue
<script setup lang="ts">
import { onMounted } from "vue";
import { useAppStore } from "../stores/app";
import { api } from "../api/tauri";
import FfmpegStatusBar from "../components/FfmpegStatusBar.vue";
import DirCard from "../components/DirCard.vue";

const store = useAppStore();
const emit = defineEmits<{ open: [path: string]; settings: [] }>();

onMounted(() => store.refreshCards());

async function addDir() {
  const path = prompt("输入目录路径:");
  if (path) { await api.addDirectory(path); await store.refreshCards(); }
}
</script>

<template>
  <div class="page">
    <div class="menubar">
      <span class="title">AutoCompress</span>
      <button @click="emit('settings')">设置(S)</button>
    </div>
    <FfmpegStatusBar />
    <div class="listhead">
      <span>目录列表 ({{ store.cards.length }})</span>
      <button @click="addDir">＋ 添加目录</button>
    </div>
    <DirCard v-for="c in store.cards" :key="c.path" :card="c"
      @open="emit('open', $event)" @changed="store.refreshCards()" />
  </div>
</template>

<style scoped>
.page { padding:12px; }
.menubar { display:flex; justify-content:space-between; align-items:center; margin-bottom:10px; }
.title { font-size:1.3em; font-weight:700; }
.listhead { display:flex; justify-content:space-between; align-items:center; margin-bottom:8px; }
</style>
```

- [ ] **Step 4: App.vue 路由骨架**

`src/App.vue`:
```vue
<script setup lang="ts">
import { onMounted, ref } from "vue";
import { useAppStore } from "./stores/app";
import DirectoryList from "./views/DirectoryList.vue";
import DirectoryDetail from "./views/DirectoryDetail.vue";
import GlobalSettings from "./components/GlobalSettings.vue";

const store = useAppStore();
const page = ref<"list" | "detail">("list");
const selectedDir = ref("");
const showSettings = ref(false);

onMounted(() => store.init());

function openDir(path: string) { selectedDir.value = path; page.value = "detail"; }
function back() { page.value = "list"; store.refreshCards(); }
</script>

<template>
  <DirectoryList v-if="page === 'list'" @open="openDir" @settings="showSettings = true" />
  <DirectoryDetail v-else :dir-path="selectedDir" @back="back" />
  <GlobalSettings v-if="showSettings" @close="showSettings = false" />
</template>
```

> 注:`DirectoryDetail.vue` 与 `GlobalSettings.vue` 在 Task 22/23 建。为让本任务能类型检查通过,可先建占位:`DirectoryDetail.vue` 内容 `<template><div><button @click="$emit('back')">← 返回</button></div></template><script setup lang="ts">defineProps<{dirPath:string}>();defineEmits<{back:[]}>();</script>`;`GlobalSettings.vue` 占位 `<template><div><button @click="$emit('close')">x</button></div></template><script setup lang="ts">defineEmits<{close:[]}>();</script>`。Task 22/23 再替换为完整实现。

- [ ] **Step 5: 类型检查 + 提交**

Run: `cd /f/Projects/webviewApp && npx vue-tsc --noEmit 2>&1 | tail -20`
Expected: 无错误。
```bash
git add -A && git commit -m "feat(fe): level-1 list, ffmpeg bar, dir card, app routing"
```

---

## Task 22: 前端二级界面 + 三 tab(Preview / Config / History)

**Files:**
- Create: `src/views/DirectoryDetail.vue`, `src/components/tabs/TabPreview.vue`, `src/components/tabs/TabHistory.vue`, `src/components/tabs/TabConfig.vue`, `src/components/ConfigForm.vue`
- (替换 Task 21 的 `DirectoryDetail.vue` 占位)

- [ ] **Step 1: TabPreview.vue**

`src/components/tabs/TabPreview.vue`:
```vue
<script setup lang="ts">
import { ref, watch, onMounted } from "vue";
import { api } from "../../api/tauri";
import { formatFileSize } from "../../util/format";
import type { FilePreview } from "../../types";

const props = defineProps<{ dirPath: string }>();
const files = ref<FilePreview[]>([]);
async function load() { files.value = await api.scanDirectory(props.dirPath); }
onMounted(load);
watch(() => props.dirPath, load);
</script>

<template>
  <table class="tbl">
    <thead><tr><th>原文件</th><th>压缩后</th><th>大小</th><th>风险</th></tr></thead>
    <tbody>
      <tr v-for="f in files" :key="f.relativePath">
        <td>{{ f.relativePath }}</td>
        <td>{{ f.finalName }}</td>
        <td>{{ formatFileSize(f.fileSize) }}</td>
        <td>{{ f.cycleRisk ? "⚠ 循环" : "✅" }}</td>
      </tr>
    </tbody>
  </table>
  <p v-if="files.length === 0" class="empty">无匹配文件</p>
</template>

<style scoped>
.tbl { width:100%; border-collapse:collapse; }
.tbl th, .tbl td { border-bottom:1px solid #eee; padding:6px; text-align:left; font-size:0.9em; }
.empty { color:#888; padding:12px; }
</style>
```

- [ ] **Step 2: TabHistory.vue**

`src/components/tabs/TabHistory.vue`:
```vue
<script setup lang="ts">
import { ref, watch, onMounted } from "vue";
import { api } from "../../api/tauri";
import { formatFileSize } from "../../util/format";
import type { RunSummary } from "../../types";

const props = defineProps<{ dirPath: string }>();
const runs = ref<RunSummary[]>([]);
const expanded = ref<Record<number, boolean>>({});
async function load() { runs.value = await api.listRunHistory(props.dirPath); }
onMounted(load);
watch(() => props.dirPath, load);
</script>

<template>
  <div>
    <p class="count">共 {{ runs.length }} 次执行</p>
    <div v-for="(r, i) in runs" :key="i" class="run">
      <div class="head" @click="expanded[i] = !expanded[i]">
        {{ expanded[i] ? "▼" : "▶" }} {{ r.startTime }}
        — 成功{{ r.successCount }} · 失败{{ r.failedCount }} · 节省{{ formatFileSize(r.totalSavedBytes) }}
      </div>
      <div v-if="expanded[i]" class="detail">
        <div>跳过(更大): {{ r.skippedLargerCount }} · 跳过(其他): {{ r.skippedOtherCount }} · 循环风险: {{ r.cycleRiskCount }}</div>
        <ul>
          <li v-for="(f, j) in r.files" :key="j">
            {{ f.name }} — {{ f.status }} ({{ formatFileSize(f.originalSize) }} → {{ formatFileSize(f.compressedSize) }})
          </li>
        </ul>
      </div>
    </div>
    <p v-if="runs.length === 0" class="empty">暂无执行记录</p>
  </div>
</template>

<style scoped>
.count { color:#555; }
.run { border:1px solid #eee; border-radius:6px; margin-bottom:6px; }
.head { padding:8px; cursor:pointer; background:#fafafa; }
.detail { padding:8px; font-size:0.9em; }
.empty { color:#888; }
</style>
```

- [ ] **Step 3: ConfigForm.vue(表单)**

`src/components/ConfigForm.vue`:
```vue
<script setup lang="ts">
import { reactive, watch } from "vue";
import type { DirConfigView, Template } from "../types";

const props = defineProps<{ modelValue: DirConfigView; templates: Template[] }>();
const emit = defineEmits<{ "update:modelValue": [v: DirConfigView] }>();

const form = reactive<DirConfigView>({ ...props.modelValue });
const custom = reactive({ useCustomParams: !props.templates.some(t => t.name === props.modelValue.params) && props.modelValue.params !== "" });

watch(() => props.modelValue, (v) => { Object.assign(form, v); }, { deep: true });
watch(form, () => emit("update:modelValue", { ...form }), { deep: true });

function addInclude() { form.include.push(""); }
function removeInclude(i: number) { form.include.splice(i, 1); }
function addExclude() { form.exclude.push(""); }
function removeExclude(i: number) { form.exclude.splice(i, 1); }
function addRename() { form.renameRules.push({ pattern: "", replacement: "" }); }
function removeRename(i: number) { form.renameRules.splice(i, 1); }
</script>

<template>
  <div class="form">
    <fieldset><legend>调度</legend>
      每天执行于 <input v-model="form.scheduleTime" placeholder="HH:MM" />
    </fieldset>

    <fieldset><legend>压缩参数</legend>
      <label><input type="checkbox" v-model="custom.useCustomParams" /> 自定义参数</label>
      <select v-if="!custom.useCustomParams" v-model="form.params">
        <option v-for="t in templates" :key="t.name" :value="t.name">{{ t.name }}</option>
      </select>
      <input v-else v-model="form.params" placeholder="裸 ffmpeg 参数" style="width:100%" />
    </fieldset>

    <fieldset><legend>白名单 INCLUDE(必需)</legend>
      <div v-for="(_, i) in form.include" :key="i" class="line">
        <input v-model="form.include[i]" /> <button @click="removeInclude(i)">×</button>
      </div>
      <button @click="addInclude">＋ 添加</button>
    </fieldset>

    <fieldset><legend>黑名单 EXCLUDE</legend>
      <div v-for="(_, i) in form.exclude" :key="i" class="line">
        <input v-model="form.exclude[i]" /> <button @click="removeExclude(i)">×</button>
      </div>
      <button @click="addExclude">＋ 添加</button>
    </fieldset>

    <fieldset><legend>过滤 FILTERS</legend>
      <div class="grid">
        <label>最小 MB <input type="number" v-model.number="form.minSizeMb" /></label>
        <label>最大 MB <input type="number" v-model.number="form.maxSizeMb" /></label>
        <label>修改 ≥ <input v-model="form.mtimeAfter" placeholder="YYYY-MM-DD" /></label>
        <label>修改 ≤ <input v-model="form.mtimeBefore" placeholder="YYYY-MM-DD" /></label>
        <label>创建 ≥ <input v-model="form.ctimeAfter" placeholder="YYYY-MM-DD" /></label>
        <label>创建 ≤ <input v-model="form.ctimeBefore" placeholder="YYYY-MM-DD" /></label>
      </div>
    </fieldset>

    <fieldset><legend>重命名规则</legend>
      <div v-for="(r, i) in form.renameRules" :key="i" class="line">
        <input v-model="r.pattern" placeholder="pattern" />
        →
        <input v-model="r.replacement" placeholder="replacement" />
        <button @click="removeRename(i)">×</button>
      </div>
      <button @click="addRename">＋ 添加</button>
    </fieldset>
  </div>
</template>

<style scoped>
.form fieldset { margin-bottom:10px; }
.line { display:flex; gap:6px; align-items:center; margin-bottom:4px; }
.grid { display:grid; grid-template-columns:1fr 1fr; gap:6px; }
</style>
```

- [ ] **Step 4: TabConfig.vue(含 mtime 轮询)**

`src/components/tabs/TabConfig.vue`:
```vue
<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from "vue";
import { api } from "../../api/tauri";
import ConfigForm from "../ConfigForm.vue";
import type { DirConfigView, Template } from "../../types";

const props = defineProps<{ dirPath: string }>();
const view = ref<DirConfigView | null>(null);
const templates = ref<Template[]>([]);
const error = ref("");
const lastMtime = ref(0);
let timer: number | undefined;

async function load() {
  view.value = await api.getDirectoryConfig(props.dirPath);
  const gc = await api.getGlobalConfig();
  templates.value = gc.templates;
  lastMtime.value = await api.getConfigMtime(props.dirPath);
}
async function poll() {
  const m = await api.getConfigMtime(props.dirPath);
  if (m !== 0 && m !== lastMtime.value) { lastMtime.value = m; await load(); }
}
onMounted(async () => { await load(); timer = window.setInterval(poll, 1500); });
onUnmounted(() => { if (timer) clearInterval(timer); });
watch(() => props.dirPath, load);

async function save() {
  if (!view.value) return;
  error.value = "";
  try {
    if (view.value.exists) await api.saveDirectoryConfig(props.dirPath, view.value);
    else { await api.createDirectoryConfig(props.dirPath, view.value); view.value.exists = true; }
    lastMtime.value = await api.getConfigMtime(props.dirPath);
  } catch (e) { error.value = String(e); }
}
async function reset() { await load(); }
async function openExternal() { await api.openConfigInEditor(props.dirPath); }
</script>

<template>
  <div v-if="view">
    <ConfigForm v-model="view" :templates="templates" />
    <div class="bar">
      <button @click="save">{{ view.exists ? "💾 保存" : "创建配置文件" }}</button>
      <button @click="reset">↻ 重置</button>
      <button @click="openExternal">📂 打开外部编辑器</button>
    </div>
    <p v-if="error" class="err">{{ error }}</p>
  </div>
</template>

<style scoped>
.bar { display:flex; gap:8px; margin-top:10px; }
.err { color:#a11; margin-top:8px; }
</style>
```

- [ ] **Step 5: DirectoryDetail.vue(替换占位)**

`src/views/DirectoryDetail.vue`:
```vue
<script setup lang="ts">
import { ref, computed } from "vue";
import { useAppStore } from "../stores/app";
import { api } from "../api/tauri";
import TabPreview from "../components/tabs/TabPreview.vue";
import TabConfig from "../components/tabs/TabConfig.vue";
import TabHistory from "../components/tabs/TabHistory.vue";

const props = defineProps<{ dirPath: string }>();
const emit = defineEmits<{ back: [] }>();
const store = useAppStore();
const tab = ref<"preview" | "config" | "history">("preview");

const card = computed(() => store.cards.find(c => c.path === props.dirPath));

async function compressNow() {
  try { await api.compressDirectoryNow(props.dirPath); }
  catch (e) { alert(String(e)); }
}
</script>

<template>
  <div class="page">
    <div class="backbar">
      <button @click="emit('back')">← 返回</button>
      <span class="path">📁 {{ dirPath }}</span>
      <button @click="compressNow">立即压缩此目录</button>
    </div>
    <div v-if="card" class="summary">
      匹配 {{ card.fileCount }} · 参数 {{ card.paramsName || "默认" }} · 下次 {{ card.nextRunTime }}
      <span v-if="card.cycleRiskCount">· ⚠ {{ card.cycleRiskCount }} 循环风险</span>
      <div>上次: {{ card.lastRunTime || "—" }} {{ card.lastRunResult }}</div>
    </div>
    <div class="tabs">
      <button :class="{ active: tab==='preview' }" @click="tab='preview'">压缩文件预览</button>
      <button :class="{ active: tab==='config' }" @click="tab='config'">配置</button>
      <button :class="{ active: tab==='history' }" @click="tab='history'">压缩执行历史</button>
    </div>
    <TabPreview v-if="tab==='preview'" :dir-path="dirPath" />
    <TabConfig v-else-if="tab==='config'" :dir-path="dirPath" />
    <TabHistory v-else :dir-path="dirPath" />
  </div>
</template>

<style scoped>
.page { padding:12px; }
.backbar { display:flex; gap:10px; align-items:center; margin-bottom:10px; }
.path { flex:1; font-weight:600; }
.summary { background:#f6f6f6; border-radius:6px; padding:8px; margin-bottom:10px; font-size:0.9em; }
.tabs { display:flex; gap:4px; margin-bottom:10px; }
.tabs button.active { background:#06c; color:#fff; }
</style>
```

- [ ] **Step 6: 类型检查 + 提交**

Run: `cd /f/Projects/webviewApp && npx vue-tsc --noEmit 2>&1 | tail -20`
Expected: 无错误。
```bash
git add -A && git commit -m "feat(fe): level-2 detail + preview/config/history tabs + config form"
```

---

## Task 23: 前端全局设置窗口 + 退出确认

**Files:**
- Create: `src/components/GlobalSettings.vue`(替换 Task 21 占位)
- Modify: `src/App.vue`(退出确认弹窗)

- [ ] **Step 1: GlobalSettings.vue**

`src/components/GlobalSettings.vue`:
```vue
<script setup lang="ts">
import { ref, onMounted } from "vue";
import { api } from "../api/tauri";
import { useAppStore } from "../stores/app";
import type { GlobalConfig } from "../types";

const emit = defineEmits<{ close: [] }>();
const store = useAppStore();
const cfg = ref<GlobalConfig | null>(null);

onMounted(async () => { cfg.value = await api.getGlobalConfig(); });

function addTemplate() { cfg.value!.templates.push({ name: "", params: "" }); }
function removeTemplate(i: number) { cfg.value!.templates.splice(i, 1); }

async function save() {
  if (!cfg.value) return;
  cfg.value.templates = cfg.value.templates.filter(t => t.name.trim() !== "");
  await api.saveGlobalConfig(cfg.value);
  await store.refreshCards();
  await store.refreshFfmpeg();
  emit("close");
}
</script>

<template>
  <div class="overlay" @click.self="emit('close')">
    <div class="dialog" v-if="cfg">
      <h3>全局设置</h3>
      <label>ffmpeg 路径 <input v-model="cfg.ffmpegPath" style="width:100%" /></label>
      <label>ffmpeg 超时(秒) <input type="number" v-model.number="cfg.ffmpegTimeoutSeconds" /></label>
      <label>日志保留天数 <input type="number" v-model.number="cfg.logRetentionDays" /></label>
      <label><input type="checkbox" v-model="cfg.startWithWindows" /> 开机自启动</label>
      <label><input type="checkbox" v-model="cfg.minimizeToTray" /> 最小化到托盘</label>

      <h4>模板</h4>
      <div v-for="(t, i) in cfg.templates" :key="i" class="tmpl">
        <input v-model="t.name" placeholder="名称" />
        <input v-model="t.params" placeholder="参数" style="flex:1" />
        <button @click="removeTemplate(i)">×</button>
      </div>
      <button @click="addTemplate">＋ 添加模板</button>

      <div class="bar">
        <button @click="save">保存</button>
        <button @click="emit('close')">取消</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.overlay { position:fixed; inset:0; background:rgba(0,0,0,.4); display:flex;
  justify-content:center; align-items:center; }
.dialog { background:#fff; border-radius:8px; padding:16px; width:520px; max-height:80vh; overflow:auto; }
.dialog label { display:block; margin:6px 0; }
.tmpl { display:flex; gap:6px; margin-bottom:4px; }
.bar { display:flex; gap:8px; margin-top:12px; }
</style>
```

- [ ] **Step 2: App.vue 加退出确认弹窗**

在 `src/App.vue` `<template>` 末尾(GlobalSettings 之后)加:
```vue
  <div v-if="store.compressingWhileClose" class="overlay">
    <div class="confirm">
      <p>正在压缩,确定退出吗?(当前压缩任务将被中断)</p>
      <button @click="forceQuit">强制退出</button>
      <button @click="store.compressingWhileClose = false">继续等待</button>
    </div>
  </div>
```
并在 `<script setup>` 加:
```ts
import { getCurrentWindow } from "@tauri-apps/api/window";
async function forceQuit() { await getCurrentWindow().destroy(); }
```
样式 `<style>`(非 scoped 或加到全局):
```css
.overlay { position:fixed; inset:0; background:rgba(0,0,0,.4); display:flex; justify-content:center; align-items:center; }
.confirm { background:#fff; padding:20px; border-radius:8px; }
```

- [ ] **Step 3: capability 允许 window destroy**

`src-tauri/capabilities/default.json` `permissions` 加 `"core:window:allow-destroy"`。

- [ ] **Step 4: 类型检查 + 提交**

Run: `cd /f/Projects/webviewApp && npx vue-tsc --noEmit 2>&1 | tail -20`
Expected: 无错误。
```bash
git add -A && git commit -m "feat(fe): global settings dialog + close-while-compressing confirm"
```

---

## Task 24: 前端组件单测(Vitest)+ 端到端验收

**Files:**
- Create: `src/util/format.test.ts`, `src/components/DirCard.test.ts`, `vitest.config.ts`
- Create: `docs/manual-verification-checklist.md`(移植原项目清单)
- Modify: `package.json`(test 脚本)

- [ ] **Step 1: 装 Vitest**

Run: `cd /f/Projects/webviewApp && npm install -D vitest @vue/test-utils jsdom`
`vitest.config.ts`:
```ts
import { defineConfig } from "vitest/config";
import vue from "@vitejs/plugin-vue";
export default defineConfig({
  plugins: [vue()],
  test: { environment: "jsdom" },
});
```
`package.json` `scripts` 加:`"test": "vitest run"`。

- [ ] **Step 2: 写 format.test.ts**

`src/util/format.test.ts`:
```ts
import { describe, it, expect } from "vitest";
import { formatFileSize } from "./format";

describe("formatFileSize", () => {
  it("formats units", () => {
    expect(formatFileSize(0)).toBe("0.0 B");
    expect(formatFileSize(1536)).toBe("1.5 KB");
    expect(formatFileSize(1024 * 1024)).toBe("1.0 MB");
  });
});
```

- [ ] **Step 3: 写 DirCard.test.ts(徽章文案渲染)**

`src/components/DirCard.test.ts`:
```ts
import { describe, it, expect, vi } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import DirCard from "./DirCard.vue";
import type { DirCardInfo } from "../types";

vi.mock("../api/tauri", () => ({ api: {} }));

function card(overrides: Partial<DirCardInfo> = {}): DirCardInfo {
  return {
    path: "D:/x", enabled: true, badge: "valid", badgeDetail: "",
    fileCount: 3, totalSize: 1024, paramsName: "H.265", cycleRiskCount: 0,
    lastRunTime: "", lastRunResult: "", nextRunTime: "今天 03:00", ...overrides,
  };
}

describe("DirCard", () => {
  beforeEach(() => setActivePinia(createPinia()));
  it("shows valid badge", () => {
    const w = mount(DirCard, { props: { card: card() } });
    expect(w.text()).toContain("● 有效");
  });
  it("shows overlap badge", () => {
    const w = mount(DirCard, { props: { card: card({ badge: "overlap", badgeDetail: "D:/parent" }) } });
    expect(w.text()).toContain("● 重叠");
    expect(w.text()).toContain("D:/parent");
  });
  it("shows cycle risk warning", () => {
    const w = mount(DirCard, { props: { card: card({ cycleRiskCount: 2 }) } });
    expect(w.text()).toContain("2 个循环风险");
  });
});
```
(顶部补 `import { beforeEach } from "vitest";`。)

- [ ] **Step 4: 运行前端测试**

Run: `cd /f/Projects/webviewApp && npm test 2>&1 | tail -25`
Expected: 全部通过。

- [ ] **Step 5: 移植手动验收清单**

`docs/manual-verification-checklist.md`:参照原项目 `F:\Projects\AutoCompress\docs\manual-verification-checklist.md`,把构建/运行命令改为:
```
构建:cd src-tauri && cargo build
Rust 测试:cd src-tauri && cargo test
前端测试:npm test
开发运行:npm run tauri dev
打包:npm run tauri build
```
保留其"启动与一级界面 / 二级界面三 tab / 全局设置 / 调度与压缩(schedule.time = now+1min 触发,严格不补跑,立即压缩不重置 nextRun,串行队列)/ 弹窗确认(移除压缩中目录、退出压缩中)"全部勾选项,逐条与本重构对齐。

- [ ] **Step 6: 全量回归 + 提交**

Run: `cd /f/Projects/webviewApp/src-tauri && cargo test 2>&1 | tail -5 && cd /f/Projects/webviewApp && npm test 2>&1 | tail -5`
Expected: Rust + 前端测试全绿。
```bash
git add -A && git commit -m "test(fe): Vitest component tests + manual verification checklist"
```

- [ ] **Step 7: 手动端到端(GUI,人工执行)**

Run: `cd /f/Projects/webviewApp && npm run tauri dev`
按 `docs/manual-verification-checklist.md` 逐条走查:添加目录 → 卡片四态 → 进二级 → 三 tab → 改配置保存 → 外部编辑器 + mtime 重载 → 设 schedule.time 触发 → 历史记录 → 立即压缩 → 串行 → tray → 退出确认 → `--run-once`(`build/AutoCompress.exe --run-once`)。

---

## 完成标准

- `cargo test` 全绿(util / pattern_matcher / template_manager / directory_config / global_config / scanner / scheduler / logger / file_compare / engine / app + dummy-ffmpeg 集成)。
- `npm test` 全绿(format + DirCard)。
- `npm run tauri dev` 手动清单全部通过,功能与原 AutoCompress 完全一致。
- `npm run tauri build` 产出单 exe。
