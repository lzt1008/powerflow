<script setup lang="ts">
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import Progress from '@/components/ui/progress/Progress.vue'
import { usePower } from '@/composables'
import NumberFlow from '@number-flow/vue'
import { invoke } from '@tauri-apps/api/core'
import { useWindowFocus } from '@vueuse/core'
import { Battery, BatteryCharging, ExternalLink } from 'lucide-vue-next'
import { ref, watch } from 'vue'

const showOpenLink = ref(false)
const power = usePower()

function openApp() {
  invoke('open_app')
}

const focused = useWindowFocus()

watch(focused, () => {
  invoke<boolean>('is_main_window_hidden').then((res) => {
    showOpenLink.value = res
  })
})
</script>

<template>
  <div class="relative max-w-max basis-1/3 flex gap-4">
    <Card class="flex-1">
      <CardHeader class="space-y-0 pb-2 gap-y-0">
        <CardTitle class="flex items-center justify-between gap-2 text-base truncate">
          <div class="mr-10 flex gap-2">
            {{ power.isCharging ? 'Charging Power' : 'System Power' }}
            <a
              v-if="showOpenLink"
              class="flex items-center gap-2 text-[10px] text-foreground/70 cursor-pointer"
              @click="openApp"
            >
              <ExternalLink class="h-4 w-4" />
            </a>
          </div>
          <div
            class="rounded-md dark:bg-green-600 bg-green-500 px-2 py-1 text-xs truncate text-background/80 font-mono"
          >
            <span class="font-bold mr-1">{{ power.adapterDetails.watts }}W</span>
            <span class="text-[10px]">({{ power.adapterDetails.voltage / 1000 }}V,{{
              power.adapterDetails.amperage / 1000 }}A)</span>
          </div>
        </CardTitle>
        <CardDescription class="text-[10px] font-mono">
          {{ power.adapterDetails.name }}
        </CardDescription>
      </CardHeader>
      <CardContent class="space-y-4 mt-1">
        <div class="flex flex-col gap-6">
          <div class="flex gap-2">
            <NumberFlow
              class="text-4xl font-bold"
              :format="{ maximumFractionDigits: 1, minimumFractionDigits: 1 }"
              :value="power.isCharging ? power.systemIn : power.systemPower" suffix="w"
            />
          </div>
        </div>

        <div class="flex flex-col gap-2">
          <div class="flex items-center justify-between">
            <div class="flex items-center gap-2">
              <BatteryCharging v-if="power.isCharging" class="h-5 w5 text-green-500" />
              <Battery v-else class="h-5 w-5" />
              <span class="text-xl font-bold mr-4">{{ power.batteryLevel.toFixed(2) }}%</span>
            </div>
            <div
              class="text-sm font-medium truncate"
              :class="power.isCharging ? 'text-green-500' : 'text-muted-foreground'"
            >
              <span class="font-semibold">{{ power.isCharging ? 'Charging' : 'On Battery' }}</span>
              • {{ power.timeRemaining }}
            </div>
          </div>
          <Progress :model-value="power.batteryLevel" />
        </div>
      </CardContent>
    </Card>
  </div>
</template>
