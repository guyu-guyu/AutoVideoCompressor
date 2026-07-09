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
  stopCompression: () => invoke<void>("stop_compression"),
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
