<script setup lang="ts">
import { useAppStore } from "../stores/app";
import { api } from "../api/tauri";
const store = useAppStore();
async function recheck() { await api.recheckFfmpeg(); }
</script>

<template>
  <div class="ffmpeg-bar" :class="store.ffmpeg.ready ? 'ok' : 'err'">
    <span v-if="store.ffmpeg.ready">✅ ffmpeg {{ store.ffmpeg.version }} 已就位</span>
    <span v-else>❌ ffmpeg 不可用（{{ store.ffmpeg.error }}）</span>
    <button @click="recheck">↻ 重新检测</button>
  </div>
</template>

<style scoped>
.ffmpeg-bar { display:flex; justify-content:space-between; align-items:center;
  padding:6px 12px; border-radius:6px; margin-bottom:10px; }
.ok { background:#e6f7e6; color:#207520; }
.err { background:#fbe6e6; color:#a11; }
</style>
