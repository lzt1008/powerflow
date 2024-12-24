import type { Component } from 'vue'
import { MotionPlugin } from '@vueuse/motion'
import { createPinia } from 'pinia'
import { createPlugin as createTauriPiniaPlugin } from 'tauri-plugin-pinia'
import { createApp } from 'vue'
import '../assets/index.css'

export function setup(entry: Component) {
  const pinia = createPinia().use(createTauriPiniaPlugin())

  createApp(entry)
    .use(MotionPlugin)
    .use(pinia)
    .mount('#app')
}
