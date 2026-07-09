<script setup lang="ts">
import { onMounted } from "vue";
import { useAppStore } from "../stores/app";
import { api } from "../api/tauri";
import FfmpegStatusBar from "../components/FfmpegStatusBar.vue";
import DirCard from "../components/DirCard.vue";

const store = useAppStore();
const emit = defineEmits<{ open: [path: string]; settings: [] }>();

onMounted(() => store.refreshCards());

async function addDir() {
  const path = prompt("输入目录路径:");
  if (path) { await api.addDirectory(path); await store.refreshCards(); }
}
</script>

<template>
  <div class="page">
    <div class="menubar">
      <span class="title">AutoCompress</span>
      <button @click="emit('settings')">设置(S)</button>
    </div>
    <FfmpegStatusBar />
    <div class="listhead">
      <span>目录列表 ({{ store.cards.length }})</span>
      <button @click="addDir">＋ 添加目录</button>
    </div>
    <DirCard v-for="c in store.cards" :key="c.path" :card="c"
      @open="emit('open', $event)" @changed="store.refreshCards()" />
  </div>
</template>

<style scoped>
.page { padding:12px; }
.menubar { display:flex; justify-content:space-between; align-items:center; margin-bottom:10px; }
.title { font-size:1.3em; font-weight:700; }
.listhead { display:flex; justify-content:space-between; align-items:center; margin-bottom:8px; }
</style>
