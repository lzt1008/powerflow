import type { UseTimeAgoOptions } from '@vueuse/core'
import { useI18n } from 'vue-i18n'

export function useTimeAgoOptions(short = false): UseTimeAgoOptions<false> {
  const { d, t } = useI18n()
  const prefix = short ? 'short_' : ''

  const fn = (n: number, past: boolean, key: string) =>
    t(`time_ago_options.${prefix}${key}_${past ? 'past' : 'future'}`, n)

  return {
    rounding: 'floor',
    showSecond: true,
    updateInterval: 1000,
    // a month
    max: 1000 * 3600 * 24 * 30,
    messages: {
      justNow: t('time_ago_options.just_now'),
      // just return the value
      past: n => n,
      // just return the value
      future: n => n,
      second: (n, p) => fn(n, p, 'second'),
      minute: (n, p) => fn(n, p, 'minute'),
      hour: (n, p) => fn(n, p, 'hour'),
      day: (n, p) => fn(n, p, 'day'),
      week: (n, p) => fn(n, p, 'week'),
      month: (n, p) => fn(n, p, 'month'),
      year: (n, p) => fn(n, p, 'year'),
      invalid: '',
    },
    fullDateFormatter(date) {
      return d(date, {
        year: date.getFullYear () !== new Date().getFullYear() ? 'numeric' : undefined,
        month: 'long',
        day: 'numeric',
      })
    },
  }
}
