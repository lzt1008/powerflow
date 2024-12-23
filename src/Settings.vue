<script setup lang="tsx">
import type { Component, SetupContext } from 'vue'
import { Label } from '@/components/ui/label'
import {
  NumberField,
  NumberFieldContent,
  NumberFieldDecrement,
  NumberFieldIncrement,
  NumberFieldInput,
} from '@/components/ui/number-field'
import {
  Select,
  SelectContent,
  SelectGroup,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { Separator } from '@/components/ui/separator'
import { Switch } from '@/components/ui/switch'
import { open } from '@tauri-apps/plugin-shell'
import { Activity, BadgeInfo, BatteryCharging, CircleDashed, ExternalLink, Eye, Gauge, Languages, Moon, Palette, RotateCw, Sun, SunMoon, Wallet } from 'lucide-vue-next'
import { storeToRefs } from 'pinia'
import { h, ref, watch } from 'vue'
import { version } from '../package.json'
import { events } from './bindings'
import { Skeleton } from './components/ui/skeleton'
import { usePreference } from './stores/preference'

const commitHash = __COMMIT_HASH__

const loading = ref(true)
const preference = usePreference()

preference.$tauri.start().then(() => {
  loading.value = false
  const refs = storeToRefs(preference)

  for (const key in refs) {
    // TODO: fix types
    const ref = refs[key as keyof typeof refs]
    watch(ref, () => {
      events.preferenceEvent.emit({
        [key as keyof typeof refs]: ref.value,
      } as any)
    })
  }
})

interface SettingsItemProps {
  name: string
  description: string
  icon: Component
}

function SettingsItem(props: SettingsItemProps, { slots }: SetupContext) {
  return (
    <div class="flex items-center justify-between">
      <div class="flex gap-4">
        {
          h(props.icon, { class: 'size-4 mt-[4px]' })
        }
        <div class="flex flex-col gap-1">
          <Label class="font-medium">{props.name}</Label>
          <span class="text-xs text-muted-foreground mr-4">
            {props.description}
          </span>
        </div>
      </div>
      {slots.default ? slots.default() : null}
    </div>
  )
}

interface SettingsSectionProps {
  title: string
  icon: Component
}

function SettingsSection(props: SettingsSectionProps) {
  return (
    <div>
      <h3 class="flex items-center gap-2 text-lg font-bold">
        {h(props.icon, { class: 'h-5 w-5' })}
        {props.title}
      </h3>
    </div>
  )
}
</script>

<template>
  <div class="space-y-8 p-6 bg-background overflow-auto h-dvh">
    <!-- Appearance Section -->
    <section class="space-y-8">
      <SettingsSection title="Appearance" :icon="Eye" />
      <div class="space-y-4">
        <SettingsItem
          name="Theme"
          description="Select preferred theme"
          :icon="Palette"
        >
          <Select v-model="preference.theme">
            <SelectTrigger class="w-[130px]">
              <SelectValue placeholder="Select a theme" />
            </SelectTrigger>
            <SelectContent>
              <SelectGroup>
                <SelectItem value="system">
                  <div class="flex items-center">
                    <SunMoon class="size-4 mr-3" />
                    System
                  </div>
                </SelectItem>
                <SelectItem class="flex" value="light">
                  <div class="flex items-center">
                    <Sun class="size-4 mr-3" />
                    Light
                  </div>
                </SelectItem>
                <SelectItem class="flex" value="dark">
                  <div class="flex items-center">
                    <Moon class="size-4 mr-3" />
                    Dark
                  </div>
                </SelectItem>
              </SelectGroup>
            </SelectContent>
          </Select>
        </SettingsItem>

        <SettingsItem name="Language" description="Select preferred language" :icon="Languages">
          <Select v-model="preference.language" default-value="en">
            <SelectTrigger class="w-[120px]">
              <SelectValue placeholder="Select a language" />
            </SelectTrigger>
            <SelectContent>
              <SelectGroup>
                <SelectItem value="en">
                  English
                </SelectItem>
                <SelectItem value="zh-cn">
                  简体中文
                </SelectItem>
              </SelectGroup>
            </SelectContent>
          </Select>
        </SettingsItem>

        <SettingsItem
          name="Animations" description="Show smooth transitions for changing values"
          :icon="CircleDashed"
        >
          <Skeleton v-if="loading" class="w-12 h-6" />
          <Switch
            v-else id="animations" v-model:checked="preference.animationsEnabled"
            class="data-[state=checked]:bg-green-500"
          />
        </SettingsItem>
      </div>
    </section>

    <Separator />

    <!-- Updates & Monitoring Section -->
    <section class="space-y-8">
      <SettingsSection title="Updates & Monitoring" :icon="RotateCw" />
      <div class="space-y-4">
        <SettingsItem name="Update Frequency" description="How often to refresh power data" :icon="Gauge">
          <NumberField
            v-model="preference.updateInterval"
            :format-options="{
              useGrouping: false,
              style: 'unit',
              unit: 'millisecond',
              unitDisplay: 'short',
            }"
            locale="en-US" :min="500" :step="500" class="w-32"
          >
            <NumberFieldContent>
              <NumberFieldDecrement />
              <NumberFieldInput />
              <NumberFieldIncrement />
            </NumberFieldContent>
          </NumberField>
        </SettingsItem>

        <SettingsItem
          name="Background Monitoring" description="Continue monitoring when app is in background"
          :icon="Activity"
        >
          <Switch id="background-monitoring" class="data-[state=checked]:bg-green-500" disabled checked />
        </SettingsItem>

        <SettingsItem
          name="Status Bar" description="What kind of information to show in status bar"
          :icon="BadgeInfo"
        >
          <Select v-model="preference.statusBarItem" default-value="system">
            <SelectTrigger class="w-[150px]">
              <SelectValue placeholder="Select a language" />
            </SelectTrigger>
            <SelectContent>
              <SelectGroup>
                <SelectItem value="system">
                  System Toatal
                </SelectItem>
                <SelectItem value="screen">
                  Screen Power
                </SelectItem>
                <SelectItem value="heatpipe">
                  Heatpipe Power
                </SelectItem>
              </SelectGroup>
            </SelectContent>
          </Select>
        </SettingsItem>

        <SettingsItem
          name="Show Charging Power" description="When charging, override status bar with charging power"
          :icon="BatteryCharging"
        >
          <Switch v-model:checked="preference.statusBarShowCharging" class="data-[state=checked]:bg-green-500" />
        </SettingsItem>
      </div>
    </section>

    <Separator />

    <!-- About Section -->
    <section class="space-y-8">
      <div>
        <h3 class="flex items-center gap-2 text-lg font-semibold">
          <Wallet class="h-5 w-5" />
          About
        </h3>
      </div>
      <div class="grid grid-cols-2 gap-4">
        <div>
          <div class="text-sm font-medium text-muted-foreground">
            Version
          </div>
          <div class="text-sm">
            {{ version }}
          </div>
        </div>
        <div>
          <div class="text-sm font-medium text-muted-foreground">
            Build
          </div>
          <div class="text-sm flex items-center">
            {{ commitHash.slice(0, 7) }}
            <a
              class="ml-2 mr-1 text-xs text-muted-foreground underline flex items-center gap-1 cursor-pointer"
              @click="open(`https://github.com/lzt1008/powerflow/commit/${commitHash}`)"
            >
              View on GitHub
              <ExternalLink class="size-3 text-muted-foreground" />
            </a>
          </div>
        </div>
        <div>
          <div class="text-sm font-medium text-muted-foreground">
            License
          </div>
          <div class="text-sm">
            MIT License
          </div>
        </div>
        <div>
          <div class="text-sm font-medium text-muted-foreground">
            Author
          </div>
          <div class="text-sm">
            Samuel Lyon
          </div>
        </div>
      </div>
    </section>
  </div>
</template>
