<script setup lang="ts">
import { events } from '@/bindings'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import Progress from '@/components/ui/progress/Progress.vue'
import { usePower } from '@/composables'
import { usePreference } from '@/stores/preference'
import NumberFlow from '@number-flow/vue'
import { window } from '@tauri-apps/api'
import { invoke } from '@tauri-apps/api/core'
import { useWindowFocus } from '@vueuse/core'
import { useMotion } from '@vueuse/motion'
import { addMinutes, format, formatDistanceToNow } from 'date-fns'
import { ArrowUpDown, BatteryCharging, BatteryFull, BatteryMedium, ExternalLink } from 'lucide-vue-next'
import { computed, onUnmounted, ref, watch } from 'vue'
import Skeleton from './ui/skeleton/Skeleton.vue'

const showOpenLink = ref(false)

const power = usePower()

const preferenceIsLoading = ref(true)
const preference = usePreference()

preference.$tauri.start().then(() => {
  preferenceIsLoading.value = false
})

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

const showRemainDuration = ref(false)
const buttonText = computed(() => {
  if (showRemainDuration.value) {
    return power.value.timeRemaining
  }
  return format(
    addMinutes(new Date(), power.value.io.timeRemaining),
    'HH:mm',
  )
})

const focused = useWindowFocus()
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

const unlisten = events.hidePopoverEvent.listen(() => {
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

          <Skeleton v-if="power.isLoading" class="w-24 h-6" />
          <div
            v-else-if="power.isCharging"
            class="rounded-md dark:bg-green-600 bg-green-500 px-2 py-1 text-xs truncate font-mono"
          >
            <span class="font-bold mr-1 text-background">{{ power.adapterDetails.watts }}W</span>
            <span class="text-[10px] text-background/80">({{ (power.adapterDetails?.voltage || 0) / 1000 }}V,{{
              (power.adapterDetails?.amperage || 0) / 1000 }}A)</span>
          </div>
          <div
            v-else
            class="rounded-md dark:bg-green-600 bg-green-500 px-2 py-1 text-xs truncate font-mono w-20 text-background flex items-center justify-center
            cursor-pointer hover:bg-green-600 transition-colors
            "
            @click="showRemainDuration = !showRemainDuration"
          >
            <span class="font-bold mr-1">{{ buttonText }}</span>
            <ArrowUpDown
              class="size-3 text-background/80 transition-transform duration-300"
              :class="{ 'rotate-180': showRemainDuration }"
            />
          </div>
        </CardTitle>
        <CardDescription class="text-[10px] font-mono">
          <template v-if="power.isReady">
            {{ power.isCharging ? power.adapterDetails.name : 'On Battery' }}
          </template>
          <Skeleton v-else class="w-20 h-[10px]" />
        </CardDescription>
      </CardHeader>
      <CardContent class="space-y-4 mt-1">
        <div class="flex flex-col gap-6">
          <div class="flex gap-2">
            <Skeleton v-if="power.isLoading || preferenceIsLoading" class="w-40 h-[50px] mt-2" />
            <NumberFlow
              v-else-if="preference.animationsEnabled" class="text-4xl font-bold"
              :format="{ maximumFractionDigits: 1, minimumFractionDigits: 1 }"
              :value="power.isCharging ? power.systemIn : power.systemPower" suffix="w"
            />
            <div v-else class="text-4xl leading-[54px] font-bold">
              {{ (power.isCharging ? power.systemIn : power.systemPower).toFixed(1) }}w
            </div>
          </div>
        </div>

        <div class="flex flex-col gap-2">
          <div class="flex items-center justify-between">
            <div v-if="power.isReady" class="flex items-center gap-2">
              <BatteryCharging v-if="power.isCharging" class="size-5 text-green-500" />
              <BatteryFull v-else-if="power.batteryLevel > 0.66" class="size-5" />
              <BatteryMedium v-else-if="power.batteryLevel > 0.33" class="size-5" />
              <BatteryLow
                v-else
                class="size-5"
                :class="{
                  'text-red-500': power.batteryLevel < 0.1,
                }"
              />
              <span class="text-xl font-bold mr-4">
                {{ power.batteryLevel.toFixed(2) }}%</span>
            </div>
            <Skeleton v-else class="w-24 h-7" />

            <div
              v-if="power.isReady" class="text-sm font-medium truncate"
              :class="power.isCharging ? 'text-green-500' : 'text-muted-foreground'"
            >
              <span class="font-semibold">{{
                formatDistanceToNow(addMinutes(new Date(), power.isCharging
                  ? power.smc.timeToFull
                  : power.smc.timeToEmpty,
                )) }}</span>
              <span> to {{ power.isCharging ? 'full' : 'empty' }}</span>
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
