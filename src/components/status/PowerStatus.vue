<template>
  <div class="w-[22rem] flex gap-4">
    <Card class="flex-1">
      <CardHeader class="space-y-0 pb-2 gap-y-0">
        <CardTitle class="flex items-center justify-between gap-2 text-base truncate">
          <PowerStatusTitle />
        </CardTitle>
        <CardDescription class="text-[10px] font-mono flex gap-[2px] items-center">
          <PowerStatusDescription />
        </CardDescription>
      </CardHeader>
      <CardContent class="space-y-3 mt-1">
        <PowerStatusNumber />
        <PowerStatusFooter />
      </CardContent>
    </Card>
  </div>
</template>
