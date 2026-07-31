# AutoVideoCompressor 项目技术文档

> 基于 Tauri 2 + Rust + Vue 3 的 Windows 视频压缩桌面工具。
> 自动扫描指定目录下的视频文件，调用 ffmpeg 压缩，并保留体积更小的版本。

本文档面向要理解、维护或二次开发本项目的工程师，覆盖技术栈、代码结构与关键实现细节。

应用、包、可执行文件和全局数据目录统一使用 **AutoVideoCompressor**；每个监控目录内的配置子目录为兼容已有配置，仍保留名称 `.autocompress`。

---

## 1. 技术栈总览

| 层级 | 技术 | 说明 |
|------|------|------|
| 应用框架 | **Tauri 2.x** | 前端用系统 WebView2 渲染，业务逻辑跑在 Rust 主进程 |
| 后端语言 | **Rust**（2021 Edition） | 全部业务逻辑（扫描/压缩/调度/配置/日志）都在 Rust 层，前端只做展示 |
| 前端框架 | **Vue 3**（`<script setup>` + Composition API） | SFC 单文件组件 |
| 前端语言 | **TypeScript** | 严格模式（`strict: true`） |
| 状态管理 | **Pinia** | 单一 store：`useAppStore` |
| UI 组件库 | **Naive UI** | 按需自动引入（unplugin-vue-components） |
| 构建工具 | **Vite 5** | 前端构建；`vue-tsc` 做类型检查 |
| 测试框架 | Vitest（前端）+ Cargo test（Rust） | 前端 4+ 用例，Rust 36+ 用例 |
| 外部依赖 | **ffmpeg**（不随包分发，需用户自行安装） | 通过子进程调用，路径可在设置中指定或走 PATH |
| 目标平台 | Windows 10/11 x64 | 仅支持 Windows；打包为 NSIS / MSI / 便携版 exe |

### 1.1 前端依赖（package.json）

- 运行依赖：`vue ^3.4`、`pinia ^2.1`、`naive-ui ^2.44`、`@tauri-apps/plugin-dialog ^2.7`
- 开发依赖：`@tauri-apps/api ^2`、`@tauri-apps/cli ^2`、`vite ^5`、`vue-tsc ^2`、`typescript ^5.3`、`vitest ^1`、`@vue/test-utils ^2`、`unplugin-auto-import`、`unplugin-vue-components`

### 1.2 Rust 依赖（src-tauri/Cargo.toml）

- `tauri 2`（`tray-icon` feature）— 应用框架 + 系统托盘
- `tauri-plugin-opener` / `tauri-plugin-dialog` / `tauri-plugin-autostart` — 打开文件、文件选择对话框、开机自启动
- `serde` / `serde_json` — 序列化
- `regex` — include/exclude/rename 规则匹配引擎
- `chrono` — 时间处理（调度计算、日志时间戳）
- `walkdir` — 递归遍历目录
- `fs4` — 查询磁盘剩余空间
- 开发依赖 `tempfile` — 单元测试用临时目录

---

## 2. 目录结构

```
webviewApp/
├── src/                       # 前端源码 (Vue 3 + TS)
│   ├── main.ts                 # 前端入口：createApp + Pinia
│   ├── App.vue                 # Naive UI Provider 外壳
│   ├── AppShell.vue             # 应用主容器：标题栏 + 一二级页面路由
│   ├── types.ts                 # 前端类型定义（camelCase，对应 Rust types）
│   ├── theme.ts                  # Naive UI 主题覆盖
│   ├── api/
│   │   └── tauri.ts              # invoke()/listen() 的统一封装（16 命令 + 4 事件）
│   ├── stores/
│   │   └── app.ts                 # Pinia store：卡片列表/ffmpeg状态/运行时状态/进度
│   ├── util/
│   │   └── format.ts               # 文件大小格式化等工具函数
│   ├── views/                       # 页面级组件
│   │   ├── DirectoryList.vue          # 一级页面：目录卡片列表
│   │   └── DirectoryDetail.vue        # 二级页面：单目录详情（Tab 容器）
│   └── components/
│       ├── TitleBar.vue                # 自定义无边框标题栏（拖拽/最小化/最大化/关闭）
│       ├── FfmpegStatusBar.vue          # ffmpeg 可用性状态条
│       ├── DirCard.vue                  # 目录卡片（一级页面每行）
│       ├── ConfigForm.vue                # 目录级配置表单（核心表单组件）
│       ├── GlobalSettings.vue            # 全局设置弹窗
│       └── tabs/
│           ├── TabPreview.vue              # 压缩文件预览表格
│           ├── TabConfig.vue                # 配置 Tab（承载 ConfigForm）
│           └── TabHistory.vue               # 执行历史 Tab
│
├── src-tauri/                  # 后端源码 (Rust)
│   ├── Cargo.toml
│   ├── build.rs                 # 调用 tauri_build::build()
│   ├── tauri.conf.json            # 窗口/打包配置
│   ├── icons/                      # 应用图标
│   └── src/
│       ├── main.rs                  # 进程入口：CLI 参数、Tauri Builder、托盘、窗口事件
│       ├── lib.rs                    # 模块声明（供 main.rs 以 lib 形式引用）
│       ├── app.rs                     # AppCore：全局状态与核心业务流程
│       ├── commands.rs                 # 16 个 #[tauri::command] handler
│       ├── types.rs                     # 前后端共享的数据结构（serde 序列化）
│       ├── error.rs                      # AppError / AppResult
│       ├── scanner.rs                     # 文件扫描 + 批次计算(compute_next_run_set)
│       ├── scheduler.rs                    # 定时调度器（严格不补跑）
│       ├── windows_task_scheduler.rs        # 通过 Task Scheduler COM API 管理 Windows 计划任务
│       ├── logger.rs                        # 运行日志读写（人类可读 + JSON 块）
│       ├── config/
│       │   ├── global_config.rs               # 全局配置（%APPDATA%/AutoVideoCompressor/config.json）
│       │   ├── directory_config.rs             # 目录级配置（<dir>/.autocompress/config.json）
│       │   ├── pattern_matcher.rs               # include/exclude/rename 正则引擎
│       │   └── template_manager.rs               # ffmpeg 参数模板解析
│       ├── compressor/
│       │   ├── engine.rs                          # ffmpeg 子进程调用、超时/取消控制
│       │   └── file_compare.rs                     # 压缩结果比较与文件落地
│       └── util/
│           ├── fs_util.rs                           # 文件系统工具（递归列举/安全删除改名/磁盘空间）
│           └── string_util.rs                        # glob→regex、临时文件名、大小格式化
│
├── scripts/
│   └── rename-exe.cjs           # 构建后处理：给便携版 exe 加版本号后缀
├── docs/                        # 项目文档（本文件所在目录）
├── dist/                        # `vite build` 输出（前端静态资源）
├── index.html                   # 前端 HTML 入口（挂载点 #app）
├── vite.config.ts                 # Vite 配置（自动导入、组件解析、端口 1420）
├── vitest.config.ts                # 前端测试配置
├── tsconfig.json / tsconfig.node.json
└── package.json
```

---

## 3. 整体架构

### 3.1 进程模型

Tauri 2 采用系统自带 **WebView2** 渲染前端，没有类似 Electron 那样打包完整 Chromium：

- **主进程（Rust）**：承载全部业务逻辑——文件扫描、ffmpeg 调用、定时调度、配置持久化、日志读写。是唯一能访问文件系统/子进程的一方。
- **渲染进程（WebView2 中的 Vue 应用）**：纯 UI 层，只负责展示状态和收集用户输入，通过 IPC 与主进程通信，不直接操作文件系统。

### 3.2 前后端通信机制

**前端 → 后端（命令调用）**：`@tauri-apps/api/core` 的 `invoke()`，对应 Rust 侧的 `#[tauri::command]` 函数。前端在 `src/api/tauri.ts` 里统一封装成 `api.xxx()`，失败时统一打日志再向上抛。

```ts
// src/api/tauri.ts
async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try { return await invoke<T>(cmd, args); }
  catch (e) { console.error(`[tauri] invoke '${cmd}' 失败:`, e); throw e; }
}
```

**后端 → 前端（事件推送）**：Rust 侧通过 `tauri::Emitter::emit()` 主动推送，前端用 `@tauri-apps/api/event` 的 `listen()` 订阅，封装在 `events.xxx()`。用于压缩进度、状态变化等异步通知（命令调用是一次性请求-响应，无法承载持续更新）。

```rust
// src-tauri/src/app.rs — AppCore::emit
fn emit<S: serde::Serialize + Clone>(&self, event: &str, payload: S) {
    if let Some(h) = self.app_handle.lock().unwrap().as_ref() {
        let _ = h.emit(event, payload);
    }
}
```

### 3.3 16 个 Tauri Command（`commands.rs`）

| 分类 | 命令 | 作用 |
|------|------|------|
| 目录管理 | `list_directories` | 返回一级页面所有目录卡片信息（聚合扫描结果+状态+调度） |
| | `add_directory` / `remove_directory` / `set_directory_enabled` | 增删改监控目录列表 |
| 全局配置 | `get_global_config` / `save_global_config` | 读写 `%APPDATA%/AutoVideoCompressor/config.json` |
| 目录配置 | `get_directory_config` / `save_directory_config` / `create_directory_config` | 读写 `<dir>/.autocompress/config.json`；不存在时返回默认模板视图（未落盘） |
| | `get_config_mtime` | 返回配置文件 mtime，供前端轮询检测外部编辑器修改 |
| | `open_config_in_editor` | 用系统默认程序打开配置文件 |
| 扫描/压缩 | `scan_directory` | 预览匹配到的文件（含是否在下次压缩批次内、循环风险标记） |
| | `compress_directory_now` / `stop_compression` | 立即压缩 / 取消当前压缩 |
| | `list_run_history` | 读取历史压缩记录（来自日志文件） |
| ffmpeg | `get_ffmpeg_status` / `recheck_ffmpeg` | 查询/重新探测 ffmpeg 可用性 |

### 3.4 4 个事件（前端订阅，见 `src/api/tauri.ts` 的 `events` 对象）

| 事件名 | 载荷 | 触发时机 |
|--------|------|----------|
| `dir-state-changed` | `DirRuntimeState` | 目录压缩状态变化（开始/结束） |
| `compress-progress` | `CompressProgress`（dirPath/currentFile/completed/total） | 压缩过程中逐文件进度 |
| `ffmpeg-status-changed` | `FfmpegStatus` | ffmpeg 探测结果变化 |
| `close-requested-while-compressing` | 无 | 用户在压缩进行中尝试关闭窗口，前端弹窗确认 |

---

## 4. 后端（Rust）核心实现

### 4.1 AppCore：全局状态容器（`app.rs`）

```rust
pub struct AppCore {
    pub config: Mutex<GlobalConfig>,                 // 全局配置（线程安全）
    pub scheduler: Scheduler,                         // 定时调度器
    pub template_manager: Mutex<TemplateManager>,      // ffmpeg 参数模板解析器
    pub compressor_running: AtomicBool,                 // 是否有压缩任务在跑（串行锁）
    pub cancel_flag: AtomicBool,                         // 取消信号
    pub ffmpeg_status: Mutex<FfmpegStatus>,               // 最近一次 ffmpeg 探测结果
    pub last_runs: Mutex<HashMap<String, (String, String)>>, // 每目录最近一次运行结果（内存缓存）
    pub app_handle: Mutex<Option<AppHandle>>,              // 用于向前端 emit 事件
}
```

通过 `Arc<AppCore>` 注入 Tauri 的 `.manage()`，所有 command handler 以 `State<Arc<AppCore>>` 取用。

**串行执行保证**：`compressor_running` 是一个 `AtomicBool`，`start_for_directory` 用 `swap(true, ...)` 原子地检测并占用——同一时刻只允许一个目录在压缩，其余请求直接返回失败（前端提示"已有目录正在压缩"）。

**取消机制**：`cancel_flag` 由 `stop_compression` 命令置位，压缩循环在每个文件开始前检查该标志；ffmpeg 子进程运行期间也会周期性检查并 `kill()`。取消后清理当前正在写的临时文件（用重试删除应对 Windows 文件锁延迟释放）。

### 4.2 执行主流程 `execute_for_directory`

这是整个压缩流水线的核心方法，按顺序做：

1. 加载目录配置（`DirectoryConfig::load`），无效则直接返回（若是调度触发，仍需推进调度表避免死循环重试）。
2. 检查该目录是否与其他已配置目录存在路径重叠（`detect_overlaps`），重叠则跳过，避免同一文件被两个目录规则重复处理。
3. `scanner::scan` 扫描全部匹配文件 → `compute_next_run_set` 按 `max_compress_size_bytes` 累计裁剪出本次实际要处理的子集（超出部分留给下次）。
4. 初始化 `Logger`，写运行日志头部。
5. 逐文件循环：
   - 检查取消标志；
   - `emit("compress-progress", ...)` 推送进度；
   - 用 `TemplateManager::resolve` 把配置里的模板名或裸参数解析成实际 ffmpeg 参数；
   - 检查磁盘剩余空间（至少要有原文件一半大小的空闲）；
   - 调用 `compressor::engine::compress` 实际跑 ffmpeg；
   - `file_compare::compare_and_cleanup` 比较原始/压缩后大小，决定保留哪个版本、执行改名/删除；
   - 记录 `FileResult` 到日志。
6. 汇总 `RunSummary`（成功/跳过/失败计数、节省字节数），写入日志尾部的 JSON 块；若是调度触发，调用 `scheduler.mark_completed` 把下次运行时间推到明天同一时刻。
7. 更新 `last_runs` 内存缓存（供一级页面卡片展示"上次运行结果"），并 `emit("dir-state-changed", ...)` 通知前端。

### 4.3 定时调度后端

全局配置 `use_windows_task_scheduler` 控制后端选择，默认为 `false`：

- **应用内调度**（`scheduler.rs`）：`DirSchedule { dir_path, enabled, next_run }` 为每个目录保存一条排程记录，`Scheduler::start` 每秒轮询一次；`compute_next_run` 计算当天或次日的目标时间，`mark_completed` 在触发后把下次运行推进 24 小时，因此严格不补跑。
- **Windows 计划任务**（`windows_task_scheduler.rs`）：通过 Windows Task Scheduler COM API 创建、更新、运行和删除任务，不依赖第三方 CLI。每个目录对应 `\AutoVideoCompressor\<安全目录名>` 任务，动作是当前 exe 加 `--scheduled --directory <path>`；旧版本遗留在根目录的 `AutoVideoCompressor-<安全目录名>` 任务会在新任务成功注册后删除。任务以交互式登录令牌运行，并显式关闭 `StartWhenAvailable`，错过计划时间后不会补跑；在用户已登录时，任务会启动或唤醒 GUI，只把指定目录加入串行队列。前端完成事件监听后才开始压缩，并自动进入对应目录显示进度和停止按钮。目录增删、启停、调度时间或全局唤醒选项变化都会同步对应任务。

全局配置 `wake_computer_for_scheduled_tasks` 控制计划任务的 `WakeToRun`，默认为 `false`。前端仅在启用 Windows 计划任务后显示“唤醒计算机执行任务”；启用后，应用创建的所有目录任务均可在计划时间唤醒处于睡眠状态的计算机。

`refresh_schedule_table`（在 `app.rs`）始终根据各目录的 `schedule.time` 重建排程表，供界面展示下次运行时间；选择 Windows 计划任务时，该表不启动轮询。

### 4.4 配置系统（`config/`）

**全局配置**（`global_config.rs`）— 扁平 JSON，路径 `%APPDATA%/AutoVideoCompressor/config.json`：

```json
{
  "directories": [{ "path": "D:/Videos", "enabled": true }],
  "ffmpeg_path": "",
  "ffmpeg_timeout_seconds": 3600,
  "minimize_to_tray": true,
  "start_with_windows": false,
  "use_windows_task_scheduler": false,
  "wake_computer_for_scheduled_tasks": false,
  "log_retention_days": 90,
  "templates": [
    { "name": "H.265 高质量", "params": "-c:v libx265 -crf 18 -preset slow -c:a aac -b:a 192k" },
    { "name": "H.264 平衡",   "params": "-c:v libx264 -crf 23 -preset medium -c:a aac -b:a 192k" },
    { "name": "H.264 快速",   "params": "-c:v libx264 -crf 28 -preset fast -c:a aac -b:a 128k" }
  ]
}
```

- `add_directory` 按规范化路径（转小写、统一斜杠、去尾部斜杠）去重。
- `detect_overlaps`：检测后添加的目录路径是否是先添加目录的子路径，标记为 overlap（仅标记较晚添加的一方，避免互相冲突处理同一批文件）。

**目录级配置**（`directory_config.rs`）— 路径 `<监控目录>/.autocompress/config.json`：

```json
{
  "include": ["*.mp4", "*.mov", "*.avi", "*.mkv"],
  "exclude": ["*[compress]*"],
  "filters": { "max_size_mb": 2048, "min_size_mb": 10 },
  "max_compress_size_mb": 4096,
  "rename_rules": [{ "pattern": "^(.+)(\\.[^.]+)$", "replacement": "$1[compress]$2" }],
  "params": "H.265 高质量",
  "use_custom_params": false,
  "schedule": { "time": "03:00" }
}
```

- `include` 必须是非空数组，否则整个配置判定为无效（`valid=false`），此时该目录不会被扫描/压缩。
- 加载时会立即编译成 `PatternMatcher`（正则引擎），后续 `passes_filters` 复用编译好的正则，避免重复编译。
- `filters` 支持大小区间与修改时间/创建时间区间过滤（`mtime_after/before`、`ctime_after/before`，格式 `YYYY-MM-DD`，闭区间按当天 00:00 到 +24h 计算）。

**模式匹配引擎**（`pattern_matcher.rs`）：

- include/exclude 规则先尝试作为正则编译（忽略大小写），失败则按 glob 语法转换（`glob_to_regex`：`*`→`.*`，`?`→`.`，特殊字符转义）后再编译，均整体锚定 `^(?:...)$ `。
- rename 规则**不**加大小写忽略、**不**整体锚定（保留原始 `Regex::new`，支持部分匹配和捕获组替换，如 `$1[compress]$2`），用于把匹配到的原文件名转换成压缩后的目标文件名。
- `has_cycle_risk`：识别"重命名后的文件名仍会被 include 规则重新匹配"的情况（例如 rename 规则没生效导致文件名不变，或新文件名恰好也满足 include 且没被 exclude 排除），标记为循环风险，提示用户此文件可能被反复压缩。

**参数模板管理**（`template_manager.rs`）：`resolve(name_or_params)` — 若传入的字符串匹配某个模板名则返回该模板的 ffmpeg 参数，否则原样返回（即视为用户自定义的裸参数）。这是"选择模板 vs 自定义参数"两种模式共用同一个字段（`params`）的关键：`use_custom_params` 只是前端展示逻辑的开关，实际压缩时后端统一走 `resolve`。

### 4.5 文件扫描（`scanner.rs`）

- `scan()`：递归遍历目录（`walkdir` via `fs_util::list_files_recursive`），对每个文件计算相对路径、判断是否通过 `passes_filters`，并预计算：
  - `temp_name`：压缩时的临时输出文件名（原文件名插入 `_tmp` 后缀，在扩展名前）；
  - `final_name`：应用 rename 规则后的最终文件名；
  - `cycle_risk`：是否存在循环压缩风险。
- Windows 下 `std::fs::canonicalize` 会产生 `\\?\` 冗长路径前缀，`strip_verbatim_prefix` 负责去掉它——因为 ffmpeg 命令行不认这个前缀。
- `compute_next_run_set(files, max_compress_size_bytes)`：先按相对路径排序保证确定性顺序，再按累计大小裁剪出这一批实际会压缩的文件（未超限的进 `in_run`，超限部分进 `skipped`，留给下次运行）。当 `max_compress_size_bytes` 为 `None` 或 0 时，全部文件都在本次范围内。

### 4.6 压缩引擎（`compressor/`）

**`engine.rs`**：

- `compress()` 拼装 ffmpeg 命令：`-i <输入> <用户参数> -y <输出临时文件>`，调用 `run_process` 执行。
- `run_process` 用 `Stdio::null()` 屏蔽 stdin/stdout/stderr，**这是关键的稳定性细节**：如果不这样做，ffmpeg 向 stderr 高速写入进度信息，而父进程未读取管道会导致 OS 管道缓冲区写满，进而挂死子进程；同时把 stdin 设为 null 避免继承 GUI 父进程无效的 stdin 句柄导致某些 Windows 环境下挂起。
- 用轮询 `try_wait()` + `sleep(100ms)` 实现超时和取消检测（而非阻塞 `wait()`），返回码：`-1` 启动失败，`-2` 超时，`-3` 用户取消。
- Windows 下用 `CREATE_NO_WINDOW` (0x08000000) 标志隐藏 ffmpeg 控制台窗口弹出。
- `probe_ffmpeg()`：用 `Command::output()`（内部自带读取线程，不会有管道死锁问题）执行 `ffmpeg -version`，从输出中解析版本号字符串，用于设置里显示"ffmpeg 就位"状态。

**`file_compare.rs`**：

- `compare_and_cleanup()`：比较压缩前后文件大小 → 若 ffmpeg 退出码非 0，判定失败，删除临时文件，保留原文件；若压缩后更大，判定 `SkippedLarger`，丢弃压缩结果保留原文件；若压缩后更小，判定 `Success`：删除原文件、去掉临时文件名后缀、按最终路径（含 rename 规则）改名落地。

### 4.7 日志系统（`logger.rs`）

- 每次运行生成一个文件 `<dir>/.autocompress/logs/run_<时间戳>.log`，内容为**人类可读文本 + 尾部一段 JSON 块**（用 `--- JSON ---` / `--- JSON END ---` 标记边界）。
- 人类可读部分：逐文件的 emoji 状态行（✅ 成功 / ⏭ 更大丢弃 / ❌ 失败 / ℹ 跳过）+ 汇总统计。
- JSON 块：给程序自己读取用的机器可读格式（snake_case 字段），`read_history()` 从所有历史日志文件里提取 JSON 块反序列化为 `RunSummary` 列表，按文件名时间戳倒序返回（最新的在前）——这是"执行历史" Tab 的数据来源，同时也是**应用重启后卡片"上次运行"信息的兜底数据源**（内存缓存 `last_runs` 在重启后为空，此时读历史日志补上）。
- `clean_old_logs(retention_days)`：启动时按全局配置的保留天数清理过期日志文件。

### 4.8 工具函数（`util/`）

- `fs_util.rs`：递归列文件、带重试的安全删除（应对 Windows 文件锁延迟释放，最多重试 5 次，间隔 120ms）、安全改名、查询磁盘剩余空间（`fs4`）、全局配置目录定位（`%APPDATA%/AutoVideoCompressor`，无该环境变量则退回 `%TEMP%`）。
- `string_util.rs`：`glob_to_regex`（glob→regex 字符转义规则）、`insert_temp_suffix`/`remove_temp_suffix`（在扩展名前插入/去除 `_tmp`）、`format_file_size`（B/KB/MB/GB/TB，一位小数）、ISO8601 本地时间格式化。

### 4.9 错误处理（`error.rs`）

`AppError { message: String }` 实现 `Serialize`，作为所有 Tauri command 的统一错误类型（`AppResult<T> = Result<T, AppError>`）——Rust 侧返回 `Err` 时，前端 `invoke()` 的 Promise 会以该结构体的 JSON 形式 reject，前端直接 `catch` 后 `String(e)` 展示。

---

## 5. 前端（Vue 3）核心实现

### 5.1 启动流程

```
index.html (#app)
  → src/main.ts: createApp(App).use(createPinia()).mount("#app")
    → App.vue：仅提供 Naive UI 的 ConfigProvider / MessageProvider / DialogProvider 外壳
      → AppShell.vue（provider 的后代组件，可以正确 inject 出 useDialog()/useMessage()）
        → onMounted: store.init()
            1. api.listDirectories() → 填充卡片列表
            2. api.getFfmpegStatus() → 填充 ffmpeg 状态
            3. 注册 4 个事件监听（各自独立 try/catch，一个失败不影响其余）
        → 渲染 TitleBar + (DirectoryList | DirectoryDetail) + GlobalSettings(条件显示)
```

> **重要实现细节**：`useDialog()`/`useMessage()` 必须在 Provider 的**后代组件**里调用才能正确 `inject` 到实例，因此 `App.vue` 本身不调用这两个 hook，而是把内容全部下沉到 `AppShell.vue`。`AppShell` 拿到的 dialog/message 实例被挂到 `window.$dialog` / `window.$message`，供未使用 Composition API 的深层子组件（如 `DirCard.vue` 的事件处理函数）直接调用，替代浏览器原生 `alert`/`confirm`。

### 5.2 页面路由

没有引入 vue-router，用一个简单的 `ref<"list" | "detail">` 状态在 `AppShell.vue` 里手动切换两级页面：

- **一级（DirectoryList.vue）**：展示所有监控目录的卡片列表（`DirCard.vue`），可添加新目录（调用系统目录选择器 `@tauri-apps/plugin-dialog` 的 `open()`）。
- **二级（DirectoryDetail.vue）**：单个目录详情，内含三个 Tab（预览/配置/历史）。

### 5.3 状态管理（`stores/app.ts`）

单一 Pinia store `useAppStore`，持有：

- `cards`：一级页面所有目录卡片数据（`DirCardInfo[]`）；
- `ffmpeg`：ffmpeg 可用性状态；
- `runtime`：按目录路径索引的运行时状态（`Record<string, DirRuntimeState>`）；
- `progress`：按目录路径索引的压缩进度；
- `compressingWhileClose`：是否弹出"压缩中确认退出"对话框。

`refreshCards()` 每次都重新拉取全部卡片（后端 `list_directories` 会重新扫描磁盘、重算下次运行时间等），在 `dir-state-changed` 事件回调里也会调用它，保证阶段变化时卡片信息（如"上次运行结果"）同步刷新。

### 5.4 关键组件设计要点

**`ConfigForm.vue`**（目录级配置表单，最复杂的组件）：

- 采用**严格单向数据流**：不持有本地副本、不用任何 `watch` 去把 `props.modelValue` 同步回本地 state。任何字段变化都是"基于 `props.modelValue` 展开拷贝 + 改该字段 + `emit('update:modelValue', ...)`"。这是为了避免父子之间"父 emit → 子 watch 回填 → 再次触发子 emit"的死循环，该循环曾导致输入框刚输入就失焦还原（组件注释里明确记录了这一坑）。
- 重命名规则数组（`n-dynamic-input`）的更新是**例外**：`updateRename()` 直接就地修改 `props.modelValue.renameRules[index][key]`，而不是整体替换新数组。原因是 Naive UI 的 `DynamicInput` 内部按**对象引用**分配 Vue `key`；若整体替换为新对象引用，该行会被判定为"新增行"从而销毁重建 DOM，导致输入框每敲一个字符就失焦。Vue 的 `props` 虽是 `shallowReadonly`，但嵌套对象允许就地修改，父组件持有同一响应式对象引用，保存时能读到最新值。
- `use_custom_params` 开关在"选择模板"与"自定义 ffmpeg 参数"两种模式间切换：切到自定义时，先把当前选中模板解析出的实际参数字符串填进 `params` 字段（而不是留模板名），确保用户能在此基础上手改；切回模板模式则把 `params` 重置为模板列表第一项的名称。

**`TabConfig.vue`**：

- 用 1.5 秒轮询 `getConfigMtime` 检测磁盘上配置文件是否被外部程序（如用户点了"打开外部编辑器"手动改了 JSON）修改，mtime 变化才重新 `load()`，避免用户正在编辑时被覆盖（未保存不会触发 mtime 变化）。
- 配置文件不存在时只显示"创建默认配置文件"按钮，创建后才展示完整 `ConfigForm`。

**`DirCard.vue`**：

- 五种状态徽标（`valid`有效已排程 / `unscheduled`有效未排程 / `invalid`无配置文件 / `config_error`配置解析错误 / `overlap`路径与其他目录重叠），由后端 `compute_badge` 计算好直接传下来，前端只做展示映射。
- 删除目录时如果目录正在压缩会用不同的确认文案提示"将中断当前任务"，确认后传 `force=true` 给后端跳过运行中检查。

**`TitleBar.vue`**：因窗口是无边框（`tauri.conf.json` 中 `decorations: false`），标题栏是纯前端实现的可拖拽区域（`getCurrentWindow().startDragging()`），并手写了最小化/最大化/关闭按钮调用 `@tauri-apps/api/window` 的窗口控制 API。用 `"__TAURI_INTERNALS__" in window` 判断是否运行在 Tauri 容器里，纯浏览器开发调试时隐藏窗口控制按钮。

### 5.5 类型映射约定

前端 `types.ts`（camelCase）与 Rust `types.rs`（Rust 端用 `#[serde(rename_all = "camelCase")]`）严格对应，靠 serde 在 JSON 序列化边界自动转换字段命名风格（Rust 内部字段名是 snake_case，序列化成 JSON 时转 camelCase）。两侧类型定义需要手动保持同步——没有自动生成绑定的工具链。

---

## 6. 桌面集成特性

### 6.1 系统托盘

`main.rs` 的 `.setup()` 里用 `TrayIconBuilder` 创建托盘图标 + 右键菜单（显示主窗口 / 退出）。若全局配置 `minimize_to_tray` 为真，点击关闭按钮时 `on_window_event` 拦截 `CloseRequested`，调用 `api.prevent_close()` + `window.hide()`，应用继续在后台运行。

### 6.2 压缩中关闭窗口的确认流程

`on_window_event` 检测到 `compressor_running` 为真时，同样 `prevent_close()`，并 `emit("close-requested-while-compressing", ())` 通知前端；前端在 `AppShell.vue` 弹出确认弹窗，用户选择"强制退出"则调用 `getCurrentWindow().destroy()` 直接销毁窗口（跳过所有拦截逻辑）。

### 6.3 开机自启动

`tauri_plugin_autostart` 注册 Windows 启动项，由全局配置 `start_with_windows` 字段控制（保存全局配置时同步调用插件 API，具体开关逻辑封装在插件内部）。

### 6.4 命令行启动模式

- `--run-once [--directory <path>]`：手动 headless 模式，跳过 Tauri GUI；指定目录时只处理该目录，否则处理所有已启用目录，完成后退出。
- `--scheduled --directory <path>`：Windows 计划任务专用模式，必须指定且只排队该目录。若应用未运行则正常初始化 GUI；若已有实例则通过 pending IPC 提交目录请求，已有窗口会显示并获得焦点。请求在前端注册完状态/进度监听后启动，因此可查看逐文件进度并通过现有停止命令中断。

---

## 7. 关键配置文件

### 7.1 `src-tauri/tauri.conf.json`

```json
{
  "productName": "AutoVideoCompressor",
  "identifier": "com.autovideocompressor.app",
  "build": {
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "npm run build",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [{ "title": "AutoVideoCompressor", "width": 900, "height": 700, "decorations": false }]
  },
  "bundle": { "active": true, "targets": "all", "icon": [...] }
}
```

无边框窗口（`decorations: false`）配合自定义 `TitleBar.vue` 实现统一风格标题栏；`bundle.targets: "all"` 使 `tauri build` 同时产出 NSIS、MSI 与便携版 exe。

### 7.2 `vite.config.ts`

- 固定开发端口 `1420`（Tauri 的 `devUrl` 与之对应，改动需两处同步），HMR 端口 `1421`。
- `unplugin-auto-import` 自动导入 `vue`/`vue/macros` 的常用 API（`ref`/`computed`/…），生成 `src/auto-imports.d.ts`。
- `unplugin-vue-components` + `NaiveUiResolver` 自动按需注册 Naive UI 组件（模板里用 `<n-button>` 无需手动 `import`），生成 `src/components.d.ts`。
- `watch.ignored` 忽略 `src-tauri/**`，避免 Rust 编译产物触发前端 HMR。

### 7.3 `tsconfig.json`

`strict: true`，路径别名 `@/* → ./src/*`，`moduleResolution: bundler`（配合 Vite）。

### 7.4 `src-tauri/Cargo.toml`

`[lib] name = "autovideocompressor_lib"` 与 `[[bin]] name = "autovideocompressor"` 分离——业务逻辑编译为库（`lib.rs` 导出所有模块），`main.rs` 只是薄的可执行入口，这样 Rust 单元测试（`#[cfg(test)]` 分散在各模块文件内）可以直接对库做白盒测试，无需启动完整 Tauri 应用。

---

## 8. 构建与开发

```bash
# 安装依赖
npm install

# 开发模式（同时跑 Vite dev server + cargo build + 启动窗口，支持前端热更新）
npm run tauri dev

# 生产构建（前端 vue-tsc 类型检查 + vite build，再 tauri build 打包，最后重命名 exe）
npm run build:tauri

# 前端组件测试
npm test          # vitest run

# Rust 单元测试
cd src-tauri && cargo test
```

产物位置：
- 便携版：`src-tauri/target/release/AutoVideoCompressor_<version>.exe`（由 `scripts/rename-exe.cjs` 从 `autovideocompressor.exe` 复制重命名而来）
- NSIS 安装包：`src-tauri/target/release/bundle/nsis/`
- MSI 安装包：`src-tauri/target/release/bundle/msi/`

前置环境：Rust（MSVC 工具链）、Node.js 18+、Visual Studio Build Tools（"使用 C++ 的桌面开发"组件）。

---

## 9. 设计要点小结

1. **业务逻辑单向下沉到 Rust**：前端不做任何文件系统/进程操作，只负责渲染和收集输入，所有状态的唯一真源在后端（内存态 `AppCore` + 磁盘配置/日志文件）。
2. **配置分层**：全局配置管理"监控哪些目录、公共参数模板、ffmpeg 路径等应用级设置"；每个目录的匹配规则/调度时间/压缩参数独立存放在目录自身的 `.autocompress/config.json` 里，方便迁移目录时配置随目录走。
3. **严格串行 + 不补跑调度**：避免并发压缩导致资源竞争或磁盘 I/O 过载；调度错过就跳到下一天，不会因程序离线而在恢复后连续触发多次压缩风暴。
4. **双格式日志**：人类可读文本方便直接打开排查，尾部 JSON 块供程序自身解析出历史记录，两者共存于同一文件，无需额外的结构化日志库。
5. **文件安全性优先**：压缩失败/取消都会保留原文件不动，只有确认压缩后文件更小才会替换；Windows 下的文件锁竞争通过"重试删除"规避，而非报错中断。
