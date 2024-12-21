<script setup lang="ts">
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { LineChart } from '@/components/ui/chart-line'
import { usePower } from '@/composables'

const power = usePower()
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
        class="w-full h-[240px]" :data="power.statistics.length < 2 ? [] : power.statistics" index="time"
        :categories="['System Power', 'System In', 'Screen Power', 'Heatpipe Power']"
        :y-formatter="(value) => `${value}W`"
      />
    </CardContent>
  </Card>
</template>
