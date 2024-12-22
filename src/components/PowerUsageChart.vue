<script setup lang="ts">
import type { StatisticData } from '@/composables'
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { LineChart } from '@/components/ui/chart-line'
import { usePower } from '@/composables'
import { computed } from 'vue'
import ChartTooltip from './ChartTooltip.vue'

const power = usePower()

const categories = computed(() => {
  const base = ['System Power', 'Screen Power', 'Heatpipe Power'] as (keyof StatisticData)[]
  if (power.value.isCharging) {
    base.push('System In')
  }
  return base
})
</script>

<template>
  <Card class="w-full space-y-4">
    <CardHeader class="pb-0">
      <CardTitle>
        Power Usage
      </CardTitle>
    </CardHeader>
    <CardContent>
      <LineChart
        class="w-full h-[240px]"
        index="time"
        :y-formatter="(value) => `${value}W`"
        :data="power.statistics?.length < 2 ? [] : (power?.statistics || [])"
        :categories
        :custom-tooltip="ChartTooltip"
      />
    </CardContent>
  </Card>
</template>
