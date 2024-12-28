import type { ArgumentsType } from '@vueuse/core'
import { type FormatDistanceToken, formatDistanceToNow, type Locale } from 'date-fns'
import { enUS, zhCN } from 'date-fns/locale'

export const localeMap = {
  'en': enUS,
  'zh-CN': zhCN,
}

export const shortDistanceLocale: Locale = {
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

export function formatUpdateTime(date: ArgumentsType<typeof formatDistanceToNow>[0]) {
  return formatDistanceToNow(date, {
    addSuffix: true,
    includeSeconds: true,
    locale: shortDistanceLocale,
  })
}
