import { describe, expect, it } from 'vitest'
import { resolveLocale } from './useSetup'

describe('resolveLocale', () => {
  it('normalizes supported language families', () => {
    expect(resolveLocale('zh-Hans-CN', 'en')).toBe('zh-CN')
    expect(resolveLocale('en-GB', 'zh-CN')).toBe('en')
  })

  it('falls back for missing or unsupported locales', () => {
    expect(resolveLocale(undefined, 'en')).toBe('en')
    expect(resolveLocale('de-DE', 'zh-CN')).toBe('zh-CN')
  })
})
