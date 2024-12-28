<script setup lang="ts">
import { commands } from '@/bindings'
import { LaptopIcon, MobileIcon, Share2Icon } from '@radix-icons/vue'
import { SettingsIcon } from 'lucide-vue-next'

const tab = useTab()
const data = usePowerData()

const {
  tabName,
  tabNameLoading,
  shouldDisplayShadow,
} = useTitlebar()

</script>

<template>
  <div
    data-tauri-drag-region
    class="sticky top-0 flex justify-between items-center z-10 py-2 pr-4 transition-shadow bg-background"
    :class="{ shadow: shouldDisplayShadow }"
  >
    <div class="flex items-center gap-3 font-mono text-sm">
      <TabsList
        class="ml-[6rem]"
      >
        <TransitionGroup
          enter-from-class="w-0"
          leave-to-class="w-0"
          enter-to-class="w-[40px]"
          leave-from-class="w-[40px]"
          enter-active-class="duration-500"
          leave-active-class="duration-500"
        >
          <TabsTrigger key="local" class="px-0" value="local">
            <LaptopIcon class="mx-3 size-4" :class="[tab === 'local' ? 'text-green-500' : 'text-muted-foreground']" />
          </TabsTrigger>

          <TabsTrigger
            v-for="udid in Object.keys(data.remote)"
            :key="udid"
            class="px-0"
            :value="udid"
          >
            <MobileIcon class="mx-3 size-4" :class="[tab === udid ? 'text-green-500' : 'text-muted-foreground']" />
          </TabsTrigger>
        </TransitionGroup>
      </TabsList>
      <div class="flex flex-col -translate-y-[1px]">
        <Skeleton v-if="tabNameLoading" class="w-32 h-4" />
        <span v-else class="text-secondary-foreground font-bold">{{ tabName }}</span>
        <span class="text-[10px] leading-[10px] font-normal text-muted-foreground">
          {{ tab === 'local' ? 'Local' : Array.from(data.remote[tab].interface || []).join(' and ') || 'offline' }}
        </span>
      </div>
    </div>
    <div class="flex gap-2">
      <div class="rounded-md p-2 hover:bg-muted transition-colors cursor-pointer">
        <Share2Icon class="text-muted-foreground size-5" />
      </div>
      <div class="rounded-md p-2 hover:bg-muted transition-colors cursor-pointer">
        <SettingsIcon
          :stroke-width="1.5"
          class="text-muted-foreground size-5"
          @click="commands.openSettings()"
        />
      </div>
    </div>
  </div>
</template>
