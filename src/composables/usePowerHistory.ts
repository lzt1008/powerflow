import { commands } from '@/bindings'

export function usePowerHistory() {
  return useAsyncState(commands.getAllChargingHistory(), { status: 'ok', data: [] })
}
