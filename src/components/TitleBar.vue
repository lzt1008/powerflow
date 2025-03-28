<script setup lang="ts">
import { commands } from '@/bindings'
import { LaptopIcon, MobileIcon } from '@radix-icons/vue'
import { ChevronLeft, History, SettingsIcon } from 'lucide-vue-next'

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
    class="sticky top-0 flex justify-between items-center z-10 pt-2 pr-4 transition-shadow bg-background"
    :class="{ shadow: shouldDisplayShadow }"
  >
    <div class="ml-[6rem] flex items-center gap-2">
      <div
        v-if="$route.path !== '/'"
        class="p-1 rounded-md transition-colors hover:bg-muted cursor-pointer"
        @click="$router.back()"
      >
        <ChevronLeft class="size-6 text-muted-foreground -translate-x-px" />
      </div>
      <div v-else class="flex items-center gap-3 font-mono text-sm">
        <TabsList>
          <TransitionGroup
            enter-from-class="w-0"
            leave-to-class="w-0"
            enter-to-class="w-[40px]"
            leave-from-class="w-[40px]"
            enter-active-class="duration-500"
            leave-active-class="duration-500"
          >
            <TabsTrigger key="local" class="px-0" value="local">
              <LaptopIcon class="mx-3 size-4" :class="[tab === 'local' ? 'text-blue-500' : 'text-muted-foreground']" />
            </TabsTrigger>

            <TabsTrigger
              v-for="udid in Object.keys(data.remote)"
              :key="udid"
              class="px-0"
              :value="udid"
            >
              <MobileIcon class="mx-3 size-4" :class="[tab === udid ? 'text-blue-500' : 'text-muted-foreground']" />
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
    </div>
    <div class="flex gap-2">
      <div
        class="rounded-md p-2 hover:bg-muted transition-colors cursor-pointer"
        @click="$route.path === '/history' ? $router.back() : $router.push('/history')"
      >
        <CommonTooltip content="History" as-child>
          <History
            :stroke-width="1.8"
            class="text-muted-foreground size-5 transition-transform duration-300"
            :class="{ 'text-secondary-foreground rotate-[-360deg]': $route.path === '/history' }"
          />
        </CommonTooltip>
      </div>
      <div
        class="rounded-md p-2 hover:bg-muted transition-colors cursor-pointer"
        @click="commands.openSettings()"
      >
        <CommonTooltip content="Settings" as-child>
          <SettingsIcon
            :stroke-width="1.8"
            class="text-muted-foreground size-5"
          />
        </CommonTooltip>
      </div>
    </div>
  </div>
</template>
