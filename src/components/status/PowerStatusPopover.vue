<script setup lang="ts">
import { commands, events } from '@/bindings'
import { getCurrentWindow } from '@tauri-apps/api/window'
import { useMotion } from '@vueuse/motion'

const target = useTemplateRef<HTMLElement>('target')

const motion = useMotion(target, {
  initial: {
    opacity: 0,
    scale: 0.99,
    transition: {
      duration: 200,
      type: 'spring',
    },
  },
  visible: {
    opacity: 1,
    scale: 1,
    transition: {
      duration: 300,
      type: 'spring',
    },
  },
})

let unlisten: ReturnType<typeof events.hidePopoverEvent.listen>

onMounted(() => {
  unlisten = events.hidePopoverEvent.listen(() => {
    motion?.apply('initial')

    setTimeout(() => {
      getCurrentWindow().hide()
    }, 300)
  })
})

onUnmounted(() => {
  unlisten.then(f => f())
})

const focused = useWindowFocus()
watch(focused, () => {
  motion?.apply(!focused.value ? 'initial' : 'visible')
})
</script>

<template>
  <div ref="target" @click="commands.openApp()">
    <slot />
  </div>
</template>
