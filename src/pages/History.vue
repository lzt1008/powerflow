<script setup lang="ts">
import { type ChargingHistory, events } from '@/bindings'
import { Info } from 'lucide-vue-next'

const { isLoading, state, execute } = usePowerHistory()
const selectedItem = ref({} as ChargingHistory)

onMounted(() => {
  const unlisten = events.historyRecordedEvent.listen(() => execute())

  onScopeDispose(() => unlisten.then(f => f()))
})
</script>

<template>
  <div
    v-if="!isLoading && state.status === 'ok' && state.data.length === 0"
    class="w-full h-full flex flex-col gap-2 items-center justify-center text-muted-foreground"
  >
    <Info class="w-6 h-6" />
    <span class="mb-16">No history recorded yet, charge your device to get started</span>
  </div>
  <div v-else class="flex h-[calc(100vh-80px)]">
    <div class="flex flex-col gap-4 pl-4 ">
      <h2 class="font-bold text-lg">
        History
      </h2>
      <div v-if="!isLoading && state.status === 'ok'" class="flex flex-col gap-4 h-full overflow-y-auto pr-4">
        <HistoryListItem
          v-for="item in state.data"
          :key="item.id"
          v-bind="item"
          class="cursor-pointer transition-colors"
          :class="{ 'bg-muted': selectedItem.id === item.id }"
          @click="selectedItem = selectedItem.id === item.id ? {} : item"
        />
        <div class="my-4 font-mono" />
      </div>
    </div>
    <Separator orientation="vertical" class="h-auto" />
    <div class="relative grow">
      <HistoryDetail
        v-if="selectedItem.id"
        v-bind="selectedItem"
        class="h-full"
      />
      <div v-else class="overflow-y-auto h-full flex flex-col items-center justify-center">
        <Info class="size-6" />
        <p class="text-muted-foreground font-medium text-sm mb-10">
          Select a charging session to view details
        </p>
      </div>
    </div>
  </div>
</template>
