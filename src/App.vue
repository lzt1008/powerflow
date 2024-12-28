<script setup lang="ts">
import { events } from './bindings'

useSetup()

const tab = useTab()
const data = usePowerData()
const titleBar = useTitlebar()

const target = useTemplateRef<HTMLElement>('target')
const { y } = useScroll(target)

const preference = usePreference()
const preferDark = usePreferredDark()

preference.$tauri.start().then(() => {
  document.documentElement.classList.toggle('dark', preference.theme === 'system'
    ? preferDark.value
    : preference.theme === 'dark')
})

events.preferenceEvent.listen(({ payload }) => {
  if ('theme' in payload) {
    document.documentElement.classList.toggle('dark', payload.theme === 'system'
      ? preferDark.value
      : payload.theme === 'dark')
  }
})

watchEffect(() => {
  titleBar.shouldDisplayShadow.value = y.value > 0
})

onMounted(() => {
  events.windowLoadedEvent.emit()
})
</script>

<template>
  <Tabs
    v-model="tab"
    default-value="local"
  >
    <TitleBar />
    <div
      ref="target"
      class="h-[calc(100vh-52px)] pb-4 overflow-scroll "
    >
      <TabsContent value="local">
        <main class="px-4">
          <MainContent />
        </main>
      </TabsContent>
      <TabsContent
        v-for="udid in Object.keys(data.remote)"
        :key="udid"
        :value="udid"
      >
        <main class="px-4">
          <MainContent />
        </main>
      </TabsContent>
    </div>
  </Tabs>
</template>
