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
