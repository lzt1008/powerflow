<script setup lang="ts">
import NumberFlow from '@number-flow/vue'

const power = usePower()
const { preference, isLoading } = usePreferenceAsync()
</script>

<template>
  <div>
    <Skeleton v-if="power.isLoading || isLoading" class="w-40 h-[50px] mt-2" />
    <NumberFlow
      v-else-if="preference.animationsEnabled"
      class="text-4xl font-bold"
      :format="{ maximumFractionDigits: 1, minimumFractionDigits: 1 }"
      :value="power.isCharging ? power.systemIn : power.systemLoad"
      suffix="w"
    />
    <div v-else class="text-4xl leading-[54px] font-bold">
      {{ (power.isCharging ? power.systemIn : power.systemLoad).toFixed(1) }}w
    </div>
  </div>
</template>
