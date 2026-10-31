export function selectDeviceValue<T>(
  local: T,
  remote: Record<string, T>,
  tab: string,
  fallback: T,
): T {
  if (tab === 'local')
    return local
  return remote[tab] ?? fallback
}
