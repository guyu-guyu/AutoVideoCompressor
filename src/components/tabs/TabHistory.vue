<script setup lang="ts">
import { ref, watch, onMounted } from "vue";
import { api } from "../../api/tauri";
import { formatFileSize } from "../../util/format";
import type { RunSummary } from "../../types";

const props = defineProps<{ dirPath: string }>();
const runs = ref<RunSummary[]>([]);
const expanded = ref<Record<number, boolean>>({});
async function load() { runs.value = await api.listRunHistory(props.dirPath); }
onMounted(load);
watch(() => props.dirPath, load);
</script>

<template>
  <div>
    <p class="count">共 {{ runs.length }} 次执行</p>
    <div v-for="(r, i) in runs" :key="i" class="run">
      <div class="head" @click="expanded[i] = !expanded[i]">
        {{ expanded[i] ? "▼" : "▶" }} {{ r.startTime }}
        — 成功{{ r.successCount }} · 失败{{ r.failedCount }} · 节省{{ formatFileSize(r.totalSavedBytes) }}
      </div>
      <div v-if="expanded[i]" class="detail">
        <div>跳过(更大): {{ r.skippedLargerCount }} · 跳过(其他): {{ r.skippedOtherCount }} · 循环风险: {{ r.cycleRiskCount }}</div>
        <ul>
          <li v-for="(f, j) in r.files" :key="j">
            {{ f.name }} — {{ f.status }} ({{ formatFileSize(f.originalSize) }} → {{ formatFileSize(f.compressedSize) }})
          </li>
        </ul>
      </div>
    </div>
    <p v-if="runs.length === 0" class="empty">暂无执行记录</p>
  </div>
</template>

<style scoped>
.count { color:#555; }
.run { border:1px solid #eee; border-radius:6px; margin-bottom:6px; }
.head { padding:8px; cursor:pointer; background:#fafafa; }
.detail { padding:8px; font-size:0.9em; }
.empty { color:#888; }
</style>
