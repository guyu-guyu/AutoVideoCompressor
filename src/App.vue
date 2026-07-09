<script setup lang="ts">
import { onMounted, ref } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useAppStore } from "./stores/app";
import DirectoryList from "./views/DirectoryList.vue";
import DirectoryDetail from "./views/DirectoryDetail.vue";
import GlobalSettings from "./components/GlobalSettings.vue";

const store = useAppStore();
const page = ref<"list" | "detail">("list");
const selectedDir = ref("");
const showSettings = ref(false);

onMounted(() => store.init());

function openDir(path: string) { selectedDir.value = path; page.value = "detail"; }
function back() { page.value = "list"; store.refreshCards(); }

async function forceQuit() { await getCurrentWindow().destroy(); }
</script>

<template>
  <DirectoryList v-if="page === 'list'" @open="openDir" @settings="showSettings = true" />
  <DirectoryDetail v-else :dir-path="selectedDir" @back="back" />
  <GlobalSettings v-if="showSettings" @close="showSettings = false" />

  <div v-if="store.compressingWhileClose" class="overlay">
    <div class="confirm">
      <p>正在压缩,确定退出吗?(当前压缩任务将被中断)</p>
      <button @click="forceQuit">强制退出</button>
      <button @click="store.compressingWhileClose = false">继续等待</button>
    </div>
  </div>
</template>

<style>
.overlay { position:fixed; inset:0; background:rgba(0,0,0,.4); display:flex; justify-content:center; align-items:center; }
.confirm { background:#fff; padding:20px; border-radius:8px; }
</style>
