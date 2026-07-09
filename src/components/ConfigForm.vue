<script setup lang="ts">
import { reactive, watch } from "vue";
import type { DirConfigView, Template } from "../types";

const props = defineProps<{ modelValue: DirConfigView; templates: Template[] }>();
const emit = defineEmits<{ "update:modelValue": [v: DirConfigView] }>();

const form = reactive<DirConfigView>({ ...props.modelValue });
const custom = reactive({ useCustomParams: !props.templates.some(t => t.name === props.modelValue.params) && props.modelValue.params !== "" });

watch(() => props.modelValue, (v) => { Object.assign(form, v); }, { deep: true });
watch(form, () => emit("update:modelValue", { ...form }), { deep: true });

function addInclude() { form.include.push(""); }
function removeInclude(i: number) { form.include.splice(i, 1); }
function addExclude() { form.exclude.push(""); }
function removeExclude(i: number) { form.exclude.splice(i, 1); }
function addRename() { form.renameRules.push({ pattern: "", replacement: "" }); }
function removeRename(i: number) { form.renameRules.splice(i, 1); }
</script>

<template>
  <div class="form">
    <fieldset><legend>调度</legend>
      每天执行于 <input v-model="form.scheduleTime" placeholder="HH:MM" />
    </fieldset>

    <fieldset><legend>压缩参数</legend>
      <label><input type="checkbox" v-model="custom.useCustomParams" /> 自定义参数</label>
      <select v-if="!custom.useCustomParams" v-model="form.params">
        <option v-for="t in templates" :key="t.name" :value="t.name">{{ t.name }}</option>
      </select>
      <input v-else v-model="form.params" placeholder="裸 ffmpeg 参数" style="width:100%" />
    </fieldset>

    <fieldset><legend>白名单 INCLUDE(必需)</legend>
      <div v-for="(_, i) in form.include" :key="i" class="line">
        <input v-model="form.include[i]" /> <button @click="removeInclude(i)">×</button>
      </div>
      <button @click="addInclude">＋ 添加</button>
    </fieldset>

    <fieldset><legend>黑名单 EXCLUDE</legend>
      <div v-for="(_, i) in form.exclude" :key="i" class="line">
        <input v-model="form.exclude[i]" /> <button @click="removeExclude(i)">×</button>
      </div>
      <button @click="addExclude">＋ 添加</button>
    </fieldset>

    <fieldset><legend>过滤 FILTERS</legend>
      <div class="grid">
        <label>最小 MB <input type="number" v-model.number="form.minSizeMb" /></label>
        <label>最大 MB <input type="number" v-model.number="form.maxSizeMb" /></label>
        <label>修改 ≥ <input v-model="form.mtimeAfter" placeholder="YYYY-MM-DD" /></label>
        <label>修改 ≤ <input v-model="form.mtimeBefore" placeholder="YYYY-MM-DD" /></label>
        <label>创建 ≥ <input v-model="form.ctimeAfter" placeholder="YYYY-MM-DD" /></label>
        <label>创建 ≤ <input v-model="form.ctimeBefore" placeholder="YYYY-MM-DD" /></label>
      </div>
    </fieldset>

    <fieldset><legend>重命名规则</legend>
      <div v-for="(r, i) in form.renameRules" :key="i" class="line">
        <input v-model="r.pattern" placeholder="pattern" />
        →
        <input v-model="r.replacement" placeholder="replacement" />
        <button @click="removeRename(i)">×</button>
      </div>
      <button @click="addRename">＋ 添加</button>
    </fieldset>
  </div>
</template>

<style scoped>
.form fieldset { margin-bottom:10px; }
.line { display:flex; gap:6px; align-items:center; margin-bottom:4px; }
.grid { display:grid; grid-template-columns:1fr 1fr; gap:6px; }
</style>
