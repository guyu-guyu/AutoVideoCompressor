<script setup lang="ts">
import { computed } from "vue";
import type { DirConfigView, Template } from "../types";
import {
    NCard,
    NSpace,
    NCheckbox,
    NSelect,
    NInput,
    NInputNumber,
    NText,
    NDynamicInput,
    NGrid,
    NGi,
} from "naive-ui";

const props = defineProps<{
    modelValue: DirConfigView;
    templates: Template[];
}>();
const emit = defineEmits<{ "update:modelValue": [v: DirConfigView] }>();

// 纯单向数据流：不持有本地副本、不使用任何 watch。
// 读：template 直接读 props.modelValue.xxx（父层最新值）。
// 写：任一字段变化时，拷贝一份 + 改该字段 + 归一化，emit 给父持久化。
// 因为没有 watch(props) 回填，父 emit 后不会再触发子 emit —— 从根上杜绝
// 「父→子→父」无限回填循环（该循环曾导致输入框失焦即还原）。
function emitNormalized(next: DirConfigView) {
    const data: Record<string, unknown> = { ...next };
    // 数字输入框清空时统一归一化为 null（后端契约要求 number|null）
    if (data.minSizeMb === "" || data.minSizeMb === undefined)
        data.minSizeMb = null;
    if (data.maxSizeMb === "" || data.maxSizeMb === undefined)
        data.maxSizeMb = null;
    if (data.maxCompressSizeMb === "" || data.maxCompressSizeMb === undefined)
        data.maxCompressSizeMb = null;
    emit("update:modelValue", data as unknown as DirConfigView);
}

// 改单个字段：基于当前 modelValue 生成新对象再 emit（等价于对该字段的 v-model 写）
function setField<K extends keyof DirConfigView>(
    key: K,
    value: DirConfigView[K],
) {
    emitNormalized({ ...props.modelValue, [key]: value });
}

// 改重命名规则数组中某一项的某个字段。
// 注意：必须就地修改、保留该行对象的【引用不变】。n-dynamic-input 按对象引用
// 在内部 map 里给每行分配 Vue key（DynamicInput 源码 ensureKey/globalDataKeyMap）；
// 若这里用 { ...r } 生成新对象引用，该行会被分配新 key → Vue 销毁重建整行 DOM
// → 输入框每敲一个字符就失焦。props 是 shallowReadonly，嵌套对象允许就地修改，
// 父层持有同一响应式对象，保存时能读到最新值。
function updateRename(
    index: number,
    key: "pattern" | "replacement",
    value: string,
) {
    props.modelValue.renameRules[index][key] = value;
}

// isCustom reads directly from the data model (persisted in config file)
const isCustom = computed(() => props.modelValue.useCustomParams);

function onCustomToggle(checked: boolean) {
    if (checked) {
        // template mode → custom mode: resolve template to actual params
        const tmpl = props.templates.find(
            (t) => t.name === props.modelValue.params,
        );
        emitNormalized({
            ...props.modelValue,
            params: tmpl ? tmpl.params : props.modelValue.params,
            useCustomParams: true,
        });
    } else {
        // custom mode → template mode: set params to first template name
        emitNormalized({
            ...props.modelValue,
            params:
                props.templates.length > 0
                    ? props.templates[0].name
                    : props.modelValue.params,
            useCustomParams: false,
        });
    }
}

function onParamInput(value: string) {
    if (isCustom.value) setField("params", value);
}

const templateOptions = computed(() =>
    props.templates.map((t) => ({ label: t.name, value: t.name })),
);

// Current selected template's resolved params (for readonly display)
const selectedTemplateParams = computed(() => {
    const t = props.templates.find((t) => t.name === props.modelValue.params);
    return t ? t.params : "";
});
</script>

<template>
    <div class="form">
        <n-card size="small" title="调度" :bordered="true">
            <n-space align="center">
                <n-text>每天执行于</n-text>
                <n-input
                    :value="props.modelValue.scheduleTime"
                    placeholder="HH:MM"
                    style="width: 120px"
                    @update:value="(v: string) => setField('scheduleTime', v)"
                />
            </n-space>
        </n-card>

        <n-card size="small" title="压缩参数" :bordered="true">
            <n-space vertical :size="8">
                <n-checkbox
                    :checked="isCustom"
                    @update:checked="onCustomToggle"
                >
                    自定义参数
                </n-checkbox>

                <!-- 模板模式：仅显示下拉框 + 当前选择参数详情（不显示参数输入框） -->
                <template v-if="!isCustom">
                    <n-select
                        :value="props.modelValue.params"
                        :options="templateOptions"
                        style="width: 100%"
                        @update:value="(v: string) => setField('params', v)"
                    />
                    <n-text v-if="selectedTemplateParams" depth="3">
                        当前选择: <code>{{ selectedTemplateParams }}</code>
                    </n-text>
                </template>

                <!-- 自定义模式：仅显示一个参数输入框，宽度随窗口缩放 -->
                <n-input
                    v-else
                    :value="props.modelValue.params"
                    placeholder="裸 ffmpeg 参数，如 -c:v libx265 -crf 18"
                    style="width: 100%"
                    @update:value="onParamInput"
                />
            </n-space>
        </n-card>

        <n-card size="small" title="白名单 INCLUDE(必需)" :bordered="true">
            <n-dynamic-input
                :value="props.modelValue.include"
                :on-create="() => ''"
                placeholder="输入包含规则"
                @update:value="
                    (v: unknown[]) => setField('include', v as string[])
                "
            />
        </n-card>

        <n-card size="small" title="黑名单 EXCLUDE" :bordered="true">
            <n-dynamic-input
                :value="props.modelValue.exclude"
                :on-create="() => ''"
                placeholder="输入排除规则"
                @update:value="
                    (v: unknown[]) => setField('exclude', v as string[])
                "
            />
        </n-card>

        <n-card size="small" title="单次压缩限制" :bordered="true">
            <n-space vertical :size="4">
                <n-space align="center">
                    <n-text>单次压缩最大尺寸</n-text>
                    <n-input-number
                        :value="props.modelValue.maxCompressSizeMb"
                        placeholder="空=不限制"
                        style="width: 200px"
                        @update:value="
                            (v: number | null) =>
                                setField('maxCompressSizeMb', v)
                        "
                    />
                </n-space>
                <n-text depth="3"
                    >超过此总大小的文件将在下次压缩中被跳过</n-text
                >
            </n-space>
        </n-card>

        <n-card size="small" title="过滤 FILTERS" :bordered="true">
            <n-grid :cols="2" :x-gap="8" :y-gap="8" item-responsive>
                <n-gi
                    ><n-space align="center"
                        ><n-text>最小 MB</n-text
                        ><n-input-number
                            :value="props.modelValue.minSizeMb"
                            style="width: 100%"
                            @update:value="
                                (v: number | null) => setField('minSizeMb', v)
                            " /></n-space
                ></n-gi>
                <n-gi
                    ><n-space align="center"
                        ><n-text>最大 MB</n-text
                        ><n-input-number
                            :value="props.modelValue.maxSizeMb"
                            style="width: 100%"
                            @update:value="
                                (v: number | null) => setField('maxSizeMb', v)
                            " /></n-space
                ></n-gi>
                <n-gi
                    ><n-space align="center"
                        ><n-text>修改 ≥</n-text
                        ><n-input
                            :value="props.modelValue.mtimeAfter"
                            placeholder="YYYY-MM-DD"
                            @update:value="
                                (v: string) => setField('mtimeAfter', v)
                            " /></n-space
                ></n-gi>
                <n-gi
                    ><n-space align="center"
                        ><n-text>修改 ≤</n-text
                        ><n-input
                            :value="props.modelValue.mtimeBefore"
                            placeholder="YYYY-MM-DD"
                            @update:value="
                                (v: string) => setField('mtimeBefore', v)
                            " /></n-space
                ></n-gi>
                <n-gi
                    ><n-space align="center"
                        ><n-text>创建 ≥</n-text
                        ><n-input
                            :value="props.modelValue.ctimeAfter"
                            placeholder="YYYY-MM-DD"
                            @update:value="
                                (v: string) => setField('ctimeAfter', v)
                            " /></n-space
                ></n-gi>
                <n-gi
                    ><n-space align="center"
                        ><n-text>创建 ≤</n-text
                        ><n-input
                            :value="props.modelValue.ctimeBefore"
                            placeholder="YYYY-MM-DD"
                            @update:value="
                                (v: string) => setField('ctimeBefore', v)
                            " /></n-space
                ></n-gi>
            </n-grid>
        </n-card>

        <n-card size="small" title="重命名规则" :bordered="true">
            <n-dynamic-input
                :value="props.modelValue.renameRules"
                :on-create="() => ({ pattern: '', replacement: '' })"
                @update:value="
                    (v: unknown[]) =>
                        setField(
                            'renameRules',
                            v as DirConfigView['renameRules'],
                        )
                "
            >
                <template
                    #default="{
                        value,
                        index,
                    }: {
                        value: DirConfigView['renameRules'][number];
                        index: number;
                    }"
                >
                    <div class="rename-row">
                        <n-input
                            :value="value.pattern"
                            placeholder="pattern"
                            @update:value="
                                (v: string) => updateRename(index, 'pattern', v)
                            "
                        />
                        <n-text>→</n-text>
                        <n-input
                            :value="value.replacement"
                            placeholder="replacement"
                            @update:value="
                                (v: string) =>
                                    updateRename(index, 'replacement', v)
                            "
                        />
                    </div>
                </template>
            </n-dynamic-input>
        </n-card>
    </div>
</template>

<style scoped>
.form {
    display: flex;
    flex-direction: column;
    gap: 10px;
}
.rename-row {
    display: flex;
    gap: 8px;
    align-items: center;
    width: 100%;
}
code {
    background: #f0f0f0;
    padding: 1px 6px;
    border-radius: 3px;
}
</style>
