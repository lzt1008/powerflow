import { commands } from '@/bindings'
import { toRaw } from 'vue'

const tab = useTab()
const data = usePowerData()
const shouldDisplayShadow = ref(false)

const tabNameLoading = ref(true)
const tabName = computedAsync(async () => {
  if (tab.value === 'local') {
    return commands.getMacName().then(name => name || 'Local')
  }

  const currTab = tab.value
  const remote = data.remote[currTab]
  if (!remote)
    return currTab

  const interfaces = new Set(toRaw(remote).interface)
  const device = await commands.getDeviceName(currTab)
  if (tab.value !== currTab || data.remote[currTab] !== remote)
    return remote.name || currTab

  remote.name = device?.[0] || remote.name || currTab
  // A connection event received while awaiting the name is newer than this snapshot.
  if (interfaces.size === remote.interface.size
    && [...interfaces].every(iface => remote.interface.has(iface))) {
    remote.interface = new Set(device?.[1] || [])
  }

  return remote.name
}, '', tabNameLoading)

export function useTitlebar() {
  return {
    shouldDisplayShadow,
    tabName,
    tabNameLoading,
  }
}
