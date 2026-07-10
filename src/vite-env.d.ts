/// <reference types="vite/client" />

declare module "*.vue" {
  import type { DefineComponent } from "vue";
  const component: DefineComponent<object, object, unknown>;
  export default component;
}

import type { DialogApi, MessageApi } from "naive-ui";
declare global {
  interface Window {
    $dialog: DialogApi;
    $message: MessageApi;
  }
}
