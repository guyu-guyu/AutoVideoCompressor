<script setup lang="ts">
// App.vue 只负责提供 Naive UI 的 provider 外壳（config / message / dialog）。
// 注意：useDialog()/useMessage() 必须在 provider 的【后代组件】里调用，
// 不能在本组件的 setup 里调用——否则 inject 拿不到实例会抛错导致整页空白。
// 真正的内容与 useDialog/useMessage 桥接放在 AppShell.vue（provider 的后代）。
import { NConfigProvider, NMessageProvider, NDialogProvider } from "naive-ui";
import { themeOverrides } from "./theme";
import AppShell from "./AppShell.vue";
</script>

<template>
  <n-config-provider :theme-overrides="themeOverrides">
    <n-message-provider>
      <n-dialog-provider>
        <AppShell />
      </n-dialog-provider>
    </n-message-provider>
  </n-config-provider>
</template>
