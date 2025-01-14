<script setup lang="ts">
import { formatUpdateTime } from '@/lib/format'

const power = usePower()
const rawData = usePowerRaw()
</script>

<template>
  <div v-if="!power.isLoading">
    {{ power.isCharging ? power.adapterName : $t('status.on_battery') }}
    <template v-if="!rawData.isLocal && rawData.offline">
      <span>·</span>
      {{ $t('status.offline') }}
    </template>
    <template v-else-if="!rawData.isLocal">
      <span>·</span>
      {{ power.isRemote ? formatUpdateTime(power.lastUpdate * 1000) : '' }}
    </template>
  </div>
  <Skeleton v-else class="w-20 h-[10px]" />
</template>
