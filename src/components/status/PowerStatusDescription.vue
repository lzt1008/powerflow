<script setup lang="ts">
import { formatUpdateTime } from '@/lib/format'

const power = usePower()
const rawData = usePowerRaw()
</script>

<template>
  <template v-if="!power.isLoading">
    {{ power.isCharging ? power.adapterDetails!.name : 'On Battery' }}
    <template v-if="!rawData.isLocal && rawData.offline">
      <span>·</span>
      offline
    </template>
    <template v-else-if="!rawData.isLocal">
      <span>·</span>
      {{ power.isRemote ? formatUpdateTime(power.io.updateTime * 1000) : '' }}
    </template>
  </template>
  <Skeleton v-else class="w-20 h-[10px]" />
</template>
