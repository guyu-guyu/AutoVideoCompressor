import { describe, it, expect, vi, beforeEach } from "vitest";
import { mount } from "@vue/test-utils";
import { createPinia, setActivePinia } from "pinia";
import DirCard from "./DirCard.vue";
import type { DirCardInfo } from "../types";

vi.mock("../api/tauri", () => ({ api: {} }));

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
});
