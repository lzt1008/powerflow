import { commands } from '@/bindings'

const tab = useTab()
const data = usePowerData()
const shouldDisplayShadow = ref(false)

const tabNameLoading = ref(true)
let requestId = 0

const tabName = computedAsync(async () => {
  const id = ++requestId
  const currTab = tab.value

  if (currTab === 'local') {
    return commands.getMacName().then(name => name || 'Local')
  }

  const device = await commands.getDeviceName(currTab)
  // Ignore stale responses from a previous tab switch.
  if (id !== requestId || tab.value !== currTab)
    return data.remote[currTab]?.name || currTab

  const remote = data.remote[currTab]
  if (!remote)
    return device?.[0] || currTab

  if (device) {
    remote.name = device[0] || currTab
    // Merge interfaces from DeviceState; never wipe ones maintained by deviceEvent.
    for (const iface of device[1] || [])
      remote.interface.add(iface)
  }

  return remote.name || currTab
}, '', tabNameLoading)

export function useTitlebar() {
  return {
    shouldDisplayShadow,
    tabName,
    tabNameLoading,
  }
}
