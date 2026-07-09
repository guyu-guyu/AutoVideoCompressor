<script setup lang="ts">
import type { DirCardInfo } from "../types";
import { formatFileSize } from "../util/format";
import { api } from "../api/tauri";
import { useAppStore } from "../stores/app";

const props = defineProps<{ card: DirCardInfo }>();
const emit = defineEmits<{ open: [path: string]; changed: [] }>();
const store = useAppStore();

const badgeText: Record<string, string> = {
  valid: "● 有效", unscheduled: "● 有效(未排程)",
  invalid: "● 无效(无配置文件)", config_error: "● 配置错误", overlap: "● 重叠",
};

const isRunning = () => store.runtime[props.card.path]?.stage === "compressing";
const runningStatus = () => store.runtime[props.card.path]?.statusText || "压缩中…";
const runningProgress = () => {
  const p = store.progress[props.card.path];
  if (!p) return "";
  return `(${p.completed}/${p.total})`;
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
async function stopIt(e: Event) {
  e.stopPropagation();
  try { await api.stopCompression(); }
  catch (err) { alert(String(err)); }
}
</script>

<template>
  <div class="card" :class="{ running: isRunning() }" @click="emit('open', card.path)">
    <div class="row1">
      <span class="path">📁 {{ card.path }}</span>
      <span v-if="isRunning()" class="badge running-badge">🔄 压缩中 {{ runningProgress() }}</span>
      <span v-else class="badge">{{ badgeText[card.badge] }}
        <template v-if="card.badgeDetail">({{ card.badgeDetail }})</template>
      </span>
      <label @click.stop><input type="checkbox" :checked="card.enabled" @change="toggle" :disabled="isRunning()" /> 启用</label>
      <button v-if="isRunning()" class="stop-btn" @click="stopIt">⏹ 停止</button>
      <button v-else @click="compressNow" :disabled="!store.ffmpeg.ready">▶ 压缩</button>
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
.card.running { border-color:#06c; background:#f0f6ff; }
.row1 { display:flex; gap:10px; align-items:center; }
.path { font-weight:600; flex:1; min-width:0; overflow:hidden; text-overflow:ellipsis; white-space:nowrap; }
.badge { font-size:0.85em; white-space:nowrap; }
.running-badge { color:#06c; font-weight:600; }
.row2 { color:#555; font-size:0.9em; margin-top:4px; }
.warn { color:#c60; font-size:0.9em; margin-top:4px; }
.row3 { display:flex; justify-content:space-between; margin-top:6px;
  border-top:1px dashed #ddd; padding-top:6px; font-size:0.9em; }
.next { color:#06c; font-weight:600; }
.stop-btn { background:#e33; color:#fff; border:none; border-radius:4px; padding:4px 8px; cursor:pointer; }
.stop-btn:hover { background:#c11; }
</style>
