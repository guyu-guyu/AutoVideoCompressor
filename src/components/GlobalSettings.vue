<script setup lang="ts">
import { ref, onMounted } from "vue";
import { api } from "../api/tauri";
import { useAppStore } from "../stores/app";
import type { GlobalConfig } from "../types";

const emit = defineEmits<{ close: [] }>();
const store = useAppStore();
const cfg = ref<GlobalConfig | null>(null);

onMounted(async () => { cfg.value = await api.getGlobalConfig(); });

function addTemplate() { cfg.value!.templates.push({ name: "", params: "" }); }
function removeTemplate(i: number) { cfg.value!.templates.splice(i, 1); }

async function save() {
  if (!cfg.value) return;
  cfg.value.templates = cfg.value.templates.filter(t => t.name.trim() !== "");
  await api.saveGlobalConfig(cfg.value);
  await store.refreshCards();
  await store.refreshFfmpeg();
  emit("close");
}
</script>

<template>
  <div class="overlay" @click.self="emit('close')">
    <div class="dialog" v-if="cfg">
      <h3>全局设置</h3>
      <label>ffmpeg 路径 <input v-model="cfg.ffmpegPath" style="width:100%" /></label>
      <label>ffmpeg 超时(秒) <input type="number" v-model.number="cfg.ffmpegTimeoutSeconds" /></label>
      <label>日志保留天数 <input type="number" v-model.number="cfg.logRetentionDays" /></label>
      <label><input type="checkbox" v-model="cfg.startWithWindows" /> 开机自启动</label>
      <label><input type="checkbox" v-model="cfg.minimizeToTray" /> 最小化到托盘</label>

      <h4>模板</h4>
      <div v-for="(t, i) in cfg.templates" :key="i" class="tmpl">
        <input v-model="t.name" placeholder="名称" />
        <input v-model="t.params" placeholder="参数" style="flex:1" />
        <button @click="removeTemplate(i)">×</button>
      </div>
      <button @click="addTemplate">＋ 添加模板</button>

      <div class="bar">
        <button @click="save">保存</button>
        <button @click="emit('close')">取消</button>
      </div>
    </div>
  </div>
</template>

<style scoped>
.overlay { position:fixed; inset:0; background:rgba(0,0,0,.4); display:flex;
  justify-content:center; align-items:center; }
.dialog { background:#fff; border-radius:8px; padding:16px; width:520px; max-height:80vh; overflow:auto; }
.dialog label { display:block; margin:6px 0; }
.tmpl { display:flex; gap:6px; margin-bottom:4px; }
.bar { display:flex; gap:8px; margin-top:12px; }
</style>
