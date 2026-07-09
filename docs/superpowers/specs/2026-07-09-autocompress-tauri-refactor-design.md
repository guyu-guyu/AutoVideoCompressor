# AutoCompress → Tauri 重构设计文档

> 将 C++/Dear ImGui 桌面应用 AutoCompress 以 Tauri (Rust) + Vue 3 + TypeScript 技术栈重构
> WebView 渲染前端,功能与原项目完全相同
> 日期:2026-07-09
> 源项目:`F:\Projects\AutoCompress`(C++20 + Dear ImGui + SDL2 + OpenGL)

---

## 0. 背景与目标

AutoCompress 是一个 Windows 桌面工具:自动扫描指定目录下的视频文件,调用 ffmpeg 压缩,保留压缩效果更好(更小)的文件。支持 per-directory 配置、per-directory cron 调度(每天 HH:MM)、per-directory 日志、循环压缩检测、系统托盘常驻、开机自启动、`--run-once` 命令行(供 Windows 任务计划程序调用)。

本设计将其**整体重构**到新技术栈,**功能完全对齐原项目**,不新增也不删减功能。配置文件磁盘格式保持不变。

### 技术栈决策

| 层级 | 选择 | 说明 |
|------|------|------|
| 后端 | **Tauri 2.x + Rust** | 单文件小体积 exe,接近原生资源占用,与原项目"轻量单 exe"哲学最契合 |
| 前端 | **Vue 3 + TypeScript + Vite** | WebView 内渲染 UI,响应式,适合两级界面 + 表单 + 表格 |
| 前端渲染 | 系统 WebView2 | Windows 11 内置,无需打包 Chromium |
| 平台 | **仅 Windows** | 与原项目一致,专注 Win 10/11 |
| 分发 | 单 exe(+ WebView2 运行时) | 对齐原项目单 exe 分发 |

### 职责划分决策(方案 A:胖后端 + 瘦前端)

所有业务逻辑用 Rust 实现,与原 C++ 模块一一对应;Vue 前端只负责 UI 渲染 + 通过 Tauri `command` 调用后端、通过 `event` 接收推送。选择理由:与原 C++ 架构模块边界几乎 1:1 对应,迁移路径最清晰,"完全相同的功能"最易验证;业务逻辑集中于一层便于测试(对齐原 doctest)。

---

## 1. 整体架构与项目结构

### 1.1 分层

```
瘦 Vue 前端(纯 UI 渲染)
      ⇅  Tauri IPC(command 请求/响应 + event 推送)
胖 Rust 后端(全部业务逻辑)
      ⇅
文件系统(config.json / .autocompress/config.json / logs) + ffmpeg 子进程
```

原 ImGui 是 immediate-mode(每帧重绘,状态全在 C++)。Vue 是 retained-mode + 响应式,因此运行态改为:**Rust 持有真值 → 通过 event 增量推送 → Pinia store 缓存 → Vue 响应式渲染**,替代原"worker 线程 pushState + UI 每帧读取"模式。

### 1.2 Rust 后端模块(对齐原 C++ 模块)

```
src-tauri/src/
├── main.rs                 # 入口:CLI 解析(--run-once)、tray、autostart、启动 App
├── app.rs                  # App 状态编排:per-directory 执行管线、串行锁、状态推送
├── config/
│   ├── config_manager.rs   # 全局 config.json 读写           (← ConfigManager)
│   ├── directory_config.rs # 目录级 .autocompress/config.json (← DirectoryConfig)
│   ├── template_manager.rs # ffmpeg 参数模板                  (← TemplateManager)
│   └── pattern_matcher.rs  # glob→正则 匹配引擎               (← PatternMatcher)
├── scanner/file_scanner.rs # 递归扫描 + include/exclude/filters (← FileScanner)
├── compressor/
│   ├── engine.rs           # 组装 ffmpeg 命令、起子进程、超时终止 (← CompressionEngine)
│   └── file_compare.rs     # 比较大小、保留较小者、去 _tmp 后缀   (← FileCompare)
├── scheduler.rs            # 时间表轮询线程(per-directory cron)  (← Scheduler)
├── logger.rs               # per-directory 日志(文本 + JSON 块)  (← Logger)
├── ffmpeg_probe.rs         # 异步 ffmpeg -version 探测            (← FfmpegStatusBar 逻辑)
├── types.rs                # 共享结构体(serde,对齐前端 TS 类型)  (← Types.h)
└── commands.rs             # Tauri command 层:前端可调用的 API 边界
```

### 1.3 Vue 前端结构(对齐原 UI 组件)

```
src/
├── main.ts, App.vue        # 挂载、路由(两级界面)
├── views/
│   ├── DirectoryList.vue   # 一级界面:ffmpeg 状态条 + 目录卡片列表 (← PageLevel1)
│   └── DirectoryDetail.vue # 二级界面:详情摘要 + 三 tab            (← PageLevel2)
├── components/
│   ├── DirCard.vue         # 目录卡片(← DirCard)
│   ├── FfmpegStatusBar.vue # ffmpeg 状态条(← FfmpegStatusBar)
│   ├── ConfigForm.vue      # 配置表单(← ConfigForm)
│   ├── tabs/
│   │   ├── TabPreview.vue  # 压缩文件预览(← TabPreview)
│   │   ├── TabConfig.vue   # 配置(← TabConfig)
│   │   └── TabHistory.vue  # 压缩执行历史(← TabHistory)
│   └── GlobalSettings.vue  # 全局设置弹窗,含模板管理(← GlobalSettingsWindow)
├── stores/                 # Pinia:目录运行态、全局配置、ffmpeg 状态
├── api/tauri.ts            # invoke 封装 + event 监听封装
└── types.ts                # 与 Rust types.rs 对齐的 TS 类型
```

### 1.4 两级界面(对齐原项目)

- **一级界面(DirectoryList)**:菜单栏(设置入口)→ ffmpeg 状态条(独立一行,含重新检测按钮)→ 目录列表(添加目录 + 卡片列表)。
- **卡片(DirCard,布局 A 紧凑两行)**:标题行(路径 + 状态徽章 + 启用开关 + ▶立即压缩按钮)→ 汇总行(文件数/大小/参数)→ 警告行(循环风险)→ 底部分隔区:左"上次压缩"、右"下次压缩"。整卡可点击进入二级,立即压缩按钮单独消费点击。
- **二级界面(DirectoryDetail)**:返回栏(路径 + 状态徽章 + 启用开关)→ 详情摘要(匹配/大小/参数/调度/循环风险 + 上次/下次)→ 三 tab(压缩文件预览[默认] / 配置 / 压缩执行历史)+ "立即压缩此目录"按钮。
- **全局设置弹窗**:ffmpeg 路径/超时、模板增删改、日志保留天数、语言、开机启动、最小化到托盘。

---

## 2. IPC 接口设计(command + event)

方案 A 的核心边界。

### 2.1 Commands(前端 invoke → Rust,请求/响应)

**全局配置**
- `get_global_config() -> GlobalConfig`
- `save_global_config(config: GlobalConfig) -> Result<()>` — 改动后触发调度表刷新
- `list_templates() -> Template[]`
- `save_template(template: Template) -> Result<()>`
- `delete_template(name: String) -> Result<()>`

**目录管理**
- `list_directories() -> DirCardInfo[]` — 每个目录的卡片聚合信息
- `add_directory(path: String) -> Result<()>`
- `remove_directory(path: String, force: bool) -> Result<()>` — 正在压缩需 force
- `set_directory_enabled(path: String, enabled: bool) -> Result<()>`

**目录级配置(TabConfig)**
- `get_directory_config(path: String) -> DirConfigView` — 不存在则返回默认模板 + `exists:false`
- `save_directory_config(path: String, config: DirConfig) -> Result<()>` — 校验并写回,失败返回结构化错误
- `create_directory_config(path: String, config: DirConfig) -> Result<()>` — 首次创建(建 `.autocompress/` 目录)
- `get_config_mtime(path: String) -> u64` — TabConfig 轮询用
- `open_config_in_editor(path: String) -> Result<()>` — ShellExecute 打开外部编辑器

**预览与历史**
- `scan_directory(path: String) -> FilePreview[]` — 匹配文件(原文件/压缩后名/大小/风险)
- `list_run_history(path: String) -> RunSummary[]` — 读日志解析 JSON 块,倒序

**执行控制**
- `compress_directory_now(path: String) -> Result<()>` — 受串行锁约束,占用中返回忙
- `recheck_ffmpeg() -> Result<()>` — 手动重新探测

### 2.2 Events(Rust → 前端,主动推送)

- `dir-state-changed` → `DirRuntimeState` — 目录进入 扫描/压缩/完成/空闲,或上次/下次时间更新
- `compress-progress` → `{ dirPath, currentFile, completed, total }` — 逐文件进度
- `ffmpeg-status-changed` → `{ ready, version, error }` — 探测结果(启动异步探测完成 / 重新检测 / 压缩出错)

### 2.3 外部修改检测(mtime)

采用**前端轮询**:TabConfig 可见时定时(每 1-2 秒)调 `get_config_mtime` 比较,变化则重载。理由:更简单、只在 TabConfig 打开时才轮询、无需引入文件监听依赖,行为与原项目一致。

---

## 3. 并发模型 · 调度器 · 串行锁(Rust 落地)

### 3.1 共享状态

```rust
struct AppState {
    config: Mutex<GlobalConfig>,              // 全局配置
    scheduler: Scheduler,                     // 内含时间表
    compressor_running: Arc<AtomicBool>,      // 全局串行锁(← m_compressorRunning)
    app_handle: AppHandle,                    // emit event 给前端
}
```

Tauri `app.manage(AppState)` 注入,command 通过 `State<AppState>` 访问。

### 3.2 调度器(scheduler.rs)

- 后台**一个**线程,持有 `Vec<DirSchedule>`(dirPath + enabled + nextRun),`Mutex` 保护。
- 每秒轮询:找 `now >= nextRun && enabled` 的目录 → 回调触发压缩。
- 触发后该目录 `nextRun` 推到**明天同时间点**(严格不补跑,假定电脑常开)。
- `set_directories()`:配置变更后刷新时间表(← `refreshScheduleTable`)。
- `seconds_until_next_run(path)` / `next_run(path)`:供 `list_directories` 计算"下次时间"。

```rust
struct DirSchedule {
    dir_path: String,
    enabled: bool,
    next_run: SystemTime,   // 从该目录 config 的 schedule.time 推算
}
```

### 3.3 串行执行(app.rs)

```
调度到点 或 前端 compress_directory_now(path)
  → 检查 compressor_running:
      已 true → 跳过/返回忙(不排队)
      false  → set(true),spawn 线程跑 execute_for_directory(path)
  → execute_for_directory:
      1. 加载 <dir>/.autocompress/config.json(无效则跳过)
      2. 重叠检测(被其他目录包含则跳过)
      3. file_scanner 扫描
      4. 逐文件:算最终名 → cycle 检测 → engine 起 ffmpeg → file_compare
         每个文件后 emit compress-progress
      5. logger 写 <dir>/.autocompress/logs/run_*.log
      6. emit dir-state-changed(更新上次/下次时间)
      7. compressor_running.set(false)
```

**队列语义**(对齐原设计):A 压缩时 B 到点 → B 因锁被跳过,但 B 的 `next_run` 未重置 → 每秒重检 → A 完成锁释放的下一秒 B 即触发。等价于"A 完成后立即跑 B",无需显式队列。

### 3.4 单文件压缩生命周期(对齐原项目)

```
① 计算最终文件名:原 教程.mp4 → 临时 教程_tmp.mp4(硬编码 _tmp 插在扩展名前)
   最终名 = 应用 rename_rules(无规则则原名)
② 循环压缩风险检测:最终名再次匹配 include? 或 无重命名? → 标记 cycleRisk(不阻止)
③ engine 起 ffmpeg:教程.mp4 → 教程_tmp.mp4(可超时终止)
   失败 → 删 tmp,记错误
④ file_compare:tmp < 原? → 删原、tmp 去 _tmp 后缀(或按 rename_rules 重命名)
                否       → 删 tmp,保留原,日志"压缩后更大,已丢弃"
```

### 3.5 ffmpeg 子进程(engine.rs)

- Rust `std::process::Command`(← Win32 `CreateProcess`),输出到 `*_tmp.<ext>`。
- 超时:监控线程,超时则 `child.kill()` + 删 tmp + 记错误(← `WaitForSingleObject` 超时)。
- Windows 下加 `CREATE_NO_WINDOW` flag 避免弹黑框。

### 3.6 ffmpeg 探测(ffmpeg_probe.rs)

- 执行 `ffmpeg -version` 解析 version。检查时机:①程序启动异步探测一次(缓存);②压缩出错发现不可用时;③前端手动"重新检测"。
- 每次结果 emit `ffmpeg-status-changed`。失败显示原因(路径不存在 / 启动失败 / version 解析失败)。

### 3.7 系统集成(main.rs)

- **系统托盘**:Tauri tray API,右键菜单(显示主窗口 / 立即压缩 / 退出);最小化到托盘常驻。
- **开机自启动**:Tauri autostart 插件;启动后隐到托盘。
- **`--run-once` 命令行**:启动时解析参数,若含 `--run-once` 则不建窗口,立即对所有已配置且有效的目录跑一次压缩循环,完成后退出(供任务计划程序调用)。
- **退出时正在压缩**:`on_window_event` 拦截关窗 → 前端弹确认框(等待/强制退出)。

---

## 4. 数据模型 · 配置文件 schema · 类型对齐

配置文件磁盘格式与原项目**完全一致**,保证行为相同。

### 4.1 全局 config.json(程序目录或 %APPDATA%)

```jsonc
{ "version": 1,
  "general": { "language", "minimize_to_tray", "start_with_windows", "log_retention_days" },
  "ffmpeg": { "path", "timeout_seconds" },
  "templates": [ { "name", "params" } ],
  "directories": [ { "path", "enabled" } ] }
```

### 4.2 目录级 `<dir>/.autocompress/config.json`

```jsonc
{ "include": ["*.mp4","*.mov","*.avi","*.mkv"], "exclude": ["*_tmp.*"],
  "filters": { "max_size_mb", "min_size_mb", "mtime_after", "mtime_before", "ctime_after", "ctime_before" },
  "rename_rules": [ { "pattern": "^(.+)\\.mp4$", "replacement": "$1_compressed.mp4" } ],
  "params": "H.265 高质量",
  "schedule": { "time": "03:00" } }
```

目录结构:
```
<目录>/
├── .autocompress/
│   ├── config.json
│   └── logs/run_YYYY-MM-DD_HH-MM-SS.log   ← 每次执行一个文件,不合并
└── 视频文件...
```

### 4.3 日志格式(与原项目一致)

人类可读文本 + 末尾以 `--- JSON ---` / `--- JSON END ---` 包围的机器可读 JSON 块。JSON 块含:runId、起止时间、时长、directories(每次只跑一个目录)、files[]、summary(各类计数 + 总节省字节)。日志保留按目录各自清理,retention 取全局 `log_retention_days`,启动时清理过期。

### 4.4 Rust ⇄ TS 类型对齐

- 跨 IPC 结构体在 `types.rs` 用 `#[derive(Serialize, Deserialize)]` + `#[serde(rename_all = "camelCase")]`,Rust `snake_case` 字段在前端呈现为 `camelCase`。
- 磁盘 JSON 字段名(`max_size_mb`、`rename_rules` 等)保持原样:用 serde 字段级 `rename` 精确控制,**磁盘格式不受 camelCase 影响**。
- 前端 `src/types.ts` 手写对应 interface(项目规模不大,手写比引入 ts-rs 代码生成更简单;后续若嫌重复可加 ts-rs)。

### 4.5 关键运行态类型(对齐原 Types.h)

- `FileStatus` enum:Success / SkippedLarger / Failed / SkippedOther
- `FileResult`:原名、最终名、状态、原/压缩大小、节省字节、ffmpeg 退出码/耗时、cycleRisk
- `RunSummary`:runId、起止时间、时长、目录结果、文件列表、各类计数、总节省、循环风险数(含 `compute_totals()`)
- `DirRuntimeState`:dirPath、stage(Idle/Scanning/Compressing/Completed)、currentFile、completed/total、lastRunTime/Result、nextRunTime
- `DirCardInfo`:卡片展示聚合(状态徽章、文件数/大小、参数名、循环风险数、上次/下次)

### 4.6 有效性 / 状态徽章规则(对齐原项目)

- `include` 非空 = 有效;有效但无 `schedule.time` = **有效(未排程)**
- 配置缺失 = **无效(无配置文件)**;解析失败 = **配置错误** + 行号
- 被其他目录包含 = **重叠(被 X 包含)**,跳过调度

### 4.7 匹配引擎规则(对齐原项目)

- 每条规则为正则(支持常见 glob 自动转换);匹配对象为相对根目录的文件路径。
- 黑名单优先级 > 白名单;`filters` 按大小/时间过滤。
- 循环压缩检测:`finalName == originalName` 或 `finalName 匹配任一 include` → cycleRisk。

---

## 5. 错误处理 · 测试策略

### 5.1 错误处理矩阵(对齐原项目)

| 场景 | 处理方式 | 用户可见反馈 |
|------|----------|-------------|
| ffmpeg 未就位 | 状态条红字显示原因;禁用立即压缩/调度触发 | `ffmpeg-status-changed` → 状态条 ●不可用 + 原因 |
| ffmpeg 执行超时 | `child.kill()` + 删 tmp + 记错误 | 日志 [超时] + 历史 ❌ |
| ffmpeg 返回非零 | 读 stderr,删 tmp | 日志记录 ffmpeg 错误输出 |
| 目录不存在 | 标记无效,跳过调度 | 卡片 [无效] + 下次"—" |
| config.json 缺失 | 卡片 [无效],提示进二级创建 | 卡片 [无效] + 详情提示 |
| config.json 解析失败 | 跳过该目录,记错误 | 卡片 [配置错误] + 行号 |
| 未配 schedule.time | 不参与自动调度 | 卡片 [有效(未排程)] |
| 目录被重叠包含 | 跳过该目录调度 | 卡片 [重叠] + 下次"—" |
| 磁盘空间不足 | 删 tmp,跳过该文件 | 日志 [空间不足] |
| 文件被占用 | 跳过该文件,继续下一个 | 日志 [跳过-被占用] |
| config.json 外部修改 | 前端 mtime 轮询 → 自动重载覆盖表单 | 表单静默刷新(不弹确认) |
| 表单保存 JSON 校验失败 | 不写文件,command 返回 Err | 表单底部红字提示 |
| 退出时正在压缩 | 拦截关窗事件 | 前端弹窗确认(等待/强制退出) |
| 移除正在压缩的目录 | 需 force=true | 弹窗确认 → 中断并移除 |

Rust 侧统一 `Result<T, AppError>`,`AppError` 派生 `Serialize`,command 出错时前端 `invoke` catch 到结构化错误。

### 5.2 Rust 单元测试(对齐原 doctest 覆盖,`#[cfg(test)]` + `tempfile`)

- `directory_config`:load/save 往返一致(含 schedule.time);解析失败返回错误;缺 include 标记无效;缺 schedule.time = 有效未排程
- `scheduler`:多目录不同 time,验证 next_run 推算(今天未到=今天、已到=明天);到点触发对应目录;串行锁下 B 等 A
- `config_manager`:全局 config.json 往返;旧文件含已移除字段不崩(忽略未知字段)
- `logger`:日志写到 `<dir>/.autocompress/logs/`,JSON 块可解析出 startTime/summary
- `pattern_matcher`:glob→正则 转换、include/exclude 匹配、cycle 检测

### 5.3 集成 / 前端测试

- **dummy ffmpeg**:小脚本,被调用时输出 `ffmpeg version fake` 并产出固定大小文件,验证 engine 启动/超时/version 解析,不依赖真实 ffmpeg。
- **前端**:关键组件(DirCard 状态徽章、ConfigForm 校验)用 Vitest;两级导航、tray、autostart、`--run-once` 走手动 checklist。
- **验收基线**:以原项目 `docs/manual-verification-checklist.md` 移植为端到端验收清单,确保"功能完全相同"。

---

## 6. 实现顺序建议

1. **脚手架**:Tauri 2 + Vue 3 + TS + Vite 项目初始化;`types.rs` + `types.ts` 骨架;IPC 通路打通(一个 hello command + event)。
2. **数据层**:`config_manager`、`directory_config`(含 schedule)、`template_manager`、`pattern_matcher` + 单测。
3. **扫描/压缩层**:`file_scanner`、`engine`、`file_compare`、`logger`(per-directory)+ dummy ffmpeg 集成测试。
4. **调度/编排层**:`scheduler` 时间表轮询、`app` 串行执行管线、`ffmpeg_probe` 异步探测 + 单测。
5. **command/event 层**:`commands.rs` 全部接口接线。
6. **前端**:Pinia store + api 封装;一级界面(DirCard + FfmpegStatusBar);二级界面 + 三 tab(ConfigForm 含 mtime 轮询);全局设置弹窗。
7. **系统集成**:tray、autostart、`--run-once`、退出确认。
8. **验收**:移植手动验证清单,端到端对齐原项目。

---

## 7. 明确不做(对齐原项目 V1 范围)

以下为原项目 V2 推迟项,本次同样不实现:按星期调度、tab1 预览分页/搜索/排序、tab1 手动压缩单文件、通知(Toast/邮件/webhook)、日志统计图表、批量改配置、ffmpeg 实时进度读取、压缩队列优先级排序。

---

*本设计基于 2026-07-09 头脑风暴确认。功能范围以原项目 `F:\Projects\AutoCompress` 为准,完全对齐。后续实现计划由 writing-plans 技能基于此文档拆解。*
