import { fileURLToPath, URL } from "node:url";

import { configDefaults, defineConfig } from "vite-plus";
import vue from "@vitejs/plugin-vue";
import vueJsx from "@vitejs/plugin-vue-jsx";
import ElementPlus from "unplugin-element-plus/vite";
import vueDevTools from "vite-plugin-vue-devtools";
import wasm from "vite-plugin-wasm";

// https://vite.dev/config/
export default defineConfig({
  fmt: {},
  test: {
    environment: "jsdom",
    exclude: [...configDefaults.exclude, "e2e/**"],
  },
  lint: {
    jsPlugins: [{ name: "vite-plus", specifier: "vite-plus/oxlint-plugin" }],
    rules: { "vite-plus/prefer-vite-plus-imports": "error" },
    options: {
      typeAware: true,
      typeCheck: true,
    },
  },
  base: "/",
  build: {
    rolldownOptions: {
      output: {
        codeSplitting: {
          groups: [
            {
              name: "vue-vendor",
              test: /node_modules\/(?:@vue|vue|vue-i18n)\//,
            },
            {
              name: "element-plus-icons",
              test: /node_modules\/@element-plus\/icons-vue\//,
            },
            {
              name: "element-plus-components-a-l",
              test: /node_modules\/element-plus\/es\/components\/[a-l][^/]*\//,
            },
            {
              name: "element-plus-components-m-z",
              test: /node_modules\/element-plus\/es\/components\/[m-z][^/]*\//,
            },
            {
              name: "element-plus",
              test: /node_modules\/element-plus\//,
            },
          ],
        },
      },
    },
  },
  plugins: [vue(), vueJsx(), ElementPlus({}), vueDevTools(), wasm()],
  resolve: {
    alias: {
      "@": fileURLToPath(new URL("./src", import.meta.url)),
    },
  },
  optimizeDeps: {
    exclude: ["./pkg"],
  },
});
