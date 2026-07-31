# AutoVideoCompressor

基于 **Tauri 2 + Rust + Vue 3** 的 Windows 视频压缩桌面工具。自动扫描指定目录下的视频文件，调用 ffmpeg 压缩，保留更小的文件。

应用、包、可执行文件和全局数据目录统一使用 **AutoVideoCompressor**；每个监控目录内的配置子目录为兼容已有配置，仍保留名称 `.autocompress`。

> 技术栈、代码结构及关键实现细节见 [docs/PROJECT.md](docs/PROJECT.md)。

## 功能

- **Per-directory 独立配置** — 每个目录拥有独立的 include/exclude 规则、过滤器、重命名规则、调度时间
- **每日定时调度** — 可选择应用内调度或 Windows 计划任务；后者会启动或唤醒 GUI 并展示对应目录的压缩进度
- **串行执行** — 同一时间只压缩一个目录，完成后自动处理队列中的下一个
- **实时进度推送** — 逐文件压缩进度，支持手动停止
- **循环压缩检测** — 自动识别重命名后再次匹配的白名单文件，标记风险
- **全局参数模板** — 可自定义 H.265/H.264 预设模板，按目录选用或自定义
- **单次压缩总大小限制** — 按文件顺序累计，超出部分自动跳过并在预览中标注
- **压缩历史日志** — 人类可读文本 + 机器可读 JSON 块，保留天数可配
- **压缩文件预览** — 显示匹配文件、压缩后文件名、大小、循环风险、单次限制状态
- **系统托盘常驻** — 最小化到托盘、右键菜单（显示主窗口 / 退出）
- **开机自启动**
- **原生 Windows 计划任务** — 通过系统 Task Scheduler COM API 管理任务，统一存放在 `\AutoVideoCompressor\` 文件夹中
- **`--run-once` 模式** — 供脚本手动调用的无窗口单次执行模式

## 截图

> 运行 `npm run tauri dev` 可查看实际界面。

## 安装

### 二进制安装

从 [Releases](../../releases) 下载最新版：
- `AutoVideoCompressor_x.x.x_x64-setup.exe` — NSIS 安装程序（推荐）
- `autovideocompressor.exe` — 便携版，无需安装

### 依赖

- **Windows 10/11 (x64)**
- 需安装 [ffmpeg](https://ffmpeg.org/download.html)，可以在 PATH 中使用，或在全局设置中指定路径

## 构建

### 前置条件

- [Rust](https://www.rust-lang.org/tools/install) (MSVC 工具链)
  ```bash
  scoop install rustup
  rustup default stable-x86_64-pc-windows-msvc
  ```
- [Node.js](https://nodejs.org/) 18+
- [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/) ("使用 C++ 的桌面开发" 组件)

### 开发运行

```bash
git clone https://github.com/guyu-guyu/AutoVideoCompressor.git
cd AutoVideoCompressor
npm install
npm run tauri dev
```

### 生产构建

```bash
npm run build:tauri
```

产物：
- `src-tauri/target/release/AutoVideoCompressor_x.x.x.exe` — 便携版
- `src-tauri/target/release/bundle/msi/` — MSI 安装包
- `src-tauri/target/release/bundle/nsis/` — NSIS 安装包

## 技术栈

| 层级 | 技术 |
|------|------|
| 框架 | Tauri 2.x |
| 后端 | Rust（业务逻辑全部在 Rust 层） |
| 前端 | Vue 3 + TypeScript + Vite |
| UI 库 | Naive UI |
| 平台 | Windows 10/11 (x64) |

## 配置文件

### 全局配置

`%APPDATA%/AutoVideoCompressor/config.json`（扁平格式）：

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

### 目录级配置

`<目录>/.autocompress/config.json`：

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

## 命令行

```bash
# 无窗口运行一次（对所有已启用目录执行压缩，完成后退出）
autovideocompressor.exe --run-once

# 无窗口运行一次（仅压缩指定目录）
autovideocompressor.exe --run-once --directory "D:\Videos"
```

Windows 计划任务统一创建在任务计划程序库的 `\AutoVideoCompressor\` 文件夹中，动作参数为 `--scheduled --directory <path>`。该模式会显示 GUI、自动进入对应目录，并支持查看进度或手动停止。
计划任务使用 Windows 交互模式运行，因此需要用户处于登录状态。全局设置中启用“唤醒计算机执行任务”后，所有应用计划任务都会开启 `WakeToRun`，在计划时间唤醒处于睡眠状态的计算机运行任务。

## 开发

```bash
npm test                    # 前端组件测试 (Vitest)
cargo test                  # Rust 单元测试
```

## 许可

MIT
