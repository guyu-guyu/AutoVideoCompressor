import { describe, it, expect, vi, beforeEach } from "vitest";
import { config as testUtilsConfig, shallowMount } from "@vue/test-utils";
import { nextTick } from "vue";
import { NCheckbox } from "naive-ui";
import GlobalSettings from "./GlobalSettings.vue";
import type { GlobalConfig } from "../types";

testUtilsConfig.global.renderStubDefaultSlot = true;

const getGlobalConfig = vi.hoisted(() => vi.fn());
const saveGlobalConfig = vi.hoisted(() => vi.fn());
const refreshCards = vi.hoisted(() => vi.fn());
const refreshFfmpeg = vi.hoisted(() => vi.fn());

vi.mock("../api/tauri", () => ({
    api: { getGlobalConfig, saveGlobalConfig },
}));
vi.mock("../stores/app", () => ({
    useAppStore: () => ({ refreshCards, refreshFfmpeg }),
}));

function config(overrides: Partial<GlobalConfig> = {}): GlobalConfig {
    return {
        ffmpegPath: "",
        ffmpegTimeoutSeconds: 3600,
        minimizeToTray: true,
        startWithWindows: false,
        logRetentionDays: 90,
        language: "zh-CN",
        useWindowsTaskScheduler: false,
        wakeComputerForScheduledTasks: false,
        templates: [],
        ...overrides,
    };
}

async function mountSettings(value: GlobalConfig) {
    getGlobalConfig.mockResolvedValue(value);
    const wrapper = shallowMount(GlobalSettings, {
        global: {
            stubs: {
                NModal: {
                    template: '<div><slot /><slot name="footer" /></div>',
                },
            },
        },
    });
    await vi.waitFor(() => {
        expect(getGlobalConfig).toHaveBeenCalled();
        expect(wrapper.text()).toContain("使用 Windows 计划任务");
    });
    return wrapper;
}

describe("GlobalSettings Windows 计划任务设置", () => {
    beforeEach(() => {
        vi.clearAllMocks();
    });

    it("仅在启用 Windows 计划任务后显示唤醒选项", async () => {
        const wrapper = await mountSettings(config());
        expect(wrapper.text()).not.toContain("唤醒计算机执行任务");

        const schedulerCheckbox = wrapper.findAllComponents(NCheckbox)
            .find((checkbox) => checkbox.text().includes("使用 Windows 计划任务"));
        expect(schedulerCheckbox).toBeTruthy();
        schedulerCheckbox!.vm.$emit("update:checked", true);
        await nextTick();

        expect(wrapper.text()).toContain("唤醒计算机执行任务");
    });
});
