<script setup lang="ts">
import type { ChargingHistory } from '@/bindings'
import { shortEnDistanceLocale } from '@/lib/format'
import { MobileIcon } from '@radix-icons/vue'
import { formatDistanceToNow, formatDuration, intervalToDuration } from 'date-fns'
import { ChevronRight, LaptopIcon } from 'lucide-vue-next'

defineProps<ChargingHistory>()
</script>

<template>
  <div class="flex items-center justify-between rounded-xl border px-4 py-2">
    <div class="flex items-center">
      <div class="py-2 h-min">
        <MobileIcon v-if="isRemote" class="size-5" />
        <LaptopIcon v-else class="size-5" />
      </div>
      <div class="flex flex-col ml-4">
        <span class="flex items-baseline gap-2 text-lg font-bold relative">
          <span class="flex items-center gap-1">{{ fromLevel }}% <ChevronRight class="size-4" /> {{ endLevel }}%</span>
          <div class="text-muted-foreground font-mono text-sm">
            {{ formatDuration(
              intervalToDuration({
                start: timestamp * 1000,
                end: timestamp * 1000 + chargingTime * 1000,
              })
              , {
                locale: shortEnDistanceLocale,
                format: ['hours', 'minutes'],
              }) }}
          </div>
        </span>

        <span class="text-muted-foreground font-mono text-xs">{{
          formatDistanceToNow(new Date(timestamp * 1000 + chargingTime * 1000), { addSuffix: true })
        }}</span>
      </div>
    </div>

    <div class="w-24" />

    <div class="flex items-center gap-4">
      <div class="hover:bg-muted transition-colors rounded-full p-2">
        <ChevronRight class="size-4" />
      </div>
    </div>
  </div>
</template>
