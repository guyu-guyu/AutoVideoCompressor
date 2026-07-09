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
