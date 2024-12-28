import { commands, events } from '@/bindings'
import { useI18n } from 'vue-i18n'

export function useSetup() {
  const i18n = useI18n()
  const preferedLang = usePreferredLanguages()
  const preference = usePreference()
  const preferDark = usePreferredDark()

  i18n.locale.value = preferedLang.value[0]

  const toggleDark = () => {
    document.documentElement.classList.toggle('dark', preference.theme === 'system'
      ? preferDark.value
      : preference.theme === 'dark')
    commands.switchTheme(preference.theme)
  }

  preference.$tauri.start().then(() => {
    toggleDark()
    i18n.locale.value = preference.language
  })

  watchEffect(toggleDark)

  events.preferenceEvent.listen(({ payload }) => {
    if ('theme' in payload) {
      toggleDark()
    }
    if ('language' in payload) {
      i18n.locale.value = payload.language
    }
  })

  // notify rust to get a instant update
  onMounted(() => {
    events.windowLoadedEvent.emit()
  })
}
