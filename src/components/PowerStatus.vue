<script setup lang="ts">
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import Progress from '@/components/ui/progress/Progress.vue'
import { usePower } from '@/composables'
import NumberFlow from '@number-flow/vue'
import { window } from '@tauri-apps/api'
import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { useWindowFocus } from '@vueuse/core'
import { useMotion } from '@vueuse/motion'
import { Battery, BatteryCharging, ExternalLink } from 'lucide-vue-next'
import { onUnmounted, ref, watch } from 'vue'
import Skeleton from './ui/skeleton/Skeleton.vue'

const showOpenLink = ref(false)
const power = usePower()

const focused = useWindowFocus()

const target = ref<HTMLElement | null>(null)

const motion = useMotion(target, {
  initial: {
    opacity: 0,
    scale: 0.99,
    transition: {
      duration: 200,
      type: 'spring',
    },
  },
  visible: {
    opacity: 1,
    scale: 1,
    transition: {
      duration: 300,
      type: 'spring',
    },
  },
})

watch(focused, () => {
  invoke<boolean>('is_main_window_hidden').then((res) => {
    showOpenLink.value = res
  })

  if (!focused.value) {
    // motion.apply('initial')
    // setTimeout(() => {
    //   window.getCurrentWindow().hide()
    // }, 300)
  }
  else {
    motion.apply('visible')
  }
})

const unlisten = listen('hide-popover', () => {
  if (window.getCurrentWindow().label !== 'popover') {
    return
  }

  motion.apply('initial')

  setTimeout(() => {
    window.getCurrentWindow().hide()
    // window.getAllWindows()
    //   .then(w => w.find(w => w.label === 'popover')?.hide())
  }, 300)
})

onUnmounted(() => {
  unlisten.then(f => f())
})
</script>

<template>
  <div
    ref="target"
    class="relative max-w-max min-w-80 basis-1/3 flex gap-4"
  >
    <Card class="flex-1">
      <CardHeader class="space-y-0 pb-2 gap-y-0">
        <CardTitle class="flex items-center justify-between gap-2 text-base truncate">
          <div class="mr-10 flex gap-2">
            {{ power.isCharging ? 'Charging Power' : 'System Power' }}
            <a
              v-if="showOpenLink" class="flex items-center gap-2 text-[10px] text-foreground/70 cursor-pointer"
              @click="invoke('open_app')"
            >
              <ExternalLink class="h-4 w-4" />
            </a>
          </div>

          <div
            v-if="power.isReady"
            class="rounded-md dark:bg-green-600 bg-green-500 px-2 py-1 text-xs truncate font-mono"
          >
            <span class="font-bold mr-1 text-background">{{ power.adapterDetails.watts }}W</span>
            <span class="text-[10px] text-background/80">({{ power.adapterDetails.voltage / 1000 }}V,{{
              power.adapterDetails.amperage / 1000 }}A)</span>
          </div>
          <Skeleton v-else class="w-24 h-6" />
        </CardTitle>
        <CardDescription class="text-[10px] font-mono">
          <template v-if="power.isReady">
            {{ power.adapterDetails.name }}
          </template>
          <Skeleton v-else class="w-20 h-[10px]" />
        </CardDescription>
      </CardHeader>
      <CardContent class="space-y-4 mt-1">
        <div class="flex flex-col gap-6">
          <div class="flex gap-2">
            <NumberFlow
              v-if="power.isReady" class="text-4xl font-bold"
              :format="{ maximumFractionDigits: 1, minimumFractionDigits: 1 }"
              :value="power.isCharging ? power.systemIn : power.systemPower" suffix="w"
            />
            <Skeleton v-else class="w-40 h-[50px] mt-2" />
          </div>
        </div>

        <div class="flex flex-col gap-2">
          <div class="flex items-center justify-between">
            <div v-if="power.isReady" class="flex items-center gap-2">
              <BatteryCharging v-if="power.isCharging" class="h-5 w5 text-green-500" />
              <Battery v-else class="h-5 w-5" />
              <span class="text-xl font-bold mr-4">
                {{ power.batteryLevel.toFixed(2) }}%</span>
            </div>
            <Skeleton v-else class="w-24 h-7" />

            <div
              v-if="power.isReady" class="text-sm font-medium truncate"
              :class="power.isCharging ? 'text-green-500' : 'text-muted-foreground'"
            >
              <span class="font-semibold">{{ power.isCharging ? 'Charging' : 'On Battery' }}</span>
              • {{ power.timeRemaining }}
            </div>
            <Skeleton v-else class="w-32 h-5" />
          </div>
          <Progress v-if="power.isReady" :model-value="isNaN(power?.batteryLevel) ? 0 : power?.batteryLevel" />
          <Skeleton v-else class="w-full h-2" />
        </div>
      </CardContent>
    </Card>
  </div>
</template>
