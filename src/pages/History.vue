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
    class="w-full h-full flex items-center justify-center text-muted-foreground"
  >
    No history recorded yet, charge your device to get started
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
        <Separator class="mx-auto w-3/4 my-8 font-mono" label="End" />
      </div>
    </div>
    <Separator orientation="vertical" class="h-auto" />
    <div class="relative grow">
      <HistoryDetail
        v-if="selectedItem.id"
        v-bind="selectedItem"
        class="overflow-y-auto"
      />
      <div v-else class="overflow-y-auto flex flex-col items-center justify-center">
        <Info class="size-6" />
        <p class="text-muted-foreground font-medium text-sm">
          Select a charging session to view details
        </p>
      </div>
    </div>
  </div>
</template>
