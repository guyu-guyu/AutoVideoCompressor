import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  DirCardInfo, DirConfigView, FilePreview, RunSummary,
  FfmpegStatus, GlobalConfig, DirRuntimeState, CompressProgress,
} from "../types";

// 所有 Tauri 后端调用的统一出口：失败时记录命令名与原始错误，便于定位是哪条 invoke 出问题。
// 成功时不打日志（避免 getConfigMtime 这类 1.5s 轮询刷屏）。
async function call<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  try {
    return await invoke<T>(cmd, args);
  } catch (e) {
    console.error(`[tauri] invoke '${cmd}' 失败:`, e);
    throw e;
  }
}

export const api = {
  frontendReady: () => call<void>("frontend_ready"),
  listDirectories: () => call<DirCardInfo[]>("list_directories"),
  getGlobalConfig: () => call<GlobalConfig>("get_global_config"),
  saveGlobalConfig: (config: GlobalConfig) => call<void>("save_global_config", { config }),
  addDirectory: (path: string) => call<void>("add_directory", { path }),
  removeDirectory: (path: string, force: boolean) => call<void>("remove_directory", { path, force }),
  setDirectoryEnabled: (path: string, enabled: boolean) =>
    call<void>("set_directory_enabled", { path, enabled }),
  getDirectoryConfig: (path: string) => call<DirConfigView>("get_directory_config", { path }),
  saveDirectoryConfig: (path: string, config: DirConfigView) =>
    call<void>("save_directory_config", { path, config }),
  createDirectoryConfig: (path: string, config: DirConfigView) =>
    call<void>("create_directory_config", { path, config }),
  getConfigMtime: (path: string) => call<number>("get_config_mtime", { path }),
  openConfigInEditor: (path: string) => call<void>("open_config_in_editor", { path }),
  scanDirectory: (path: string) => call<FilePreview[]>("scan_directory", { path }),
  listRunHistory: (path: string) => call<RunSummary[]>("list_run_history", { path }),
  compressDirectoryNow: (path: string) => call<void>("compress_directory_now", { path }),
  stopCompression: () => call<void>("stop_compression"),
  recheckFfmpeg: () => call<void>("recheck_ffmpeg"),
  getFfmpegStatus: () => call<FfmpegStatus>("get_ffmpeg_status"),
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
  onScheduledCompressionRequested: (cb: (dirPath: string) => void): Promise<UnlistenFn> =>
    listen<string>("scheduled-compression-requested", (e) => cb(e.payload)),
};
