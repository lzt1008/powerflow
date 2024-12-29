import type { InterfaceType, IORegistry, SMCPowerData } from '@/bindings'
import type { Reactive } from 'vue'
import { events } from '@/bindings'
import { useDocumentVisibility } from '@vueuse/core'

import { computed, reactive } from 'vue'
import { useTab } from './useTab'

const MAX_STATISTICS_LENGTH = 20
const LOCAL_UPDATE_INTERVAL = 3

export interface StatisticData {
  'time': string
  'System Power': number
  'System In': number
  'Battery Level': number
  'Screen Power'?: number
  'Heatpipe Power'?: number
}

interface RawPowerData {
  smc: SMCPowerData
  io: IORegistry
  statistics: StatisticData[]
}

function trimStatistics(statistics: StatisticData[]) {
  if (statistics.length > MAX_STATISTICS_LENGTH)
    statistics.shift()
}

function calcBatteryLevel(io: IORegistry): number {
  if (!io.appleRawMaxCapacity)
    return 0
  return (io.appleRawCurrentCapacity / io.appleRawMaxCapacity) * 100
}

const localPowerData: Reactive<RawPowerData> = reactive({
  smc: {} as SMCPowerData,
  io: {} as IORegistry,
  statistics: [],
})

let localUpdateCount = 0

events.powerTickEvent.listen(async ({ payload }) => {
  const { smc, io } = payload
  localPowerData.smc = smc
  localPowerData.io = io

  localUpdateCount++
  if (localUpdateCount < LOCAL_UPDATE_INTERVAL)
    return
  localUpdateCount = 0

  trimStatistics(localPowerData.statistics)

  const levelRaw = calcBatteryLevel(localPowerData.io)
  const sysPowerRaw = smc.systemTotal

  localPowerData.statistics.push({
    'time': new Date().toLocaleTimeString(),
    'System Power': +(sysPowerRaw - (sysPowerRaw % 0.01)).toFixed(2),
    'System In': smc.deliveryRate < 0.01 ? 0 : smc.deliveryRate,
    'Battery Level': +(levelRaw - (levelRaw % 0.01)).toFixed(2),
    'Screen Power': smc.brightness || 0,
    'Heatpipe Power': smc.heatpipe || 0,
  })
})

export type RemotePowerData = RawPowerData & {
  name: string
  offline: boolean
  interface: Set<InterfaceType>
}

interface PowerData {
  local: RawPowerData
  remote: Record<string, RemotePowerData>
}

const power = reactive<PowerData>({
  local: localPowerData,
  remote: {},
})

function getOrCreateDeviceData(udid: string): RemotePowerData {
  if (!power.remote[udid]) {
    power.remote[udid] = {
      io: {} as IORegistry,
      smc: {} as SMCPowerData,
      statistics: [],
      name: '',
      offline: false,
      interface: new Set(),
    }
  }
  return power.remote[udid]
}

events.deviceEvent.listen(({ payload }) => {
  const deviceData = getOrCreateDeviceData(payload.udid)

  if (payload.action === 'Attached') {
    deviceData.interface.add(payload.interface)
    deviceData.offline = false
  }
  else if (payload.action === 'Detached') {
    deviceData.interface.delete(payload.interface)
  }
  if (deviceData.interface.size === 0) {
    deviceData.offline = true
  }
})

events.devicePowerTickEvent.listen(({ payload }) => {
  const deviceData = getOrCreateDeviceData(payload.udid)
  deviceData.io = payload.io

  const statistics = deviceData.statistics
  trimStatistics(statistics)

  const level = calcBatteryLevel(payload.io)
  const powerTelemetryData = payload.io.powerTelemetryData
  // TODO
  const power = payload.io.amperage * payload.io.voltage / 1000
  const time = new Date(payload.io.updateTime * 1000).toLocaleTimeString()

  if (!statistics.length || time !== statistics[statistics.length - 1]?.time) {
    statistics.push({
      time,
      'System Power': (powerTelemetryData?.systemLoad || power || 0) / 1000,
      'System In': (powerTelemetryData?.systemPowerIn || power || 0) / 1000,
      'Battery Level': +level.toFixed(2),
    })
  }
})

const vis = useDocumentVisibility()
const tab = useTab()

const currentPower = computed<RawPowerData>(() => {
  return tab.value === 'local' ? power.local : power.remote[tab.value] || {}
})

export function usePower() {
  return computed<LocalPowerReturn | RemotePowerReturn>(() => {
    const isLocal = tab.value === 'local'
    return createPowerData(currentPower.value, isLocal)
  })
}

export function usePowerData() {
  return power
}

export function usePowerRaw() {
  return computed<
    RawPowerData & { isLocal: true } |
    RemotePowerData & { isLocal: false }
  >(() => {
    const isLocal = tab.value === 'local'
    return {
      ...isLocal ? currentPower.value : power.remote[tab.value],
      isLocal,
    } as any
  })
}

interface PowerReturnBase {
  isLoading: boolean
  isReady: boolean
  statistics: StatisticData[]
  io: IORegistry
  isCharging: boolean
  timeRemaining: number
  adapterDetails?: {
    name?: string
    voltage?: number
    amperage?: number
    watts?: number
  }
  batteryLevel: number
  batteryPower: number
  systemPower: number
  systemIn: number
  powerLoss: number
  temperature: number
  fullyCharged: boolean
}

interface LocalPowerReturn extends PowerReturnBase {
  isRemote: false
  smc: SMCPowerData
  heatpipePower: number
  screenPower: number
}

interface RemotePowerReturn extends PowerReturnBase {
  isRemote: true
}

function createPowerData(
  { smc, io, statistics }: RawPowerData,
  isLocal: boolean,
): LocalPowerReturn | RemotePowerReturn {
  const isIOEmpty = Object.keys(io).length === 0
  const detail = io.adapterDetails

  const base: Omit<PowerReturnBase, 'isCharging' | 'timeRemaining'> = {
    isLoading: isIOEmpty || vis.value === 'hidden',
    isReady: false,
    statistics,
    io,
    adapterDetails: {
      voltage: detail?.adapterVoltage || 0,
      amperage: detail?.current || 0,
      watts: detail?.watts || 0,
      name: detail?.name || detail?.description || '',
    },
    batteryLevel: calcBatteryLevel(io),
    powerLoss: (io.powerTelemetryData?.adapterEfficiencyLoss ?? 0) / 1000,
    temperature: (io.temperature || 0) / (isLocal ? 1 : 100),
    systemIn: 0,
    batteryPower: 0,
    systemPower: 0,
    fullyCharged: io.fullyCharged,
  }

  if (isLocal) {
    const localData: LocalPowerReturn = {
      ...base,
      isRemote: false,
      smc,
      isCharging: smc.chargingStatus === 1,
      timeRemaining: smc.chargingStatus === 1 ? smc.timeToFull : smc.timeToEmpty,
      systemPower: smc.systemTotal || 0,
      screenPower: smc.brightness || 0,
      heatpipePower: smc.heatpipe || 0,
      systemIn: smc.deliveryRate || 0,
      batteryPower: smc.batteryRate || 0,
    }
    return localData
  }
  else {
    const ptd = io.powerTelemetryData
    const p = io.amperage * io.voltage / 1000
    const remoteData: RemotePowerReturn = {
      ...base,
      isRemote: true,
      isCharging: io.isCharging,
      timeRemaining: io.timeRemaining,
      systemIn: (ptd?.systemPowerIn || p || 0) / 1000,
      batteryPower: (ptd?.batteryPower || p || 0) / 1000,
      systemPower: (ptd?.systemLoad || p || 0) / 1000,
    }
    return remoteData
  }
}
