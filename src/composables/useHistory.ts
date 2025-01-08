import { type ChargingHistory, commands, type Result } from '@/bindings'

export function useAsyncData<T>(promiseFn: () => Promise<Result<T, string>>) {
  const data = ref<T | null>(null)
  const isLoading = ref(true)
  const err = ref('')

  const load = async () => {
    const r = await promiseFn()
    if (r.status === 'ok') {
      data.value = r.data
    }
    else {
      err.value = r.error
      console.error(r.error)
    }
    isLoading.value = false
  }

  const update = () => {
    isLoading.value = true
    data.value = null
    err.value = ''
    load()
  }

  load()

  return {
    data,
    isLoading,
    err,
    update,
  }
}

const selectedItem = ref(null as ChargingHistory | null)
const history = useAsyncData<ChargingHistory[]>(() => commands.getAllChargingHistory())

export function useHistory() {
  return {
    selectedItem,
    history,
  }
}
