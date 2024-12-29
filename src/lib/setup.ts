import messages from '@intlify/unplugin-vue-i18n/messages'
import { MotionPlugin } from '@vueuse/motion'
import { createPlugin as createTauriPiniaPlugin } from 'tauri-plugin-pinia'
import { createI18n } from 'vue-i18n'
import '../assets/index.css'

const i18n = createI18n({
  locale: 'en',
  fallbackLocale: 'en',
  messages,
})

export function setup(entry: Component) {
  createApp(entry)
    .use(MotionPlugin)
    .use(
      createPinia()
        .use(createTauriPiniaPlugin()),
    )
    .use(i18n)
    .mount('#app')
}
