import type { Component } from 'vue'
import messages from '@intlify/unplugin-vue-i18n/messages'
import { MotionPlugin } from '@vueuse/motion'
import { createPinia } from 'pinia'
import { createPlugin as createTauriPiniaPlugin } from 'tauri-plugin-pinia'
import { createApp } from 'vue'
import { createI18n } from 'vue-i18n'
import '../assets/index.css'

const i18n = createI18n({
  locale: 'en',
  fallbackLocale: 'en',
  messages,
})

export function setup(entry: Component) {
  const pinia = createPinia().use(createTauriPiniaPlugin())

  createApp(entry)
    .use(MotionPlugin)
    .use(pinia)
    .use(i18n)
    .mount('#app')
}
