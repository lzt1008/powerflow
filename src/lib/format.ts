import type { ArgumentsType } from '@vueuse/core'
import type { useI18n } from 'vue-i18n'
import { type FormatDistanceToken, formatDistanceToNow, type Locale } from 'date-fns'
import { enUS, zhCN } from 'date-fns/locale'

export function formatChargingDuration(seconds: number, t: ReturnType<typeof useI18n>['t']) {
  const hours = Math.floor(seconds / 3600)
  const minutes = Math.floor((seconds % 3600) / 60)
  return hours > 0 ? `${hours}${t('time.hour')} ${minutes}${t('time.minute')}` : `${minutes}${t('time.minute')}`
}

export const localeMap = {
  'en': enUS,
  'zh-CN': zhCN,
}

export const shortestDistanceLocale: Locale = {
  ...enUS,
  formatDistance: (token, count, options) => {
    const shortUnits: Record<FormatDistanceToken, string> = {
      lessThanXSeconds: `<${count}s`,
      lessThanXMinutes: `<${count}m`,
      halfAMinute: 'half a min',
      aboutXHours: '~1h',
      aboutXMonths: '~1mo',
      aboutXYears: '~1y',
      aboutXWeeks: '~1w',
      almostXYears: '~1y',
      xWeeks: `${count}w`,
      xSeconds: `${count}s`,
      xMinutes: `${count}m`,
      xHours: `${count}h`,
      xDays: `${count}d`,
      xMonths: `${count}mo`,
      xYears: `${count}y`,
      overXYears: `>${count}y`,
    }
    return `${shortUnits[token]} ${options?.addSuffix ? 'ago' : ''}`
  },
}

export const shortEnDistanceLocale: Locale = {
  ...enUS,
  formatDistance: (token, count, options) => {
    const shortUnits: Record<FormatDistanceToken, string> = {
      lessThanXSeconds: `less than${count}s`,
      lessThanXMinutes: `less than ${count}m`,
      halfAMinute: 'half a min',
      aboutXHours: `~${count}hour`,
      aboutXMonths: `~${count}mon`,
      aboutXYears: `~${count}year`,
      aboutXWeeks: `~${count}w`,
      almostXYears: `~${count}y`,
      xWeeks: `${count}week`,
      xSeconds: `${count}sec`,
      xMinutes: `${count}mins`,
      xHours: `${count}hours`,
      xDays: `${count}days`,
      xMonths: `${count}mon`,
      xYears: `${count}years`,
      overXYears: `>${count}years`,
    }
    return `${shortUnits[token]} ${options?.addSuffix ? 'ago' : ''}`
  },
}

export function formatUpdateTime(date: ArgumentsType<typeof formatDistanceToNow>[0]) {
  return formatDistanceToNow(date, {
    addSuffix: true,
    includeSeconds: true,
    locale: shortestDistanceLocale,
  })
}
