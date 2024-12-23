<script setup lang="ts">
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { LaptopIcon, MobileIcon, Share2Icon } from '@radix-icons/vue'
import { emit } from '@tauri-apps/api/event'
import { useScroll } from '@vueuse/core'
import { SettingsIcon } from 'lucide-vue-next'
import { onMounted, ref, watchEffect } from 'vue'
import PowerAnalytics from './components/PowerAnalytics.vue'

const shouldDisplayShadow = ref(false)
const target = ref<HTMLElement | null>(null)
const { y } = useScroll(target)

watchEffect(() => {
  shouldDisplayShadow.value = y.value > 0
})

onMounted(() => {
  emit('window:load')
})
</script>

<template>
  <Tabs default-value="mac">
    <div
      data-tauri-drag-region
      class="sticky top-0 flex justify-between items-center z-10 py-2 pr-4 transition-shadow"
      :class="{ shadow: shouldDisplayShadow }"
    >
      <div class="flex items-center gap-3 font-mono text-sm">
        <TabsList class="ml-[6rem]">
          <TabsTrigger value="mac">
            <LaptopIcon class="size-4 text-green-500" />
          </TabsTrigger>
          <TabsTrigger value="iphone">
            <MobileIcon class="size-4 text-muted-foreground" />
          </TabsTrigger>
        </TabsList>
        <div class="flex flex-col -translate-y-[1px]">
          <span class="text-secondary-foreground font-bold">Macbook Pro</span>
          <span class="text-[10px] leading-[10px] font-normal text-muted-foreground">2022</span>
        </div>
      </div>
      <div class="flex gap-2">
        <div class="rounded-md p-2 hover:bg-muted transition-colors cursor-pointer">
          <Share2Icon class="text-muted-foreground size-5" />
        </div>
        <div class="rounded-md p-2 hover:bg-muted transition-colors cursor-pointer">
          <SettingsIcon class="text-muted-foreground size-5" />
        </div>
      </div>
    </div>

    <div ref="target" class="h-[calc(100vh-52px)] pb-4 overflow-scroll bg-background">
      <TabsContent value="mac" class="overflow-scroll">
        <main class="bg-background container px-4">
          <PowerAnalytics class="overflow-scroll" />
        </main>
      </TabsContent>
      <TabsContent value="iphone" class="h-full">
        <div class="flex items-center justify-center h-full font-mono text-4xl">
          todo!()
        </div>
      </TabsContent>
    </div>
  </Tabs>
</template>

<style scoped>
</style>
