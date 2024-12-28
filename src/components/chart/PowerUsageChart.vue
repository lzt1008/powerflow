<script setup lang="ts">
import CustomChartTooltip from './CustomChartTooltip.vue'

const power = usePower()
const categories = computed(() => {
  const base = ['System Power'] as (keyof StatisticData)[]
  if (!power.value.isRemote) {
    base.push('Screen Power', 'Heatpipe Power')
  }
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
        {{ $t('power_usage') }}
      </CardTitle>
    </CardHeader>
    <CardContent>
      <Skeleton v-if="power.isLoading" class="w-full h-[240px]" />
      <LineChart
        v-else
        class="w-full h-[240px]"
        index="time"
        :y-formatter="(value) => `${value}W`"
        :data="power?.statistics"
        :categories
        :custom-tooltip="CustomChartTooltip"
      />
    </CardContent>
  </Card>
</template>
