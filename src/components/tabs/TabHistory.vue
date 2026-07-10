<script setup lang="ts">
import { ref, watch, onMounted } from "vue";
import { api } from "../../api/tauri";
import { formatFileSize } from "../../util/format";
import type { RunSummary } from "../../types";
import { NCollapse, NCollapseItem, NText, NEmpty, NSpace } from "naive-ui";

const props = defineProps<{ dirPath: string }>();
const runs = ref<RunSummary[]>([]);
async function load() { runs.value = await api.listRunHistory(props.dirPath); }
onMounted(load);
watch(() => props.dirPath, load);
</script>

<template>
  <div>
    <n-text depth="2" class="count">共 {{ runs.length }} 次执行</n-text>
    <n-collapse v-if="runs.length > 0" accordion class="runs">
      <n-collapse-item v-for="(r, i) in runs" :key="i" :name="i">
        <template #header>
          <n-space :size="6" align="center">
            <n-text>{{ r.startTime }}</n-text>
            <n-text depth="3">— 成功{{ r.successCount }} · 失败{{ r.failedCount }} · 节省{{ formatFileSize(r.totalSavedBytes) }}</n-text>
          </n-space>
        </template>
        <n-space vertical :size="4">
          <n-text depth="3">
            跳过(更大): {{ r.skippedLargerCount }} · 跳过(其他): {{ r.skippedOtherCount }} · 循环风险: {{ r.cycleRiskCount }}
          </n-text>
          <ul class="file-list">
            <li v-for="(f, j) in r.files" :key="j">
              {{ f.name }} — {{ f.status }} ({{ formatFileSize(f.originalSize) }} → {{ formatFileSize(f.compressedSize) }})
            </li>
          </ul>
        </n-space>
      </n-collapse-item>
    </n-collapse>
    <n-empty v-else description="暂无执行记录" class="empty" />
  </div>
</template>

<style scoped>
.count { color:#555; }
.runs { margin-top:8px; }
.empty { padding:12px; }
.file-list { margin:0; padding-left:20px; font-size:0.9em; }
</style>
