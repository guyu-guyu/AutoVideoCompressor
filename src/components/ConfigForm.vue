<script setup lang="ts">
import { ref, watch, computed } from "vue";
import type { DirConfigView, Template } from "../types";

const props = defineProps<{ modelValue: DirConfigView; templates: Template[] }>();
const emit = defineEmits<{ "update:modelValue": [v: DirConfigView] }>();

const form = ref<DirConfigView>({ ...props.modelValue });

watch(() => props.modelValue, (v) => { form.value = { ...v }; }, { deep: true });
watch(form, () => emit("update:modelValue", { ...form.value }), { deep: true });

// isCustom reads directly from the data model (persisted in config file)
const isCustom = computed(() => form.value.useCustomParams);

function onCustomToggle(e: Event) {
  const checked = (e.target as HTMLInputElement).checked;
  if (checked) {
    // template mode → custom mode: resolve template to actual params
    const tmpl = props.templates.find(t => t.name === form.value.params);
    if (tmpl) {
      form.value.params = tmpl.params;
    }
    form.value.useCustomParams = true;
  } else {
    // custom mode → template mode: set params to first template name
    if (props.templates.length > 0) {
      form.value.params = props.templates[0].name;
    }
    form.value.useCustomParams = false;
  }
}

function onParamInput(e: Event) {
  if (isCustom.value) {
    form.value.params = (e.target as HTMLInputElement).value;
  }
}

function onTemplateSelect(e: Event) {
  const target = e.target as HTMLSelectElement;
  form.value.params = target.value;
}

// Current selected template's resolved params (for readonly display)
const selectedTemplateParams = computed(() => {
  const t = props.templates.find(t => t.name === form.value.params);
  return t ? t.params : "";
});

function addInclude() { form.value.include.push(""); }
function removeInclude(i: number) { form.value.include.splice(i, 1); }
function addExclude() { form.value.exclude.push(""); }
function removeExclude(i: number) { form.value.exclude.splice(i, 1); }
function addRename() { form.value.renameRules.push({ pattern: "", replacement: "" }); }
function removeRename(i: number) { form.value.renameRules.splice(i, 1); }
</script>

<template>
  <div class="form">
    <fieldset><legend>调度</legend>
      每天执行于 <input v-model="form.scheduleTime" placeholder="HH:MM" />
    </fieldset>

    <fieldset><legend>压缩参数</legend>
      <label class="chk-label">
        <input type="checkbox" :checked="isCustom" @change="onCustomToggle" />
        自定义参数
      </label>
      <div class="param-row">
        <select :disabled="isCustom" :value="form.params" @change="onTemplateSelect" class="tmpl-select">
          <option v-for="t in templates" :key="t.name" :value="t.name">{{ t.name }}</option>
        </select>
        <input
          :value="isCustom ? form.params : selectedTemplateParams"
          @input="onParamInput"
          :readonly="!isCustom"
          :placeholder="isCustom ? '裸 ffmpeg 参数，如 -c:v libx265 -crf 18' : ''"
          class="param-input"
        />
      </div>
      <div v-if="!isCustom && selectedTemplateParams" class="tmpl-hint">
        当前选择: <code>{{ selectedTemplateParams }}</code>
      </div>
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
        <label>单次压缩最大 MB <input type="number" v-model.number="form.maxCompressSizeMb" placeholder="空=不限制" /></label>
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
.chk-label { display:block; margin-bottom:6px; cursor:pointer; user-select:none; }
.param-row { display:flex; gap:8px; align-items:stretch; }
.tmpl-select { min-width:160px; }
.param-input { flex:1; }
.param-input[readonly] { background:#f5f5f5; color:#666; cursor:default; }
.tmpl-hint { margin-top:4px; font-size:0.85em; color:#888; }
.tmpl-hint code { background:#f0f0f0; padding:1px 6px; border-radius:3px; font-size:0.95em; }
</style>
