<script setup lang="ts">
import { localeMap } from '@/lib/format'
import { addMinutes, formatDistanceToNow } from 'date-fns'
import { BatteryCharging, BatteryFull, BatteryLow, BatteryMedium } from 'lucide-vue-next'

const power = usePower()
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
        <span class="font-semibold">{{
          formatDistanceToNow(
            addMinutes(new Date(), power.timeRemaining),
            { locale: localeMap[$i18n.locale as 'en' | 'zh-CN'] },
          )
        }}</span>
        <span>{{ power.isCharging ? $t('status.to_full') : $t('status.to_empty') }}</span>
      </div>
      <Skeleton v-else class="w-32 h-5" />
    </div>
    <Progress v-if="!power.isLoading" :model-value="isNaN(power?.batteryLevel) ? 0 : power?.batteryLevel" />
    <Skeleton v-else class="w-full h-2" />
  </div>
</template>
