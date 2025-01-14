<script setup lang="ts">
import { commands } from '@/bindings'

const target = useTemplateRef<HTMLElement>('target')

provide('isPopover', true)
</script>

<template>
  <div ref="target" class="w-full" @click="commands.openApp()">
    <slot is-popover />
  </div>
</template>
