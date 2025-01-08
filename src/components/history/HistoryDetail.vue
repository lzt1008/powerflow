<script setup lang="ts">
import type { ChargingHistory, ChargingHistoryDetail } from '@/bindings'
import { commands } from '@/bindings'
import CustomChartTooltip from '@/components/chart/CustomChartTooltip.vue'
import { useHistory } from '@/composables/useHistory'
import { shortEnDistanceLocale } from '@/lib/format'
import { error as logerror } from '@tauri-apps/plugin-log'
import { format, formatDuration, intervalToDuration } from 'date-fns'
import { Download, EllipsisVertical, Loader2, Trash2 } from 'lucide-vue-next'

const props = defineProps<ChargingHistory>()
const { selectedItem, history } = useHistory()

const isLoading = ref(true)
const error = ref()
const data = asyncComputed(
  () => commands.getDetailById(props.id)
    .then((r) => {
      if (r.status === 'error') {
        error.value = r.error
        logerror(r.error)
        return {} as ChargingHistoryDetail
      }
      return r.data
    }),
  {} as ChargingHistoryDetail,
  isLoading,
)
</script>

<template>
  <div class="h-full overflow-y-auto">
    <div v-if="isLoading" class="w-full h-full flex items-center justify-center">
      <Loader2 class="animate-spin" />
    </div>
    <div v-else-if="error" class="w-full h-full flex items-center justify-center text-red-500">
      {{ error }}
    </div>
    <div v-else class="px-6 pb-8">
      <div class="flex justify-between items-center">
        <div>
          <h1 class="text-2xl font-bold">
            {{ name || 'Unknown' }}
          </h1>
          <h2 class="text-sm font-bold mt-1 text-muted-foreground">
            with {{ adapterName }}
          </h2>
        </div>
        <div>
          <DropdownMenu>
            <DropdownMenuTrigger class="p-2 rounded-md hover:bg-muted transition-colors">
              <EllipsisVertical class="w-4 h-4" />
            </DropdownMenuTrigger>
            <DropdownMenuContent
              :side-offset="10"
              align="end"
            >
              <DropdownMenuItem>
                <Download class="w-4 h-4" />
                Export Data
              </DropdownMenuItem>
              <DropdownMenuSeparator />
              <DropdownMenuItem
                class="text-red-500 focus:text-red-500 focus:bg-red-500/10"
                @click="() => {
                  commands.deleteHistoryById(id)
                  selectedItem = null
                  history.update()
                }"
              >
                <Trash2 />
                Delete
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>
        </div>
      </div>
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
          <div>
            <div class="text-muted-foreground">
              Adapter Watts
            </div>
            <div>{{ data.peak.adapterWatts }}W({{ data.peak.adapterVoltage }}V, {{ data.peak.adapterAmperage }}A)</div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>
