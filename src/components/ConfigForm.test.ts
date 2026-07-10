import { describe, it, expect, vi } from "vitest";
import { mount } from "@vue/test-utils";
import { ref, nextTick, h } from "vue";
import ConfigForm from "./ConfigForm.vue";
import type { DirConfigView, Template } from "../types";

// 复现二级界面配置 tab 输入框「失焦还原」问题的最小 harness：
// 模拟 TabConfig 用 v-model="view" 持有 ConfigForm 的数据（view 是 ref，写入普通对象后被 Vue 包成响应式 Proxy）。

function baseView(overrides: Partial<DirConfigView> = {}): DirConfigView {
  return {
    exists: true, valid: true, errorMessage: "",
    include: ["*.mp4"], exclude: [],
    maxSizeMb: null, minSizeMb: null,
    mtimeAfter: null, mtimeBefore: null, ctimeAfter: null, ctimeBefore: null,
    renameRules: [], params: "H.265", useCustomParams: false,
    maxCompressSizeMb: null, scheduleTime: "03:00",
    ...overrides,
  };
}
const templates: Template[] = [{ name: "H.265", params: "-c:v libx265 -crf 18" }];

describe("ConfigForm 双向绑定不回弹", () => {
  it("用户输入 scheduleTime 后，值稳定保留（不被父→子回流覆盖）", async () => {
    const view = ref<DirConfigView>(baseView());
    // 父组件：v-model 绑定到 view，完全模拟 TabConfig 的用法
    const Parent = {
      components: { ConfigForm },
      setup() {
        return () => h(ConfigForm, {
          modelValue: view.value,
          templates,
          "onUpdate:modelValue": (v: DirConfigView) => { view.value = v; },
        });
      },
    };
    const w = mount(Parent);
    await nextTick();

    // 找到调度时间输入框（第一个 n-input 的原生 input）
    const inputs = w.findAll("input");
    const scheduleInput = inputs[0];
    expect(scheduleInput).toBeTruthy();

    // 模拟用户输入 "a"
    await scheduleInput.setValue("a");
    await nextTick();
    await nextTick();

    // 关键断言：输入后 DOM 值应为 "a"，不应被回流覆盖还原为 "03:00"
    expect((scheduleInput.element as HTMLInputElement).value).toBe("a");
    // 父层 view 也应同步为 "a"
    expect(view.value.scheduleTime).toBe("a");
  });

  it("清空数字输入框时归一化为 null（后端契约 number|null）", async () => {
    const view = ref<DirConfigView>(baseView({ minSizeMb: 5 }));
    const Parent = {
      setup() {
        return () => h(ConfigForm, {
          modelValue: view.value,
          templates,
          "onUpdate:modelValue": (v: DirConfigView) => { view.value = v; },
        });
      },
    };
    const w = mount(Parent);
    await nextTick();
    // 找到「最小 MB」数字输入框并清空
    const numberInput = w.findAll("input").find(i =>
      (i.element as HTMLInputElement).value === "5");
    expect(numberInput).toBeTruthy();
    await numberInput!.setValue("");
    await nextTick();
    expect(view.value.minSizeMb).toBeNull();
  });

  it("编辑重命名规则时行不重建（焦点稳定）且值同步到父层", async () => {
    const view = ref<DirConfigView>(baseView({
      renameRules: [{ pattern: "old", replacement: "new" }],
    }));
    const Parent = {
      setup() {
        return () => h(ConfigForm, {
          modelValue: view.value,
          templates,
          "onUpdate:modelValue": (v: DirConfigView) => { view.value = v; },
        });
      },
    };
    const w = mount(Parent);
    await nextTick();

    // 定位 pattern 输入框（值为 "old"），记录其 DOM 节点
    const patternInput = w.findAll("input").find(i =>
      (i.element as HTMLInputElement).value === "old");
    expect(patternInput).toBeTruthy();
    const domBefore = patternInput!.element;

    // 逐字符输入模拟：改成 "olds"
    await patternInput!.setValue("olds");
    await nextTick();
    await nextTick();

    // 关键断言 1：值同步到父层
    expect(view.value.renameRules[0].pattern).toBe("olds");
    // 关键断言 2：输入框 DOM 节点未被重建（引用相同）→ 焦点不会丢失
    const patternAfter = w.findAll("input").find(i =>
      (i.element as HTMLInputElement).value === "olds");
    expect(patternAfter!.element).toBe(domBefore);
  });
});
