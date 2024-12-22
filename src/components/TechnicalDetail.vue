<script setup lang="ts">
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { usePower } from '@/composables';
import { Battery, CloudLightning, Cpu, Thermometer } from 'lucide-vue-next'

const power = usePower()
</script>

<template>
  <div class="flex-1 grid gap-4 grid-cols-4">
    <Card>
      <CardHeader class="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle class="text-sm font-medium">
          Temperature
        </CardTitle>
        <Thermometer class="h-4 w-4 text-muted-foreground" />
      </CardHeader>
      <CardContent>
        <div class="text-2xl font-bold">
          {{ power.smc.temperature.toFixed(1) }}°C
        </div>
        <p class="text-xs text-muted-foreground">
          Current battery temperature
        </p>
      </CardContent>
    </Card>
    <Card>
      <CardHeader class="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle class="text-sm font-medium">
          Battery Health
        </CardTitle>
        <Battery class="h-4 w-4 text-muted-foreground" />
      </CardHeader>
      <CardContent>
        <div class="text-2xl font-bold">
          {{ (power.io.appleRawMaxCapacity / power.io.designCapacity * 100).toFixed(1) }}%
        </div>
        <p class="text-xs text-muted-foreground">
          Maximum capacity relative to new
        </p>
      </CardContent>
    </Card>
    <Card>
      <CardHeader class="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle class="text-sm font-medium">
          Cycle Count
        </CardTitle>
        <Cpu class="h-4 w-4 text-muted-foreground" />
      </CardHeader>
      <CardContent>
        <div class="text-2xl font-bold">
          {{ power.io.cycleCount }}
        </div>
        <p class="text-xs text-muted-foreground">
          Total battery cycles
        </p>
      </CardContent>
    </Card>
    <Card>
      <CardHeader class="flex flex-row items-center justify-between space-y-0 pb-2">
        <CardTitle class="text-sm font-medium">
          Energy
        </CardTitle>
        <CloudLightning class="h-4 w-4 text-muted-foreground" />
      </CardHeader>
      <CardContent>
        <div class="text-2xl font-bold">
          {{ power.io.appleRawCurrentCapacity }}mAh
        </div>
        <p class="text-xs text-muted-foreground">
          Design Capacity: {{ power.io.designCapacity }}mAh
        </p>
      </CardContent>
    </Card>
  </div>
</template>
