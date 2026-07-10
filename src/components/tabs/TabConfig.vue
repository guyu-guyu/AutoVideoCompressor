<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted, computed } from "vue";
import { api } from "../../api/tauri";
import ConfigForm from "../ConfigForm.vue";
import type { DirConfigView, Template } from "../../types";
import { NButton, NSpace, NAlert, NCard } from "naive-ui";

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
  console.log("[TabConfig] load 完成:", props.dirPath, "exists=", view.value?.exists);
}
async function poll() {
  const m = await api.getConfigMtime(props.dirPath);
  // 仅当磁盘配置文件 mtime 真实变化时才重载（用户编辑期间不会触发，因未保存）
  if (m !== 0 && m !== lastMtime.value) {
    console.log("[TabConfig] 检测到配置文件 mtime 变化，重新加载:", lastMtime.value, "->", m);
    lastMtime.value = m;
    await load();
  }
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

const hasError = computed(() => error.value.length > 0);
</script>

<template>
  <div v-if="view">
    <!-- No config exists: show only create button -->
    <div v-if="!view.exists" class="no-config">
      <p>此目录尚未创建配置文件</p>
      <n-button type="primary" :loading="creating" @click="createDefault">
        {{ creating ? "创建中…" : "＋ 创建默认配置文件" }}
      </n-button>
      <n-alert v-if="hasError" type="error" class="err">{{ error }}</n-alert>
    </div>

    <!-- Config exists: show full form -->
    <template v-else>
      <ConfigForm v-model="view" :templates="templates" />
      <n-space class="bar">
        <n-button type="primary" @click="save">{{ view.exists ? "💾 保存" : "创建配置文件" }}</n-button>
        <n-button @click="reset">↻ 重置</n-button>
        <n-button @click="openExternal">📂 打开外部编辑器</n-button>
      </n-space>
      <n-alert v-if="hasError" type="error" class="err">{{ error }}</n-alert>
    </template>
  </div>
</template>

<style scoped>
.bar { margin-top:10px; }
.err { margin-top:8px; }
.no-config { text-align:center; padding:40px 20px; }
.no-config p { color:#888; margin-bottom:16px; }
</style>
