import './assets/main.css'

import { createApp } from 'vue'
import ElementPlus from 'element-plus'
import 'element-plus/dist/index.css'
import * as ElementPlusIconsVue from '@element-plus/icons-vue'
import App from './App.vue'
import i18n from './utils/i18n'
import { initWasm } from './utils/wasm'

const app = createApp(App)

// Register Element Plus
app.use(ElementPlus)
app.use(i18n)

// Register all icons
for (const [key, component] of Object.entries(ElementPlusIconsVue)) {
  app.component(key, component)
}

// Mount the application
app.mount('#app')

// Initialize WASM asynchronously (does not block app rendering)
initWasm().catch((error) => {
  console.error('Failed to initialize WASM:', error)
})
