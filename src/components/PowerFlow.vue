<script setup lang="tsx">
import { Battery, CloudLightningIcon, Cpu, Laptop, Monitor, Smartphone } from 'lucide-vue-next'
import CommonTooltip from './CommonTooltip.vue'

const formatter = new Intl.NumberFormat('en-US', {
  maximumFractionDigits: 1,
  minimumFractionDigits: 1,
})

interface FlowItemProps {
  tooltip: string
  icon: Component
  color: string
}

const FlowItem: Component = ({ tooltip, icon, color }: FlowItemProps, { slots }) => {
  return (
    <CommonTooltip content={tooltip} as-child>
      <div class="w-24 shrink-0 flex justify-center items-center gap-2 rounded-lg border bg-background px-2 py-1.5 cursor-pointer">
        { h(icon, { class: `h-4 w-4 ${color}` }) }
        <span class="text-xs font-medium">
          { slots.default?.() }
        </span>
      </div>
    </CommonTooltip>
  )
}
const power = usePower()
</script>

<template>
  <Card class="flex-1">
    <CardHeader>
      <CardTitle>{{ $t('power_flow') }}</CardTitle>
    </CardHeader>
    <CardContent>
      <Skeleton v-if="power.isLoading" class="w-full h-[120px]" />
      <div
        v-else
        class="flex justify-between items-center w-full rounded-lg border bg-muted/50 p-4 font-mono text-secondary-foreground text-xs h-[120px]"
        :class="[power.isCharging ? '' : 'flex-row-reverse']"
      >
        <FlowItem
          v-if="power.isCharging"
          tooltip="Adapter power"
          :icon="CloudLightningIcon"
          color="text-yellow-500"
        >
          {{ formatter.format(power.systemIn + power.powerLoss / 1000) }}w
        </FlowItem>

        <CommonTooltip
          v-if="power.isCharging && power.io.powerTelemetryData?.adapterEfficiencyLoss"
          :content="`(loss ${power.io.powerTelemetryData?.adapterEfficiencyLoss}mw)`"
          as-child
        >
          <Shimmer
            v-if="power.isCharging"
            :repeat-delay="1500"
            class="rounded-full mx-2 w-full
          [--base-color:theme(colors.green.500)]
          [--base-gradient-color:theme(colors.green.300)]
          dark:[--base-color:theme(colors.green.700)]
          dark:[--base-gradient-color:theme(colors.green.400)]"
          >
            <div class="h-1 cursor-pointer" />
          </Shimmer>
        </CommonTooltip>

        <div class="flex flex-col items-center gap-2 bg-muted/50 rounded-lg border p-2">
          <div v-if="!power.isRemote" class="flex gap-4" color="text-blue-500">
            <FlowItem tooltip="Power of the screen" :icon="Monitor" color="text-blue-500">
              {{ formatter.format(power.screenPower || 0) }}w
            </FlowItem>

            <FlowItem tooltip="Power of CPU" :icon="Cpu" color="text-indigo-500">
              {{ formatter.format(power.heatpipePower || 0) }}w
            </FlowItem>
          </div>

          <FlowItem
            tooltip="System total power"
            :icon="power.isRemote ? Smartphone : Laptop"
            color="text-cyan-500"
          >
            {{ formatter.format(power.systemPower) }}w
          </FlowItem>
        </div>

        <Shimmer
          :delay="2000"
          :repeat-delay="1500"
          class="rounded-full mx-2 w-full
          [--base-color:theme(colors.green.500)]
          [--base-gradient-color:theme(colors.green.300)]
          dark:[--base-color:theme(colors.green.700)]
          dark:[--base-gradient-color:theme(colors.green.400)]"
        >
          <div class="h-1 cursor-pointer" />
        </Shimmer>

        <FlowItem tooltip="Battery power" :icon="Battery" color="text-green-500">
          {{ formatter.format(power.isCharging ? power.systemIn - power.systemPower : power.batteryPower) }}w
        </FlowItem>
      </div>
    </CardContent>
  </Card>
</template>
