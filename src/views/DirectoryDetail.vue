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
const isRunning = computed(() => store.runtime[props.dirPath]?.stage === "compressing");

async function compressNow() {
  try { await api.compressDirectoryNow(props.dirPath); }
  catch (e) { alert(String(e)); }
}
async function stopIt() {
  try { await api.stopCompression(); }
  catch (e) { alert(String(e)); }
}
</script>

<template>
  <div class="page">
    <div class="backbar">
      <button @click="emit('back')">← 返回</button>
      <span class="path">📁 {{ dirPath }}</span>
      <span v-if="isRunning" class="running-indicator">🔄 压缩中</span>
      <button v-if="isRunning" class="stop-btn" @click="stopIt">⏹ 停止压缩</button>
      <button v-else @click="compressNow" :disabled="!store.ffmpeg.ready">立即压缩此目录</button>
    </div>
    <div v-if="isRunning && store.progress[dirPath]" class="progress-bar">
      正在处理: {{ store.progress[dirPath].currentFile }}
      ({{ store.progress[dirPath].completed }}/{{ store.progress[dirPath].total }})
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
.running-indicator { color:#06c; font-weight:600; }
.summary { background:#f6f6f6; border-radius:6px; padding:8px; margin-bottom:10px; font-size:0.9em; }
.tabs { display:flex; gap:4px; margin-bottom:10px; }
.tabs button.active { background:#06c; color:#fff; }
.stop-btn { background:#e33; color:#fff; border:none; border-radius:4px; padding:4px 12px; cursor:pointer; }
.stop-btn:hover { background:#c11; }
.progress-bar { background:#e8f4ff; border-radius:6px; padding:6px 12px; margin-bottom:10px; font-size:0.9em; color:#06c; }
</style>
