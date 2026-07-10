<script setup lang="ts">
import { ref, watch, onMounted, computed, h } from "vue";
import { api } from "../../api/tauri";
import { formatFileSize } from "../../util/format";
import type { FilePreview } from "../../types";
import { NDataTable, NTag, NAlert, NSpace, NText, NEmpty } from "naive-ui";

const props = defineProps<{ dirPath: string }>();
const files = ref<FilePreview[]>([]);
async function load() { files.value = await api.scanDirectory(props.dirPath); }
onMounted(load);
watch(() => props.dirPath, load);

const nextRunFiles = computed(() => files.value.filter(f => f.inNextRun));
const skippedFiles = computed(() => files.value.filter(f => !f.inNextRun));
const nextRunSize = computed(() => nextRunFiles.value.reduce((s, f) => s + f.fileSize, 0));

// 合并为统一表格，并按是否属于下次压缩排序
const rows = computed(() =>
  [...nextRunFiles.value, ...skippedFiles.value].map(f => ({ ...f })),
);

const columns = [
  { title: "原文件", key: "relativePath" },
  { title: "压缩后", key: "finalName" },
  {
    title: "大小",
    key: "fileSize",
    render: (row: FilePreview) => formatFileSize(row.fileSize),
  },
  {
    title: "风险",
    key: "cycleRisk",
    render: (row: FilePreview) =>
      h(NTag, { type: row.cycleRisk ? "warning" : "success", size: "small", bordered: false },
        { default: () => row.cycleRisk ? "⚠ 循环" : "✅" }),
  },
  {
    title: "状态",
    key: "inNextRun",
    render: (row: FilePreview) =>
      row.inNextRun
        ? h(NTag, { type: "info", size: "small", bordered: false }, { default: () => "将压缩" })
        : h(NTag, { type: "default", size: "small", bordered: false }, { default: () => "超限跳过" }),
  },
];
</script>

<template>
  <div>
    <n-alert v-if="nextRunFiles.length > 0 || skippedFiles.length > 0" type="info" class="batch-info">
      <template #header>
        <n-space :size="8" align="center">
          <n-text strong class="next-badge">下次压缩: {{ nextRunFiles.length }} 文件, {{ formatFileSize(nextRunSize) }}</n-text>
          <n-text v-if="skippedFiles.length > 0" class="skip-badge">({{ skippedFiles.length }} 文件因体积限制跳过)</n-text>
        </n-space>
      </template>
    </n-alert>

    <n-data-table
      v-if="rows.length > 0"
      :columns="columns"
      :data="rows"
      :row-key="(row: FilePreview) => row.relativePath"
      :row-props="(row: FilePreview) => ({ class: row.inNextRun ? 'in-run' : 'skipped' })"
      size="small"
    />

    <n-empty v-else description="无匹配文件" class="empty" />
  </div>
</template>

<style scoped>
.batch-info { margin-bottom:8px; }
.next-badge { color:#0066cc; }
.skip-badge { color:#a11; }
.empty { padding:12px; }
:deep(.skipped td) { color:#aaa; }
</style>
