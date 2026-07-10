import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import { NSwitch } from "naive-ui";
import DirCard from "./DirCard.vue";
import type { DirCardInfo } from "../types";

// 回归：n-switch 的 update:value 传入 boolean 新值，toggle 必须用它调用后端。
const setDirectoryEnabled = vi.hoisted(() => vi.fn());
vi.mock("../api/tauri", () => ({ api: { setDirectoryEnabled } }));

function card(overrides: Partial<DirCardInfo> = {}): DirCardInfo {
  return {
    path: "D:/x", enabled: true, badge: "valid", badgeDetail: "",
    fileCount: 3, totalSize: 1024, paramsName: "H.265", cycleRiskCount: 0,
    lastRunTime: "", lastRunResult: "", nextRunTime: "今天 03:00",
    nextRunCount: 0, nextRunSize: 0, ...overrides,
  };
}

describe("DirCard", () => {
  beforeEach(() => { const _ = setActivePinia(createPinia()); });
  it("shows valid badge", () => {
    const w = mount(DirCard, { props: { card: card() } });
    expect(w.text()).toContain("● 有效");
  });
  it("shows overlap badge", () => {
    const w = mount(DirCard, { props: { card: card({ badge: "overlap", badgeDetail: "D:/parent" }) } });
    expect(w.text()).toContain("● 重叠");
    expect(w.text()).toContain("D:/parent");
  });
  it("shows cycle risk warning", () => {
    const w = mount(DirCard, { props: { card: card({ cycleRiskCount: 2 }) } });
    expect(w.text()).toContain("2 个循环风险");
  });
  it("toggle 用 n-switch 传入的新值调用 setDirectoryEnabled(回归: 不得把回调当 Event)", async () => {
    const w = mount(DirCard, { props: { card: card({ enabled: true }) } });
    // n-switch 点击切换时，update:value 回调收到的是新 boolean，而非 Event
    w.findComponent(NSwitch).vm.$emit("update:value", false);
    await vi.waitFor(() => {
      expect(setDirectoryEnabled).toHaveBeenCalledWith("D:/x", false);
    });
  });
});
