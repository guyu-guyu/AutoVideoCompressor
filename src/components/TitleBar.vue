<script setup lang="ts">
import { onMounted, onUnmounted, ref } from "vue";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { UnlistenFn } from "@tauri-apps/api/event";

// 仅当运行在 Tauri 环境（而非浏览器 dev）时，窗口控制按钮才生效。
const isTauri = "__TAURI_INTERNALS__" in window || "__TAURI__" in window;
const maximized = ref(false);
const emit = defineEmits<{ settings: [] }>();
let unlisten: UnlistenFn | undefined;

async function syncMaximized() {
    if (!isTauri) return;
    try {
        maximized.value = await getCurrentWindow().isMaximized();
    } catch {}
}

async function minimize() {
    try {
        await getCurrentWindow().minimize();
    } catch {}
}
async function toggleMaximize() {
    try {
        await getCurrentWindow().toggleMaximize();
    } catch {}
}
async function close() {
    try {
        await getCurrentWindow().close();
    } catch {}
}

// 通过 JS 触发拖拽：在标题栏任意位置（操作按钮区域除外）按下左键即开始拖动窗口。
// 用 startDragging() 兜底，保证 logo / 文本等子元素上也能正常拖动。
async function startDrag(e: MouseEvent) {
    if (!isTauri || e.button !== 0) return;
    const target = e.target as HTMLElement | null;
    if (target?.closest?.(".actions")) return;
    try {
        await getCurrentWindow().startDragging();
    } catch {}
}

onMounted(async () => {
    if (!isTauri) return;
    await syncMaximized();
    try {
        unlisten = await getCurrentWindow().onResized(syncMaximized);
    } catch {}
});
onUnmounted(() => {
    unlisten?.();
});
</script>

<template>
    <div class="titlebar" @mousedown="startDrag">
        <div class="brand">
            <span class="logo">🗜</span>
            <span class="name">AutoCompress</span>
        </div>
        <div class="actions">
            <button class="action" title="设置" @click="emit('settings')">
                设置
            </button>
            <div v-if="isTauri" class="controls">
                <button class="ctrl" title="最小化" @click="minimize">
                    <svg width="12" height="12" viewBox="0 0 12 12">
                        <rect y="5" width="12" height="2" fill="currentColor" />
                    </svg>
                </button>
                <button
                    class="ctrl"
                    :title="maximized ? '还原' : '最大化'"
                    @click="toggleMaximize"
                >
                    <svg
                        v-if="!maximized"
                        width="12"
                        height="12"
                        viewBox="0 0 12 12"
                    >
                        <rect
                            x="1"
                            y="1"
                            width="10"
                            height="10"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="1.5"
                        />
                    </svg>
                    <svg v-else width="12" height="12" viewBox="0 0 12 12">
                        <rect
                            x="3"
                            y="1"
                            width="8"
                            height="8"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="1.5"
                        />
                        <rect
                            x="1"
                            y="3"
                            width="8"
                            height="8"
                            fill="#fff"
                            stroke="currentColor"
                            stroke-width="1.5"
                        />
                    </svg>
                </button>
                <button class="ctrl close" title="关闭" @click="close">
                    <svg width="12" height="12" viewBox="0 0 12 12">
                        <path
                            d="M2 2 L10 10 M10 2 L2 10"
                            stroke="currentColor"
                            stroke-width="1.5"
                        />
                    </svg>
                </button>
            </div>
        </div>
    </div>
</template>

<style scoped>
.titlebar {
    height: 38px;
    flex: 0 0 38px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 8px 0 12px;
    user-select: none;
    cursor: default;
    background: #f5f7fa;
    border-bottom: 1px solid #e5e7eb;
}
.brand {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 700;
    font-size: 14px;
    color: #1f2937;
}
.logo {
    font-size: 16px;
    cursor: inherit;
}
.name {
    letter-spacing: 0.3px;
    cursor: inherit;
}

.actions {
    display: flex;
    align-items: center;
    height: 100%;
}
.action {
    height: 26px;
    margin-right: 6px;
    padding: 0 12px;
    border: none;
    border-radius: 6px;
    background: transparent;
    color: #374151;
    font-size: 13px;
    cursor: pointer;
    transition: background 0.15s;
}
.action:hover {
    background: #e5e7eb;
}
.controls {
    display: flex;
    align-items: center;
    height: 100%;
}
.ctrl {
    width: 42px;
    height: 100%;
    border: none;
    background: transparent;
    color: #374151;
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
    transition: background 0.15s;
}
.ctrl:hover {
    background: #e5e7eb;
}
.ctrl.close:hover {
    background: #e81123;
    color: #fff;
}
.ctrl svg {
    display: block;
}
</style>
