import type { StatusBarItem, Theme } from '@/bindings'
import { defineStore } from 'pinia'
import { ref } from 'vue'

// Keep in sync with MIN_INTERVAL / MAX_INTERVAL in src-tauri/src/local.rs.
const DEFAULT_INTERVAL = 1500
const MIN_INTERVAL = 500
const MAX_INTERVAL = 60_000

export const usePreference = defineStore('preference', () => {
  const theme = ref<Theme>('system')
  const animationsEnabled = ref(true)
  const updateInterval = ref(DEFAULT_INTERVAL)
  const language = ref('en')
  const statusBarItem = ref<StatusBarItem>('system')
  const statusBarShowCharging = ref(true)
  const hideOnStartup = ref(false)

  return {
    theme,
    animationsEnabled,
    updateInterval,
    language,
    statusBarItem,
    statusBarShowCharging,
    hideOnStartup,
  }
}, {
  tauri: {
    saveOnChange: true,
    saveStrategy: 'debounce',
    saveInterval: 1000,
  },
})

const VALID_STATUS_BAR_ITEMS: StatusBarItem[] = ['system', 'screen', 'heatpipe']

export function usePreferenceAsync() {
  const preference = usePreference()
  const isLoading = ref(true)
  preference.$tauri.start().then(() => {
    // Sanitize values persisted by older or buggy builds.
    if (!VALID_STATUS_BAR_ITEMS.includes(preference.statusBarItem))
      preference.statusBarItem = 'system'
    if (!(preference.updateInterval >= MIN_INTERVAL && preference.updateInterval <= MAX_INTERVAL))
      preference.updateInterval = DEFAULT_INTERVAL
    isLoading.value = false
  })
  return { preference, isLoading }
}
