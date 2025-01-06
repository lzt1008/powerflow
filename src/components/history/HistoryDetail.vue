<script setup lang="ts">
import type { ChargingHistory, ChargingHistoryDetail } from '@/bindings'
import { commands } from '@/bindings'
import CustomChartTooltip from '@/components/chart/CustomChartTooltip.vue'
import { shortEnDistanceLocale } from '@/lib/format'
import { error } from '@tauri-apps/plugin-log'
import { format, formatDuration, intervalToDuration } from 'date-fns'
import { Loader2 } from 'lucide-vue-next'

const props = defineProps<ChargingHistory>()

const isLoading = ref(false)
const state = asyncComputed(
  () => commands.getDetailById(props.id),
  { status: 'ok', data: {} as ChargingHistoryDetail },
  isLoading,
)

const data = computed(() => {
  if (state.value.status === 'ok') {
    return state.value.data
  }
  error(state.value.error)
  return {} as ChargingHistoryDetail
})
</script>

<template>
  <div v-if="isLoading">
    <div class="w-full h-full flex items-center justify-center">
      <Loader2 class="animate-spin" />
    </div>
  </div>
  <div v-else-if="state.status === 'error'" class="w-full h-full flex items-center justify-center text-red-500">
    {{ state.error }}
  </div>
  <div v-else class="px-6">
    <h1 class="text-2xl font-bold">
      {{ name || 'Unknown' }}
    </h1>
    <h2 class="text-sm font-bold mt-1 text-muted-foreground">
      {{ udid || 'Unknown' }}
    </h2>
    <div class="mt-4 grid gap-4 grid-cols-3">
      <div class="space-y-2">
        <div class="text-sm font-medium text-muted-foreground">
          Duration
        </div>
        <div class="text-2xl font-bold">
          {{ formatDuration(
            intervalToDuration({
              start: timestamp * 1000,
              end: timestamp * 1000 + chargingTime * 1000,
            }),
            { format: ['hours', 'minutes'], locale: shortEnDistanceLocale },
          )
          }}
        </div>
        <div class="text-xs text-muted-foreground">
          {{ format(timestamp * 1000, 'yyyy-MM-dd HH:mm') }}
        </div>
      </div>
      <div class="space-y-2">
        <div class="text-sm font-medium text-muted-foreground">
          Avg Power
        </div>
        <div class="text-2xl font-bold">
          {{ data.avg.adapterPower.toFixed(1) }}W
        </div>
        <div class="text-xs text-muted-foreground">
          Peak: {{ data.peak.adapterPower.toFixed(1) }}W
        </div>
      </div>
      <div class="space-y-2">
        <div class="text-sm font-medium text-muted-foreground">
          Charging rate
        </div>
        <div class="text-2xl font-bold">
          {{ ((endLevel - fromLevel) / chargingTime * 60).toFixed(2) }}%/min
        </div>
        <div class="text-xs text-muted-foreground">
          Avg Temp: {{ data.avg.temperature.toFixed(1) }}°C
        </div>
      </div>
    </div>

    <h2 class="mt-8 font-bold">
      Charging Curve
    </h2>
    <LineChart
      class="mt-8 max-h-[220px]"
      index="lastUpdate"
      :data="data.curve"
      :categories="['systemIn', 'batteryPower', 'systemLoad', 'batteryLevel']"
      :custom-tooltip="CustomChartTooltip"
      :show-legend="false"
    />

    <h2 class="mt-8 font-bold">
      Additional Detail
    </h2>
    <div class="mt-2 grid gap-4 text-sm">
      <div class="grid grid-cols-2 gap-4">
        <div>
          <div class="text-muted-foreground">
            Temperature Peak
          </div>
          <div>{{ data.peak.temperature.toFixed(1) }}°C</div>
        </div>
        <div>
          <div class="text-muted-foreground">
            Adapter Power Peak
          </div>
          <div>{{ data.peak.adapterPower.toFixed(1) }}W</div>
        </div>
      </div>
    </div>
  </div>
</template>
