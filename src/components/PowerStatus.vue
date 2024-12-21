<script setup lang="ts">
import Badge from '@/components/ui/badge/Badge.vue'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import Progress from '@/components/ui/progress/Progress.vue'
import { usePower } from '@/composables'
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
      <CardHeader class="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle class="text-base font-medium truncate mr-6">
          Power Status
        </CardTitle>
        <div
          class="flex items-center gap-2 rounded-md dark:bg-green-600 bg-green-500 px-2 py-1 text-xs truncate text-foreground/80"
        >
          {{ power.adapterDetails.name }}
        </div>
      </CardHeader>
      <CardContent class="space-y-4 mt-1">
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
              {{ power.isCharging ? 'Charging' : 'On Battery' }} • {{ power.timeRemaining }}
            </div>
          </div>
          <Progress :model-value="power.batteryLevel" />
        </div>

        <div class="flex flex-col gap-6">
          <div class="flex flex-col gap-2">
            <div class="flex gap-2 text-sm font-medium text-muted-foreground">
              <div>{{ power.isCharging ? `Charging Power` : 'System Power' }}</div>
              <Badge v-if="power.isCharging" variant="secondary" class="font-normal -translate-y-[1px]">
                {{ power.adapterDetails.watts }}W ({{ power.adapterDetails.voltage / 1000 }}V {{
                  power.adapterDetails.amperage / 1000 }}A)
              </Badge>
            </div>
            <div class="text-4xl font-bold transition-all">
              {{ (power.isCharging ? power.systemIn : power.systemPower).toFixed(1) }}W
            </div>
          </div>
        </div>
      </CardContent>
    </Card>
    <a
      v-if="showOpenLink"
      class="absolute flex items-center gap-2 text-xs text-foreground/70 bottom-4 right-4 cursor-pointer"
      @click="openApp"
    >
      <ExternalLink class="h-4 w-4" />
      Open App
    </a>
  </div>
</template>
