<script setup lang="ts">
import { addSeconds, format } from 'date-fns'
import { ArrowUpDown } from 'lucide-vue-next'

const power = usePower()
const rawData = usePowerRaw()

const showRemainDuration = ref(true)
const buttonText = computed(() => {
  if (showRemainDuration.value) {
    const minutes = power.value.timeRemain.secs / 60
    const hours = Math.floor(minutes / 60)

    return `${hours}h ${minutes % 60}m`
  }
  return format(
    addSeconds(new Date(), power.value.timeRemain.secs),
    'HH:mm',
  )
})
</script>

<template>
  <div class="mr-10 flex gap-2 items-center">
    {{ power.isCharging ? $t('status.charging_power') : $t('status.system_power') }}
    <span
      v-if="power.isRemote"
      class="mr-1 size-2 rounded-full"
      :class="{
        'bg-blue-500 animate-pulse': !rawData.isLocal && !rawData.offline,
        'bg-neutral-500': !rawData.isLocal && rawData.offline,
      }"
    />
  </div>

  <Skeleton v-if="power.isLoading" class="w-24 h-6" />
  <div
    v-else-if="power.isCharging"
    class="rounded-md
    bg-gradient-to-r from-blue-500 to-blue-600
    px-2 py-1 text-xs truncate font-mono"
  >
    <span class="font-bold mr-1 text-background">{{ power.adapterWatts }}W</span>
    <span class="text-[10px] text-background/80">({{ power.adapterVoltage }}V,{{
      power.adapterAmperage }}A)</span>
  </div>
  <div
    v-else
    class="rounded-md dark:bg-blue-600 bg-blue-00 px-2 py-1 text-xs truncate font-mono w-20 text-background flex items-center justify-center
            cursor-pointer hover:bg-blue-600 transition-colors
            "
    @click.stop="showRemainDuration = !showRemainDuration"
  >
    <span class="font-bold mr-1">{{ buttonText }}</span>
    <ArrowUpDown
      class="size-3 text-background/80 transition-transform duration-300"
      :class="{ 'rotate-180': showRemainDuration }"
    />
  </div>
</template>
