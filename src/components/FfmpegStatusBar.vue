<script setup lang="ts">
import { useAppStore } from "../stores/app";
import { api } from "../api/tauri";
import { NAlert, NButton, NSpace } from "naive-ui";
const store = useAppStore();
async function recheck() { await api.recheckFfmpeg(); }
</script>

<template>
  <n-alert
    class="ffmpeg-bar"
    :type="store.ffmpeg.ready ? 'success' : 'error'"
    :show-icon="true"
  >
    <template #header>
      <n-space align="center" :size="8">
        <span v-if="store.ffmpeg.ready">ffmpeg {{ store.ffmpeg.version }} 已就位</span>
        <span v-else>ffmpeg 不可用（{{ store.ffmpeg.error }}）</span>
        <n-button text size="small" @click="recheck">↻ 重新检测</n-button>
      </n-space>
    </template>
  </n-alert>
</template>

<style scoped>
.ffmpeg-bar { margin-bottom:10px; }
</style>
