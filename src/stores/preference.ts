import type { StatusBarItem, Theme } from '@/bindings'
import { defineStore } from 'pinia'
import { ref } from 'vue'

export const usePreference = defineStore('preference', () => {
  const theme = ref<Theme>('system')
  const animationsEnabled = ref(true)
  const updateInterval = ref(1500)
  const language = ref('en')
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
