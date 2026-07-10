<script setup lang="ts">
// AppShell 是 provider 的后代组件，这里才能正确 inject 出 dialog/message。
import { onMounted, ref } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { NModal, useDialog, useMessage } from "naive-ui";
import { useAppStore } from "./stores/app";
import DirectoryList from "./views/DirectoryList.vue";
import DirectoryDetail from "./views/DirectoryDetail.vue";
import GlobalSettings from "./components/GlobalSettings.vue";

const store = useAppStore();
const page = ref<"list" | "detail">("list");
const selectedDir = ref("");
const showSettings = ref(false);

// 此处位于 n-message-provider / n-dialog-provider 内部，inject 可正常取到实例。
// 暴露到 window，供未注入 composables 的组件替换原生 alert/confirm/prompt。
const dialog = useDialog();
const message = useMessage();
if (!dialog) console.warn("[App] useDialog() 返回空，window.$dialog 将不可用");
if (!message) console.warn("[App] useMessage() 返回空，window.$message 将不可用");
window.$dialog = dialog;
window.$message = message;
console.log("[App] AppShell 已挂载，providers 已就绪: $dialog=", !!dialog, " $message=", !!message);

onMounted(() => {
  // store.init 注册 Tauri 事件监听 + 首次拉取数据；任何失败都记录，避免静默
  store.init().catch((e) => console.error("[App] store.init 失败:", e));
});

function openDir(path: string) { selectedDir.value = path; page.value = "detail"; }
function back() { page.value = "list"; store.refreshCards(); }

async function forceQuit() { await getCurrentWindow().destroy(); }
</script>

<template>
  <DirectoryList v-if="page === 'list'" @open="openDir" @settings="showSettings = true" />
  <DirectoryDetail v-else :dir-path="selectedDir" @back="back" />
  <GlobalSettings v-if="showSettings" @close="showSettings = false" />

  <n-modal
    v-model:show="store.compressingWhileClose"
    preset="dialog"
    title="正在压缩"
    :closable="false"
    positive-text="强制退出"
    negative-text="继续等待"
    @positive-click="forceQuit"
  >
    <template #default>
      正在压缩，确定退出吗？（当前压缩任务将被中断）
    </template>
  </n-modal>
</template>
