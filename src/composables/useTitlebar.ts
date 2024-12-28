import { commands } from '@/bindings'

const tab = useTab()
const data = usePowerData()
const shouldDisplayShadow = ref(false)

const tabNameLoading = ref(true)
const tabName = computedAsync(async () => {
  if (tab.value === 'local') {
    return commands.getMacName().then(name => name || 'Local')
  }

  const currTab = tab.value as string
  const device = await commands.getDeviceName(currTab)
  data.remote[currTab].name = device?.[0] || currTab
  data.remote[currTab].interface = new Set(device?.[1] || [])

  return data.remote[currTab].name
}, '', tabNameLoading)

export function useTitlebar() {
  return {
    shouldDisplayShadow,
    tabName,
    tabNameLoading,
  }
}
