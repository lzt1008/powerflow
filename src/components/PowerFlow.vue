<script setup lang="ts">
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { Skeleton } from '@/components/ui/skeleton'
import { usePower } from '@/composables/usePower'

import { Battery, Cpu, Laptop, CloudLightningIcon as Lightning, Monitor } from 'lucide-vue-next'
import Shimmer from './Shimmer.vue'
import Tooltip from './Tooltip.vue'

const formatter = new Intl.NumberFormat('en-US', {
  // maximumSignificantDigits: 2,
  // minimumSignificantDigits: 2,
  maximumFractionDigits: 1,
  minimumFractionDigits: 1,
})
const power = usePower()
</script>

<template>
  <Card class="flex-1">
    <CardHeader>
      <CardTitle>Power Flow</CardTitle>
    </CardHeader>
    <CardContent>
      <Skeleton v-if="power.isLoading" class="w-full h-[120px]" />
      <div
        v-else
        class="flex justify-between items-center w-full rounded-lg border bg-muted/50 p-4 font-mono text-secondary-foreground text-xs"
        :class="[power.isCharging ? '' : 'flex-row-reverse']"
      >
        <Tooltip v-if="power.isCharging" content="Adapter power (loss 700mw)" as-child>
          <div class="w-24 shrink-0 flex justify-center items-center gap-2 rounded-lg border bg-background px-2 py-1.5">
            <Lightning class="h-4 w-4 text-yellow-500" />
            <span class="text-xs font-medium">{{ formatter.format(power.systemIn + power.powerLoss / 1000) }}w</span>
          </div>
        </Tooltip>

        <Shimmer
          v-if="power.isCharging" :repeat-delay="1500"
          class="rounded-full mx-2 w-full
          [--base-color:theme(colors.green.500)]
          [--base-gradient-color:theme(colors.green.300)]
          dark:[--base-color:theme(colors.green.700)]
          dark:[--base-gradient-color:theme(colors.green.400)]"
        >
          <div class="h-1" />
        </Shimmer>

        <div class="flex flex-col items-center gap-2 bg-muted/50 rounded-lg border p-2">
          <div class="flex gap-4">
            <Tooltip content="Power of the screen" as-child>
              <div class="w-24 shrink-0 flex justify-center items-center gap-2 rounded-lg border bg-background px-2 py-1.5">
                <Monitor class="h-4 w-4 text-blue-500" />
                <span class="text-xs font-medium">{{ formatter.format(power.screenPower || 0) }}w</span>
              </div>
            </Tooltip>

            <Tooltip content="Power of heatpipe" as-child>
              <div class="w-24 shrink-0 flex justify-center items-center gap-2 rounded-lg border bg-background px-2 py-1.5">
                <Cpu class="h-4 w-4 text-indigo-500" />
                <span class="text-xs font-medium">{{ formatter.format(power.heatpipePower || 0) }}w</span>
              </div>
            </Tooltip>
          </div>

          <Tooltip content="System total power" as-child>
            <div class="w-24 shrink-0 flex justify-center items-center gap-2 rounded-lg border bg-background px-2 py-1.5">
              <Laptop class="h-4 w-4 text-cyan-500" />
              <span class="text-xs font-medium">{{ formatter.format(power.systemPower) }}w</span>
            </div>
          </Tooltip>
        </div>

        <Shimmer
          :delay="2000" :repeat-delay="1500"
          class="rounded-full mx-2 w-full [--base-color:theme(colors.green.500)] [--base-gradient-color:theme(colors.green.300)] dark:[--base-color:theme(colors.green.700)] dark:[--base-gradient-color:theme(colors.green.400)]"
        >
          <div class="h-1" />
        </Shimmer>

        <Tooltip content="Battery power" as-child>
          <div class="w-24 shrink-0 flex justify-center items-center gap-2 rounded-lg border bg-background p-2 py-1.5">
            <Battery class="h-4 w-4 text-green-500" />
            <span class="text-xs font-medium">{{ formatter.format(power.isCharging ? power.systemIn - power.systemPower : power.batteryPower) }}w</span>
          </div>
        </Tooltip>
      </div>
    </CardContent>
  </Card>
</template>
