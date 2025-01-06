<script setup lang="ts">
useSetup()

const tab = useTab()
const data = usePowerData()
const titleBar = useTitlebar()

const target = useTemplateRef<HTMLElement>('target')
const { y } = useScroll(target)

watchEffect(() => {
  titleBar.shouldDisplayShadow.value = y.value > 0
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
      class="h-[calc(100vh-52px)] pb-4"
    >
      <TabsContent
        v-for="udid in ['local', ...Object.keys(data.remote)]"
        :key="udid"
        :value="udid"
        class="overflow-hidden h-full"
      >
        <RouterView v-slot="{ Component }">
          <Transition
            enter-from-class="opacity-0"
            leave-to-class="opacity-0"
            enter-to-class="opacity-100"
            leave-from-class="opacity-100"
            enter-active-class="duration-300"
            leave-active-class="duration-300"
          >
            <Component :is="Component" />
          </Transition>
        </RouterView>
      </TabsContent>
    </div>
  </Tabs>
</template>
