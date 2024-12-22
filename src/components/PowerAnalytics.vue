<script setup lang="ts">
import PowerFlow from './PowerFlow.vue'
import PowerStatus from './PowerStatus.vue'
import PowerUsageChart from './PowerUsageChart.vue'
import TechnicalDetail from './TechnicalDetail.vue'
</script>

<template>
  <div class="flex flex-col gap-4 min-w-min pb-2">
    <div class="flex gap-4">
      <PowerStatus />
      <PowerFlow />
    </div>
    <div class="flex">
      <PowerUsageChart />
    </div>
    <div class="flex">
      <TechnicalDetail />
    </div>
  </div>
</template>
