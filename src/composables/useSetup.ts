import { commands, events } from '@/bindings'

export function useSetup() {
  const preference = usePreference()
  const preferDark = usePreferredDark()

  const toggleDark = () => {
    document.documentElement.classList.toggle('dark', preference.theme === 'system'
      ? preferDark.value
      : preference.theme === 'dark')
    commands.switchTheme(preference.theme)
  }

  preference.$tauri.start().then(toggleDark)

  watchEffect(toggleDark)

  events.preferenceEvent.listen(({ payload }) => {
    if ('theme' in payload) {
      toggleDark()
    }
  })

  // notify rust to get a instant update
  onMounted(() => {
    events.windowLoadedEvent.emit()
  })
}
