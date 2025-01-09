<script setup lang="ts">
defineProps<{
  title?: string
  data: {
    name: string
    color: string
    value: any
  }[]
}>()

const formatter = new Intl.NumberFormat('en-US', {
  maximumSignificantDigits: 3,
  minimumSignificantDigits: 3,
})
</script>

<template>
  <Card class="text-sm">
    <CardContent class="p-3 min-w-[100px] flex flex-col gap-2">
      <div v-if="title" class="font-bold">
        {{ title }}
      </div>
      <div v-for="(item, key) in data.filter(i => i.name)" :key="key" class="flex justify-between">
        <div class="flex items-center">
          <span class="size-2.5 mr-2">
            <svg width="100%" height="100%" viewBox="0 0 30 30">
              <path
                d=" M 15 15 m -14, 0 a 14,14 0 1,1 28,0 a 14,14 0 1,1 -28,0"
                :stroke="item.color"
                :fill="item.color"
                stroke-width="1"
              />
            </svg>
          </span>
          <span class="text-xs text-muted-foreground">{{ item.name }}</span>
        </div>
        <span class="text-xs font-semibold ml-4 font-mono">{{
          typeof item.value === 'number' ? formatter.format(item.value) : item.value
        }}{{ item.name.toLowerCase().includes('level') ? '%' : 'w' }}</span>
      </div>
    </CardContent>
  </Card>
</template>
