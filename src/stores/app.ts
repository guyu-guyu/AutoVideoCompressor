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
  const scheduledRequest = ref({ id: 0, dirPath: "" });

  async function refreshCards() {
    try {
      const list = await api.listDirectories();
      cards.value = list;
      console.log("[store] refreshCards 完成: 共", list.length, "个目录");
    } catch (e) {
      console.error("[store] refreshCards 失败:", e);
      throw e;
    }
  }
  async function refreshFfmpeg() {
    try {
      const s = await api.getFfmpegStatus();
      ffmpeg.value = s;
      console.log("[store] refreshFfmpeg 完成: ready=", s.ready, "version=", s.version);
    } catch (e) {
      console.error("[store] refreshFfmpeg 失败:", e);
      throw e;
    }
  }

  async function init() {
    console.log("[store] init 开始");
    await refreshCards();
    await refreshFfmpeg();
    // 事件监听注册各自独立 try/catch：单个失败不应阻断其余监听
    try {
      await events.onFfmpegStatus((s) => { ffmpeg.value = s; });
      console.log("[store] 已注册 ffmpeg-status-changed 监听");
    } catch (e) { console.error("[store] 注册 onFfmpegStatus 失败:", e); }
    try {
      await events.onDirState((s) => {
        runtime.value[s.dirPath] = s;
        refreshCards(); // last/next-run and stage changed
      });
      console.log("[store] 已注册 dir-state-changed 监听");
    } catch (e) { console.error("[store] 注册 onDirState 失败:", e); }
    try {
      await events.onProgress((p) => { progress.value[p.dirPath] = p; });
      console.log("[store] 已注册 compress-progress 监听");
    } catch (e) { console.error("[store] 注册 onProgress 失败:", e); }
    try {
      await events.onCloseWhileCompressing(() => { compressingWhileClose.value = true; });
      console.log("[store] 已注册 close-requested-while-compressing 监听");
    } catch (e) { console.error("[store] 注册 onCloseWhileCompressing 失败:", e); }
    try {
      await events.onScheduledCompressionRequested((dirPath) => {
        scheduledRequest.value = { id: scheduledRequest.value.id + 1, dirPath };
      });
      console.log("[store] 已注册 scheduled-compression-requested 监听");
    } catch (e) { console.error("[store] 注册计划任务导航监听失败:", e); }
    await api.frontendReady();
    console.log("[store] init 完成");
  }

  return { cards, ffmpeg, runtime, progress, compressingWhileClose, scheduledRequest,
           refreshCards, refreshFfmpeg, init };
});
