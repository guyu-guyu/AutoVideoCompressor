<script setup lang="ts">
import { ref, watch, onMounted } from "vue";
import { api } from "../../api/tauri";
import { formatFileSize } from "../../util/format";
import type { FilePreview } from "../../types";

const props = defineProps<{ dirPath: string }>();
const files = ref<FilePreview[]>([]);
async function load() { files.value = await api.scanDirectory(props.dirPath); }
onMounted(load);
watch(() => props.dirPath, load);
</script>

<template>
  <table class="tbl">
    <thead><tr><th>原文件</th><th>压缩后</th><th>大小</th><th>风险</th></tr></thead>
    <tbody>
      <tr v-for="f in files" :key="f.relativePath">
        <td>{{ f.relativePath }}</td>
        <td>{{ f.finalName }}</td>
        <td>{{ formatFileSize(f.fileSize) }}</td>
        <td>{{ f.cycleRisk ? "⚠ 循环" : "✅" }}</td>
      </tr>
    </tbody>
  </table>
  <p v-if="files.length === 0" class="empty">无匹配文件</p>
</template>

<style scoped>
.tbl { width:100%; border-collapse:collapse; }
.tbl th, .tbl td { border-bottom:1px solid #eee; padding:6px; text-align:left; font-size:0.9em; }
.empty { color:#888; padding:12px; }
</style>
