import type { GlobalThemeOverrides } from "naive-ui";

// 主色沿用原手写样式中的蓝色 #06c，统一为 Naive UI 主题 token
export const themeOverrides: GlobalThemeOverrides = {
  common: {
    primaryColor: "#0066cc",
    primaryColorHover: "#1a75d1",
    primaryColorPressed: "#0052a3",
    primaryColorSuppl: "#1a75d1",
    borderRadius: "6px",
    fontSize: "14px",
  },
  Card: {
    borderRadius: "8px",
  },
  Button: {
    borderRadiusMedium: "6px",
  },
};
