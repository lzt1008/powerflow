<script setup lang="ts">
import type { TooltipContentProps, TooltipProviderProps } from 'radix-vue'
import {
  Tooltip,
  TooltipContent,
  TooltipProvider,
  TooltipTrigger,
} from '@/components/ui/tooltip'

interface TooltipProps {
  content?: string
  side?: TooltipContentProps['side']
  align?: TooltipContentProps['align']
  delayDuration?: TooltipProviderProps['delayDuration']
  as?: TooltipContentProps['as']
  asChild?: TooltipContentProps['asChild']
}

defineProps<TooltipProps>()
</script>

<template>
  <TooltipProvider :delay-duration>
    <Tooltip>
      <TooltipTrigger :as :as-child>
        <slot />
      </TooltipTrigger>
      <TooltipContent :side :align>
        <slot name="popper">
          {{ content }}
        </slot>
      </TooltipContent>
    </Tooltip>
  </TooltipProvider>
</template>
