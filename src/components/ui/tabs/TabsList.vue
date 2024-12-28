<script setup lang="ts">
import type { HTMLAttributes } from 'vue'
import { cn } from '@/lib/utils'
import { TabsList, type TabsListProps } from 'radix-vue'

const props = defineProps<TabsListProps & { class?: HTMLAttributes['class'] }>()
const delegatedProps = computed(() => {
  const { class: _, ...delegated } = props

  return delegated
})

const t = useTemplateRef('t')

const transitionClass = ref<string[]>([])
const style = ref({
  left: '',
  top: '',
  width: '',
  height: '',
})

function updateIndicator() {
  const activeTab
  = Array.from(t.value?.$el.children as NodeListOf<HTMLDivElement> || [])
    .find(el => el.dataset.state === 'active') || t.value?.$el.children[0]

  if (activeTab) {
    if (style.value.left && transitionClass.value.length === 0) {
      transitionClass.value.push('transition-[left] duration-300 ease-in-out')
    }
    const activeRect = activeTab.getBoundingClientRect()
    style.value = {
      left: `${activeRect.left}px`,
      top: `${activeRect.top}px`,
      width: `${activeRect.width}px`,
      height: `${activeRect.height}px`,
    }
  }
}

onMounted(() => {
  const observer = new MutationObserver(async (mutations) => {
    for (const m of mutations) {
      if (m.type === 'attributes' && m.attributeName === 'data-state') {
        updateIndicator()
      }
    }
  })

  updateIndicator()

  observer.observe(t.value?.$el as HTMLDivElement, {
    childList: true,
    subtree: true,
    attributes: true,
  })

  return () => observer.disconnect()
})
</script>

<template>
  <TabsList
    ref="t"
    v-bind="delegatedProps"
    :class="cn(
      'inline-flex items-center justify-center rounded-lg bg-muted p-1 text-muted-foreground ',
      props.class,
    )"
  >
    <slot />
  </TabsList>
  <div
    class="absolute rounded-md bg-background shadow-sm"
    :class="transitionClass"
    :style
  />
</template>
