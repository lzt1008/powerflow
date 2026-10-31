import type { StatusBarItem, Theme } from '@/bindings'
import { defineStore } from 'pinia'
import { ref } from 'vue'

const VALID_STATUS_BAR_ITEMS: StatusBarItem[] = ['system', 'screen', 'heatpipe']
const MIN_INTERVAL = 500
const MAX_INTERVAL = 60_000

export const usePreference = defineStore('preference', () => {
  const theme = ref<Theme>('system')
  const animationsEnabled = ref(true)
  const updateInterval = ref(1500)
  const language = ref('zh-CN')
  const statusBarItem = ref<StatusBarItem>('system')
  const statusBarShowCharging = ref(true)

  return {
    theme,
    animationsEnabled,
    updateInterval,
    language,
    statusBarItem,
    statusBarShowCharging,
  }
}, {
  tauri: {
    saveOnChange: true,
    saveStrategy: 'debounce',
    saveInterval: 1000,
  },
})

export function usePreferenceAsync() {
  const preference = usePreference()
  const isLoading = ref(true)
  preference.$tauri.start().then(() => {
    // Sanitize values that may have been persisted by older buggy builds
    // (e.g. statusBarItem:"none", updateInterval:86400000). The corrected
    // values are written back via saveOnChange.
    if (!VALID_STATUS_BAR_ITEMS.includes(preference.statusBarItem)) {
      console.warn('[preference] invalid statusBarItem', preference.statusBarItem, 'reset to system')
      preference.statusBarItem = 'system'
    }
    if (preference.updateInterval < MIN_INTERVAL || preference.updateInterval > MAX_INTERVAL) {
      console.warn('[preference] invalid updateInterval', preference.updateInterval, 'reset to 2000')
      preference.updateInterval = 2000
    }
    isLoading.value = false
  })
  return { preference, isLoading }
}
