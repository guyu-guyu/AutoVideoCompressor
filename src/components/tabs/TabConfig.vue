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
const creating = ref(false);
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

async function createDefault() {
  if (!view.value) return;
  creating.value = true;
  error.value = "";
  try {
    // Ensure params is set to first template name before creating
    if (!view.value.params && templates.value.length > 0) {
      view.value.params = templates.value[0].name;
    }
    await api.createDirectoryConfig(props.dirPath, view.value);
    await load(); // reload → now exists = true, will show the form
  } catch (e) { error.value = String(e); }
  finally { creating.value = false; }
}

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
    <!-- No config exists: show only create button -->
    <div v-if="!view.exists" class="no-config">
      <p>此目录尚未创建配置文件</p>
      <button class="create-btn" @click="createDefault" :disabled="creating">
        {{ creating ? "创建中…" : "＋ 创建默认配置文件" }}
      </button>
      <p v-if="error" class="err">{{ error }}</p>
    </div>

    <!-- Config exists: show full form -->
    <template v-else>
      <ConfigForm v-model="view" :templates="templates" />
      <div class="bar">
        <button @click="save">{{ view.exists ? "💾 保存" : "创建配置文件" }}</button>
        <button @click="reset">↻ 重置</button>
        <button @click="openExternal">📂 打开外部编辑器</button>
      </div>
      <p v-if="error" class="err">{{ error }}</p>
    </template>
  </div>
</template>

<style scoped>
.bar { display:flex; gap:8px; margin-top:10px; }
.err { color:#a11; margin-top:8px; }
.no-config { text-align:center; padding:40px 20px; }
.no-config p { color:#888; margin-bottom:16px; }
.create-btn { padding:10px 24px; font-size:1.1em; background:#06c; color:#fff;
  border:none; border-radius:6px; cursor:pointer; }
.create-btn:hover { background:#058; }
.create-btn:disabled { opacity:0.6; cursor:not-allowed; }
</style>
