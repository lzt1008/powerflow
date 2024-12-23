import type { IORegistry, SMCPowerData } from '@/bindings'
import type { Reactive } from 'vue'
import { events } from '@/bindings'
import { useDocumentVisibility } from '@vueuse/core'
import { computed, reactive } from 'vue'

interface RawPowerData {
  smc: SMCPowerData
  io: IORegistry
  statistics: StatisticData[]
}

export interface StatisticData {
  'time': string
  'System Power': number
  'System In': number
  'Battery Level': number
  'Screen Power': number
  'Heatpipe Power': number
}

const powerData: Reactive<RawPowerData> = reactive({
  smc: {} as SMCPowerData,
  io: {} as IORegistry,
  statistics: [],
} as RawPowerData)

let count = 0

events.powerTickEvent.listen(async ({ payload }) => {
  powerData.smc = payload.smc
  powerData.io = payload.io

  count++
  if (count === 3) {
    count = 0
    if (powerData.statistics?.length > 20) {
      powerData.statistics.shift()
    }
    const level = powerData.io.appleRawCurrentCapacity
      / powerData.io.appleRawMaxCapacity * 100
    const power = powerData.smc.systemTotal
    powerData.statistics.push({
      'time': new Date().toLocaleTimeString(),
      'System Power': power - power % 0.01,
      'System In': powerData.smc.deliveryRate < 0.01
        ? 0
        : powerData.smc.deliveryRate,
      'Battery Level': level - level % 0.01,
      'Screen Power': powerData.smc.brightness || 0,
      'Heatpipe Power': powerData.smc.heatpipe || 0,
    })
  }
})

const vis = useDocumentVisibility()

export function usePower() {
  return computed(() => ({
    isLoading: Object.keys(powerData.smc).length === 0 || vis.value === 'hidden',
    isReady: Object.keys(powerData.smc).length > 0 && vis.value === 'visible',
    statistics: powerData.statistics,
    io: powerData.io,
    smc: powerData.smc,
    isCharging: powerData.smc.chargingStatus === 1,
    timeRemaining: (() => {
      if (powerData.io.fullyCharged) {
        return 'Full'
      }
      const minutes = powerData.smc.chargingStatus === 1
        ? powerData.smc.timeToFull
        : powerData.smc.timeToEmpty

      const hours = Math.floor(minutes / 60)

      return `${hours}h ${minutes % 60}m`
    })(),
    adapterDetails: {
      voltage: powerData.io.adapterDetails?.adapterVoltage,
      amperage: powerData.io.adapterDetails?.current,
      watts: powerData.io.adapterDetails?.watts,
      name: powerData.io.adapterDetails?.name || `Adapter (${powerData.io.adapterDetails?.description})`,
    },
    batteryLevel: powerData.io.appleRawCurrentCapacity
      / powerData.io.appleRawMaxCapacity
      * 100,
    systemPower: powerData.smc.systemTotal,
    screenPower: powerData.smc.brightness,
    heatpipePower: powerData.smc.heatpipe,
    systemIn: powerData.smc?.deliveryRate || 0,
    batteryPower: powerData.smc.batteryRate,
    powerLoss: powerData.io.powerTelemetryData?.adapterEfficiencyLoss,
  }))
}
