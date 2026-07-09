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
