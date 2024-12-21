<script setup lang="ts">
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { usePower } from '@/composables/usePower'

import { Battery, Cpu, Laptop, CloudLightningIcon as Lightning, Monitor } from 'lucide-vue-next'

const power = usePower()
</script>

<template>
  <Card class="flex-1">
    <CardHeader>
      <CardTitle>Power Flow</CardTitle>
    </CardHeader>
    <CardContent>
      <div class="flex justify-between items-center w-full rounded-lg border bg-muted/50 p-4">
        <div class="max-h-min flex items-center gap-2 rounded-lg border bg-background p-2">
          <Lightning class="h-4 w-4 text-yellow-500" />
          <span class="text-sm font-medium">{{ (power.systemIn + power.powerLoss / 1000).toFixed(2) }}W</span>
        </div>

        <div class="w-full h-1 rounded-full bg-green-500 mx-2 animate-pulse" />

        <div class="flex flex-col items-center gap-2 bg-muted/50 rounded-lg border p-2">
          <div class="flex gap-4">
            <div class="flex items-center gap-2 rounded-lg border bg-background px-2 py-1.5">
              <Monitor class="h-4 w-4 text-blue-500" />
              <span class="text-sm font-medium">{{ (power.screenPower || 0).toFixed(2) }}W</span>
            </div>
            <div class="flex items-center gap-2 rounded-lg border bg-background px-2 py-1.5">
              <Cpu class="h-4 w-4 text-indigo-500" />
              <span class="text-sm font-medium">{{ (power.heatpipePower || 0).toFixed(2) }}W</span>
            </div>
          </div>
          <div class="flex items-center gap-2 rounded-lg border bg-background px-2 py-1.5">
            <Laptop class="h-4 w-4 text-cyan-500" />
            <span class="text-sm font-medium">{{ power.systemPower.toFixed(2) }}W</span>
          </div>
        </div>

        <div class="w-full h-1 rounded-full bg-green-500 mx-2 animate-pulse" />

        <div class="max-h-min flex items-center gap-2 rounded-lg border bg-background p-2">
          <Battery class="h-4 w-4 text-green-500" />
          <span class="text-sm font-medium">{{ (power.systemIn - power.systemPower).toFixed(2) }}W</span>
        </div>
      </div>
    </CardContent>
  </Card>
</template>
