<script setup lang="ts">
import { ref, watch, onMounted, computed } from "vue";
import { api } from "../../api/tauri";
import { formatFileSize } from "../../util/format";
import type { FilePreview } from "../../types";

const props = defineProps<{ dirPath: string }>();
const files = ref<FilePreview[]>([]);
async function load() { files.value = await api.scanDirectory(props.dirPath); }
onMounted(load);
watch(() => props.dirPath, load);

const nextRunFiles = computed(() => files.value.filter(f => f.inNextRun));
const skippedFiles = computed(() => files.value.filter(f => !f.inNextRun));
const nextRunSize = computed(() => nextRunFiles.value.reduce((s, f) => s + f.fileSize, 0));
</script>

<template>
  <div v-if="nextRunFiles.length > 0 || skippedFiles.length > 0" class="batch-info">
    <span class="next-badge">下次压缩: {{ nextRunFiles.length }} 文件, {{ formatFileSize(nextRunSize) }}</span>
    <span v-if="skippedFiles.length > 0" class="skip-badge">
      ({{ skippedFiles.length }} 文件因体积限制跳过)
    </span>
  </div>
  <table class="tbl">
    <thead><tr><th>原文件</th><th>压缩后</th><th>大小</th><th>风险</th><th>状态</th></tr></thead>
    <tbody>
      <tr v-for="f in nextRunFiles" :key="f.relativePath" class="in-run">
        <td>{{ f.relativePath }}</td>
        <td>{{ f.finalName }}</td>
        <td>{{ formatFileSize(f.fileSize) }}</td>
        <td>{{ f.cycleRisk ? "⚠ 循环" : "✅" }}</td>
        <td class="next">将压缩</td>
      </tr>
      <tr v-for="f in skippedFiles" :key="f.relativePath" class="skipped">
        <td>{{ f.relativePath }}</td>
        <td>{{ f.finalName }}</td>
        <td>{{ formatFileSize(f.fileSize) }}</td>
        <td>{{ f.cycleRisk ? "⚠ 循环" : "✅" }}</td>
        <td class="limit">超限跳过</td>
      </tr>
    </tbody>
  </table>
  <p v-if="files.length === 0" class="empty">无匹配文件</p>
</template>

<style scoped>
.batch-info { background:#e8f4ff; border-radius:6px; padding:6px 12px; margin-bottom:8px; font-size:0.9em; }
.next-badge { color:#06c; font-weight:600; }
.skip-badge { color:#a11; margin-left:8px; }
.tbl { width:100%; border-collapse:collapse; }
.tbl th, .tbl td { border-bottom:1px solid #eee; padding:6px; text-align:left; font-size:0.9em; }
.skipped td { color:#aaa; }
.in-run td { }
.next { color:#06c; font-weight:600; }
.limit { color:#a11; }
.empty { color:#888; padding:12px; }
</style>
