import { describe, expect, it } from 'vitest'
import { selectDeviceValue } from './power'

describe('selectDeviceValue', () => {
  it('returns local and connected remote values', () => {
    expect(selectDeviceValue(1, { phone: 2 }, 'local', 0)).toBe(1)
    expect(selectDeviceValue(1, { phone: 2 }, 'phone', 0)).toBe(2)
  })

  it('uses a safe fallback for a missing remote tab', () => {
    expect(selectDeviceValue(1, {}, 'detached-phone', 0)).toBe(0)
  })
})
