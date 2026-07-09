<script setup lang="ts">
import { ref, onMounted, onUnmounted, watch } from "vue";
import { api } from "../../api/tauri";
import ConfigForm from "../ConfigForm.vue";
import type { DirConfigView, Template } from "../../types";

const props = defineProps<{ dirPath: string }>();
const view = ref<DirConfigView | null>(null);
const templates = ref<Template[]>([]);
const error = ref("");
const lastMtime = ref(0);
let timer: number | undefined;

async function load() {
  view.value = await api.getDirectoryConfig(props.dirPath);
  const gc = await api.getGlobalConfig();
  templates.value = gc.templates;
  lastMtime.value = await api.getConfigMtime(props.dirPath);
}
async function poll() {
  const m = await api.getConfigMtime(props.dirPath);
  if (m !== 0 && m !== lastMtime.value) { lastMtime.value = m; await load(); }
}
onMounted(async () => { await load(); timer = window.setInterval(poll, 1500); });
onUnmounted(() => { if (timer) clearInterval(timer); });
watch(() => props.dirPath, load);

async function save() {
  if (!view.value) return;
  error.value = "";
  try {
    if (view.value.exists) await api.saveDirectoryConfig(props.dirPath, view.value);
    else { await api.createDirectoryConfig(props.dirPath, view.value); view.value.exists = true; }
    lastMtime.value = await api.getConfigMtime(props.dirPath);
  } catch (e) { error.value = String(e); }
}
async function reset() { await load(); }
async function openExternal() { await api.openConfigInEditor(props.dirPath); }
</script>

<template>
  <div v-if="view">
    <ConfigForm v-model="view" :templates="templates" />
    <div class="bar">
      <button @click="save">{{ view.exists ? "💾 保存" : "创建配置文件" }}</button>
      <button @click="reset">↻ 重置</button>
      <button @click="openExternal">📂 打开外部编辑器</button>
    </div>
    <p v-if="error" class="err">{{ error }}</p>
  </div>
</template>

<style scoped>
.bar { display:flex; gap:8px; margin-top:10px; }
.err { color:#a11; margin-top:8px; }
</style>
