<script setup lang="ts">
import { formatChargingDuration } from '@/lib/format'
import { BatteryCharging, BatteryFull, BatteryLow, BatteryMedium } from 'lucide-vue-next'
import { useI18n } from 'vue-i18n'

const power = usePower()
const { t } = useI18n()
</script>

<template>
  <div class="flex flex-col gap-2">
    <div class="flex items-center justify-between">
      <div v-if="!power.isLoading" class="flex items-center gap-2">
        <BatteryCharging v-if="power.isCharging" class="size-5 text-green-500" />
        <BatteryFull v-else-if="power.batteryLevel > 66" class="size-5" />
        <BatteryMedium v-else-if="power.batteryLevel > 33" class="size-5" />
        <BatteryLow
          v-else
          class="size-5"
          :class="{
            'text-red-500': power.batteryLevel < 10,
          }"
        />
        <span class="text-xl font-bold mr-4">
          {{ power.batteryLevel.toFixed(2) }}%</span>
      </div>
      <Skeleton v-else class="w-24 h-7" />

      <div
        v-if="!power.isLoading"
        class="text-sm font-medium truncate"
        :class="power.isCharging ? 'text-green-500' : 'text-muted-foreground'"
      >
        <span v-if="power.isCharging && power.batteryLevel === 100">{{ $t('status.fully_charged') }}</span>
        <template v-else>
          <span class="font-semibold mr-1">{{ formatChargingDuration(power.timeRemain.secs, t) }}</span>
          <span>{{ power.isCharging ? $t('status.to_full') : $t('status.to_empty') }}</span>
        </template>
      </div>
      <Skeleton v-else class="w-32 h-5" />
    </div>
    <PowerStatusBar v-if="!power.isLoading" />
    <Skeleton v-else class="w-full" />
  </div>
</template>
