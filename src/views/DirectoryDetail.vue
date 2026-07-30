<script setup lang="ts">
import { ref, computed } from "vue";
import { useAppStore } from "../stores/app";
import { api } from "../api/tauri";
import { formatFileSize } from "../util/format";
import TabPreview from "../components/tabs/TabPreview.vue";
import TabConfig from "../components/tabs/TabConfig.vue";
import TabHistory from "../components/tabs/TabHistory.vue";

const props = defineProps<{ dirPath: string }>();
const emit = defineEmits<{ back: [] }>();
const store = useAppStore();
const tab = ref<"preview" | "config" | "history">("preview");

const card = computed(() => store.cards.find((c) => c.path === props.dirPath));
const currentStage = computed(() => store.runtime[props.dirPath]?.stage);
const isRunning = computed(
    () => currentStage.value === "scanning" || currentStage.value === "compressing",
);

async function compressNow() {
    console.log("[DirectoryDetail] 立即压缩此目录:", props.dirPath);
    try {
        await api.compressDirectoryNow(props.dirPath);
    } catch (e) {
        console.error("[DirectoryDetail] 压缩失败:", props.dirPath, e);
        window.$message?.error(String(e));
    }
}
async function stopIt() {
    console.log("[DirectoryDetail] 停止压缩:", props.dirPath);
    try {
        await api.stopCompression();
    } catch (e) {
        console.error("[DirectoryDetail] 停止失败:", props.dirPath, e);
        window.$message?.error(String(e));
    }
}
</script>

<template>
    <div class="page">
        <div class="top">
            <div class="backbar">
                <n-button quaternary size="small" @click="emit('back')"
                    >← 返回</n-button
                >
                <n-ellipsis class="path" :tooltip="false">
                    <span>📁 {{ dirPath }}</span>
                </n-ellipsis>
                <n-tag v-if="isRunning" type="info" :bordered="false" round
                    >{{ currentStage === "scanning" ? "🔍 扫描中" : "🔄 压缩中" }}</n-tag
                >
                <n-button
                    v-if="isRunning"
                    type="error"
                    size="small"
                    @click="stopIt"
                    >⏹ 停止压缩</n-button
                >
                <n-button
                    v-else
                    type="primary"
                    size="small"
                    :disabled="!store.ffmpeg.ready"
                    @click="compressNow"
                    >立即压缩此目录</n-button
                >
            </div>

            <n-card
                v-if="isRunning && store.progress[dirPath]"
                size="small"
                class="progress-card"
            >
                <n-space vertical :size="4">
                    <span
                        >正在处理:
                        {{ store.progress[dirPath].currentFile }}</span
                    >
                    <n-progress
                        type="line"
                        :percentage="
                            store.progress[dirPath].total > 0
                                ? Math.round(
                                      (store.progress[dirPath].completed /
                                          store.progress[dirPath].total) *
                                          100,
                                  )
                                : 0
                        "
                        :height="12"
                        :show-indicator="false"
                    />
                    <span class="progress-text"
                        >({{ store.progress[dirPath].completed }}/{{
                            store.progress[dirPath].total
                        }})</span
                    >
                </n-space>
            </n-card>

            <n-card v-if="card" size="small" class="summary">
                <n-space vertical :size="4">
                    <div>
                        匹配 {{ card.fileCount }} 文件 ·
                        {{ formatFileSize(card.totalSize) }} · 参数
                        {{ card.paramsName || "默认" }} · 下次
                        {{ card.nextRunTime }}
                        <span v-if="card.cycleRiskCount" class="risk"
                            >· ⚠ {{ card.cycleRiskCount }} 循环风险</span
                        >
                    </div>
                    <div v-if="card.nextRunCount > 0" class="next-run">
                        下次压缩: <strong>{{ card.nextRunCount }}</strong> 文件
                        ·
                        {{ formatFileSize(card.nextRunSize) }}
                        <template v-if="card.nextRunCount < card.fileCount"
                            >({{
                                card.fileCount - card.nextRunCount
                            }}
                            文件超限跳过)</template
                        >
                    </div>
                    <div>
                        上次: {{ card.lastRunTime || "—" }}
                        {{ card.lastRunResult }}
                    </div>
                </n-space>
            </n-card>
        </div>

        <n-tabs v-model:value="tab" type="line" class="tabs">
            <n-tab-pane name="preview" tab="压缩文件预览">
                <TabPreview :dir-path="dirPath" />
            </n-tab-pane>
            <n-tab-pane name="config" tab="配置">
                <TabConfig :dir-path="dirPath" />
            </n-tab-pane>
            <n-tab-pane name="history" tab="压缩执行历史">
                <TabHistory :dir-path="dirPath" />
            </n-tab-pane>
        </n-tabs>
    </div>
</template>

<style scoped>
.page {
    flex: 1 1 auto;
    min-height: 0;
    display: flex;
    flex-direction: column;
    padding: 12px;
    box-sizing: border-box;
}
.top {
    flex: 0 0 auto;
}
.backbar {
    display: flex;
    gap: 10px;
    align-items: center;
    margin-bottom: 10px;
}
.path {
    flex: 1;
    font-weight: 600;
    min-width: 0;
}
.progress-card {
    margin-bottom: 10px;
}
.summary {
    margin-bottom: 10px;
    font-size: 0.9em;
}
/* tabs 占满剩余高度 */
.tabs {
    flex: 1 1 auto;
    min-height: 0;
    margin-top: 4px;
}
/* n-tabs 根节点变成 flex column */
.tabs :deep(.n-tabs) {
    display: flex !important;
    flex-direction: column !important;
    height: 100% !important;
}
/* nav 栏固定，不伸缩 */
.tabs :deep(.n-tabs-nav) {
    flex: 0 0 auto !important;
}
/* 非 animated 模式下 pane 直接就是 .n-tab-pane，让它滚动 */
.tabs :deep(.n-tab-pane) {
    flex: 1 1 auto !important;
    min-height: 0 !important;
    overflow: auto !important;
}
/* animated 模式下多一层 pane-wrapper，同样处理 */
.tabs :deep(.n-tabs-pane-wrapper) {
    flex: 1 1 auto !important;
    min-height: 0 !important;
    overflow: auto !important;
}
.risk {
    color: #d97706;
}
.next-run {
    color: #0066cc;
    font-weight: 600;
}
.progress-text {
    font-size: 0.85em;
    color: #0066cc;
}
</style>
