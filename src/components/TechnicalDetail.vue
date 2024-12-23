<script setup lang="ts">
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card'
import { usePower } from '@/composables'
import { Battery, CloudLightning, Cpu, Thermometer } from 'lucide-vue-next'
import Skeleton from './ui/skeleton/Skeleton.vue'

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
        <div v-if="!power.isLoading" class="text-2xl font-bold">
          {{ power.smc.temperature.toFixed(1) }}°C
        </div>
        <Skeleton v-else class="w-12 h-8" />
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
        <div v-if="!power.isLoading" class="text-2xl font-bold">
          {{ (power.io.appleRawMaxCapacity / power.io.designCapacity * 100).toFixed(1) }}%
        </div>
        <Skeleton v-else class="w-12 h-8" />
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
        <div v-if="!power.isLoading" class="text-2xl font-bold">
          {{ power.io.cycleCount }}
        </div>
        <Skeleton v-else class="w-12 h-8" />
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
        <div v-if="!power.isLoading" class="text-2xl font-bold">
          {{ power.io.appleRawCurrentCapacity }}mAh
        </div>
        <Skeleton v-else class="w-12 h-8" />
        <p class="flex gap-2 text-xs text-muted-foreground">
          Design Capacity: <span v-if="!power.isLoading">{{ power.io.designCapacity }}mAh</span>
          <Skeleton v-else class="w-12 h-4" />
        </p>
      </CardContent>
    </Card>
  </div>
</template>
