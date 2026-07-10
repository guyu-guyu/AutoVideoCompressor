<script setup lang="ts">
import { onMounted } from "vue";
import { useAppStore } from "../stores/app";
import { api } from "../api/tauri";
import FfmpegStatusBar from "../components/FfmpegStatusBar.vue";
import DirCard from "../components/DirCard.vue";

const store = useAppStore();
const emit = defineEmits<{ open: [path: string] }>();

onMounted(() => store.refreshCards());

async function addDir() {
    const path = window.prompt("输入目录路径:");
    console.log("[DirectoryList] 添加目录, 输入:", path);
    if (path) {
        await api.addDirectory(path);
        await store.refreshCards();
    }
}
</script>

<template>
    <div class="page">
        <FfmpegStatusBar />
        <div class="listhead">
            <span class="count">目录列表 ({{ store.cards.length }})</span>
            <n-button type="primary" size="small" @click="addDir"
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
</style>
