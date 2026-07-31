<script setup lang="ts">
import { ref, onMounted } from "vue";
import { api } from "../api/tauri";
import { useAppStore } from "../stores/app";
import type { GlobalConfig } from "../types";
import {
    NModal,
    NCard,
    NForm,
    NFormItem,
    NInput,
    NInputNumber,
    NCheckbox,
    NButton,
    NSpace,
} from "naive-ui";

const emit = defineEmits<{ close: [] }>();
const store = useAppStore();
const cfg = ref<GlobalConfig | null>(null);

onMounted(async () => {
    cfg.value = await api.getGlobalConfig();
});

function addTemplate() {
    cfg.value!.templates.push({ name: "", params: "" });
}
function removeTemplate(i: number) {
    cfg.value!.templates.splice(i, 1);
}

async function save() {
    if (!cfg.value) return;
    cfg.value.templates = cfg.value.templates.filter(
        (t) => t.name.trim() !== "",
    );
    await api.saveGlobalConfig(cfg.value);
    await store.refreshCards();
    await store.refreshFfmpeg();
    emit("close");
}
</script>

<template>
    <n-modal
        :show="true"
        preset="card"
        title="全局设置"
        style="width: 780px; max-width: 90vw"
        :auto-focus="false"
        @close="emit('close')"
        @update:show="
            (v: boolean) => {
                if (!v) emit('close');
            }
        "
    >
        <n-form v-if="cfg" label-placement="top">
            <n-form-item label="ffmpeg 路径">
                <n-input
                    v-model:value="cfg.ffmpegPath"
                    placeholder="ffmpeg 路径"
                />
            </n-form-item>
            <n-form-item label="ffmpeg 超时(秒)">
                <n-input-number
                    v-model:value="cfg.ffmpegTimeoutSeconds"
                    :min="1"
                    style="width: 100%"
                />
            </n-form-item>
            <n-form-item label="日志保留天数">
                <n-input-number
                    v-model:value="cfg.logRetentionDays"
                    :min="0"
                    style="width: 100%"
                />
            </n-form-item>
            <n-space vertical :size="4">
                <n-checkbox v-model:checked="cfg.startWithWindows"
                    >开机自启动</n-checkbox
                >
                <n-checkbox v-model:checked="cfg.minimizeToTray"
                    >最小化到托盘</n-checkbox
                >
                <n-checkbox v-model:checked="cfg.useWindowsTaskScheduler"
                    >使用 Windows 计划任务（退出后仍按时启动）</n-checkbox
                >
                <n-checkbox
                    v-if="cfg.useWindowsTaskScheduler"
                    v-model:checked="cfg.wakeComputerForScheduledTasks"
                    class="scheduler-sub-option"
                    >唤醒计算机执行任务</n-checkbox
                >
            </n-space>

            <n-form-item label="模板" class="tmpl-form-item">
                <n-space vertical :size="6" style="width: 100%">
                    <div v-for="(t, i) in cfg.templates" :key="i" class="tmpl">
                        <n-input
                            v-model:value="t.name"
                            placeholder="名称"
                            style="width: 120px"
                        />
                        <n-input
                            v-model:value="t.params"
                            placeholder="参数"
                            style="flex: 1"
                        />
                        <n-button text type="error" @click="removeTemplate(i)"
                            >×</n-button
                        >
                    </div>
                    <n-button dashed size="small" @click="addTemplate"
                        >＋ 添加模板</n-button
                    >
                </n-space>
            </n-form-item>
        </n-form>

        <template #footer>
            <n-space justify="end">
                <n-button type="primary" @click="save">保存</n-button>
                <n-button @click="emit('close')">取消</n-button>
            </n-space>
        </template>
    </n-modal>
</template>

<style scoped>
.tmpl {
    display: flex;
    gap: 6px;
    align-items: center;
}
.tmpl-form-item :deep(.n-form-item-blank) {
    width: 100%;
}
.scheduler-sub-option {
    margin-left: 24px;
}
</style>
