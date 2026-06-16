import "./assets/main.css";

import { createApp } from "vue";
import {
  ElButton,
  ElCard,
  ElCollapse,
  ElCollapseItem,
  ElContainer,
  ElDescriptions,
  ElDescriptionsItem,
  ElDivider,
  ElEmpty,
  ElForm,
  ElFormItem,
  ElHeader,
  ElIcon,
  ElInput,
  ElMain,
  ElOption,
  ElOptionGroup,
  ElScrollbar,
  ElSelect,
  ElTable,
  ElTableColumn,
  ElTag,
  ElTooltip,
  ElUpload,
} from "element-plus";
import App from "./App.vue";
import i18n from "./utils/i18n";
import { initWasm } from "./utils/wasm";

const app = createApp(App);

[
  ElButton,
  ElCard,
  ElCollapse,
  ElCollapseItem,
  ElContainer,
  ElDescriptions,
  ElDescriptionsItem,
  ElDivider,
  ElEmpty,
  ElForm,
  ElFormItem,
  ElHeader,
  ElIcon,
  ElInput,
  ElMain,
  ElOption,
  ElOptionGroup,
  ElScrollbar,
  ElSelect,
  ElTable,
  ElTableColumn,
  ElTag,
  ElTooltip,
  ElUpload,
].forEach((component) => {
  app.use(component);
});

app.use(i18n);

// Mount the application
app.mount("#app");

// Initialize WASM asynchronously (does not block app rendering)
initWasm().catch((error) => {
  console.error("Failed to initialize WASM:", error);
});
