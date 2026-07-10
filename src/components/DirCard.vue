<script setup lang="ts">
import type { DirCardInfo } from "../types";
import { formatFileSize } from "../util/format";
import { api } from "../api/tauri";
import { useAppStore } from "../stores/app";
import { NCard, NSpace, NTag, NSwitch, NButton, NText } from "naive-ui";

const props = defineProps<{ card: DirCardInfo }>();
const emit = defineEmits<{ open: [path: string]; changed: [] }>();
const store = useAppStore();

const badgeType: Record<string, "success" | "info" | "warning" | "error"> = {
  valid: "success", unscheduled: "info",
  invalid: "error", config_error: "error", overlap: "warning",
};

const isRunning = () => store.runtime[props.card.path]?.stage === "compressing";
const runningStatus = () => store.runtime[props.card.path]?.statusText || "压缩中…";
const runningProgress = () => {
  const p = store.progress[props.card.path];
  if (!p) return "";
  return `(${p.completed}/${p.total})`;
};

// n-switch 的 update:value 回调传入【新值(boolean)】，不是 Event；
// 用该新值调用后端，避免读取 props.card.enabled 在 async 期间被竞态覆盖。
async function toggle(newValue: boolean) {
  console.log("[DirCard] toggle 启用状态:", props.card.path, "->", newValue);
  try {
    await api.setDirectoryEnabled(props.card.path, newValue);
    emit("changed");
  } catch (err) {
    console.error("[DirCard] toggle 失败:", props.card.path, err);
    window.$message?.error(String(err));
  }
}
async function compressNow(e: Event) {
  e.stopPropagation();
  console.log("[DirCard] 立即压缩:", props.card.path);
  try { await api.compressDirectoryNow(props.card.path); }
  catch (err) { console.error("[DirCard] 压缩失败:", props.card.path, err); window.$message?.error(String(err)); }
}
async function stopIt(e: Event) {
  e.stopPropagation();
  console.log("[DirCard] 停止压缩:", props.card.path);
  try { await api.stopCompression(); }
  catch (err) { console.error("[DirCard] 停止失败:", props.card.path, err); window.$message?.error(String(err)); }
}
async function removeIt(e: Event) {
  e.stopPropagation();
  const msg = isRunning()
    ? `目录正在压缩，确定要删除"${props.card.path}"吗？(当前压缩任务将被中断)`
    : `确定要删除目录"${props.card.path}"吗？(仅移除列表，不影响磁盘文件)`;
  const ok = await window.$dialog?.warning({ title: "删除目录", content: msg, positiveText: "删除", negativeText: "取消" });
  if (!ok) { console.log("[DirCard] 删除已取消:", props.card.path); return; }
  console.log("[DirCard] 确认删除目录:", props.card.path, "isRunning=", isRunning());
  try {
    await api.removeDirectory(props.card.path, isRunning());
    emit("changed");
  } catch (err) { console.error("[DirCard] 删除失败:", props.card.path, err); window.$message?.error(String(err)); }
}
</script>

<template>
  <n-card
    class="card"
    :class="{ running: isRunning() }"
    size="small"
    hoverable
    @click="emit('open', card.path)"
  >
    <n-space vertical :size="6">
      <div class="row1">
        <n-ellipsis class="path" :tooltip="false">
          <span>📁 {{ card.path }}</span>
        </n-ellipsis>
        <n-tag v-if="isRunning()" type="info" :bordered="false" round>🔄 压缩中 {{ runningProgress() }}</n-tag>
        <n-tag v-else :type="badgeType[card.badge]" :bordered="false">
          {{ card.badge === "valid" ? "● 有效"
            : card.badge === "unscheduled" ? "● 有效(未排程)"
            : card.badge === "invalid" ? "● 无效(无配置文件)"
            : card.badge === "config_error" ? "● 配置错误"
            : card.badge === "overlap" ? "● 重叠" : card.badge }}
          <template v-if="card.badgeDetail">({{ card.badgeDetail }})</template>
        </n-tag>
        <n-switch
          :value="card.enabled"
          :disabled="isRunning()"
          size="small"
          @click.stop
          @update:value="toggle"
        />
        <n-text depth="3" class="enable-label">启用</n-text>
        <n-button v-if="isRunning()" type="error" size="small" @click="stopIt">⏹ 停止</n-button>
        <n-button v-else type="primary" size="small" :disabled="!store.ffmpeg.ready" @click="compressNow">▶ 压缩</n-button>
        <n-button text size="small" class="del-btn" @click="removeIt" title="从列表中移除">🗑</n-button>
      </div>

      <n-text depth="2" class="row2">
        {{ card.fileCount }} 文件 · {{ formatFileSize(card.totalSize) }}
        <template v-if="card.paramsName"> · 参数 {{ card.paramsName }}</template>
      </n-text>

      <n-text v-if="card.nextRunCount > 0" class="next-run">
        下次压缩: <strong>{{ card.nextRunCount }}</strong> 文件 · {{ formatFileSize(card.nextRunSize) }}
        <template v-if="card.nextRunCount < card.fileCount">({{ card.fileCount - card.nextRunCount }} 文件超限跳过)</template>
      </n-text>

      <n-text v-if="card.cycleRiskCount > 0" class="warn">⚠ {{ card.cycleRiskCount }} 个循环风险</n-text>

      <div class="row3">
        <n-text depth="3">上次: {{ card.lastRunTime || "—" }} {{ card.lastRunResult }}</n-text>
        <n-text depth="3" class="next">下次: {{ card.nextRunTime }}</n-text>
      </div>
    </n-space>
  </n-card>
</template>

<style scoped>
.card { margin-bottom:10px; cursor:pointer; }
.card.running { border-color:#0066cc; }
.row1 { display:flex; gap:10px; align-items:center; }
.path { font-weight:600; flex:1; min-width:0; }
.enable-label { font-size:0.85em; }
.next-run { font-size:0.85em; color:#0066cc; }
.warn { color:#d97706; font-size:0.9em; }
.row3 { display:flex; justify-content:space-between; font-size:0.9em; }
.next { color:#0066cc; font-weight:600; }
.del-btn { opacity:0.4; }
.del-btn:hover { opacity:1; }
</style>
