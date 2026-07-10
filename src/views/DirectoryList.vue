<script setup lang="ts">
import { ref, onMounted } from "vue";
import { open } from "@tauri-apps/plugin-dialog";
import { useAppStore } from "../stores/app";
import { api } from "../api/tauri";
import FfmpegStatusBar from "../components/FfmpegStatusBar.vue";
import DirCard from "../components/DirCard.vue";

const store = useAppStore();
const emit = defineEmits<{ open: [path: string] }>();

const showAdd = ref(false);
const newPath = ref("");

onMounted(() => store.refreshCards());

async function browse() {
    console.log("[DirectoryList] 打开系统目录选择器");
    try {
        const selected = await open({
            directory: true,
            multiple: false,
            title: "选择目录",
        });
        if (typeof selected === "string") newPath.value = selected;
    } catch (e) {
        console.error("[DirectoryList] 打开目录选择器失败:", e);
        window.$message?.error(String(e));
    }
}

async function confirmAdd() {
    const p = newPath.value.trim();
    if (!p) {
        window.$message?.warning("请输入或选择目录路径");
        return;
    }
    console.log("[DirectoryList] 添加目录:", p);
    await api.addDirectory(p);
    await store.refreshCards();
    showAdd.value = false;
    newPath.value = "";
}
</script>

<template>
    <div class="page">
        <FfmpegStatusBar />
        <div class="listhead">
            <span class="count">目录列表 ({{ store.cards.length }})</span>
            <n-button type="primary" size="small" @click="showAdd = true"
                >＋ 添加目录</n-button
            >
        </div>
        <DirCard
            v-for="c in store.cards"
            :key="c.path"
            :card="c"
            @open="emit('open', $event)"
            @changed="store.refreshCards()"
        />
    </div>

    <n-modal
        v-model:show="showAdd"
        preset="card"
        title="添加目录"
        style="width: 480px; max-width: 90vw"
        :auto-focus="false"
    >
        <n-space vertical :size="10">
            <span class="hint">选择或输入要监控的目录路径：</span>
            <n-input-group>
                <n-input
                    v-model:value="newPath"
                    placeholder="目录路径"
                    @keydown.enter="confirmAdd"
                />
                <n-button type="primary" ghost @click="browse">浏览</n-button>
            </n-input-group>
        </n-space>
        <template #footer>
            <n-space justify="end">
                <n-button type="primary" @click="confirmAdd">确定</n-button>
                <n-button @click="showAdd = false">取消</n-button>
            </n-space>
        </template>
    </n-modal>
</template>

<style scoped>
.page {
    padding: 12px;
}
.listhead {
    display: flex;
    justify-content: space-between;
    align-items: center;
    margin-bottom: 8px;
}
.count {
    font-weight: 600;
}
.hint {
    font-size: 13px;
    color: #6b7280;
}
</style>
