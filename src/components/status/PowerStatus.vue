<script setup lang="ts">
import { Card } from '@/components/ui/card'

const isPopover = inject('isPopover', false)
const div = h('div')
</script>

<template>
  <Component
    :is="isPopover ? div : Card"
    :class="{ 'flex-1 bg-transparent border-none shadow-none': isPopover }"
  >
    <CardHeader class="space-y-0 pb-2 gap-y-0">
      <CardTitle class="flex items-center justify-between gap-2 text-base truncate">
        <PowerStatusTitle />
      </CardTitle>
      <CardDescription class="text-[10px] font-mono flex gap-[2px] items-center">
        <PowerStatusDescription />
      </CardDescription>
    </CardHeader>
    <CardContent class="space-y-3 mt-1">
      <PowerStatusNumber />
      <PowerStatusFooter />
    </CardContent>
  </Component>
</template>
