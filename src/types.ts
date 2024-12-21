export interface SMCPowerData {
  batteryRate: number;
  deliveryRate: number;
  systemTotal: number;
  heatpipe?: number;
  brightness?: number;
  fullChargeCapacity: number;
  currentCapacity: number;
  chargingStatus: number;
  timeToEmpty: number;
  timeToFull: number;
  temperature: number;
}

export interface IORegistry {
  amperage: number;
  voltage: number;
  absoluteCapacity: number;
  designCapacity: number;
  maxCapacity: number;
  currentCapacity: number;
  appleRawMaxCapacity: number;
  appleRawCurrentCapacity: number;
  appleRawBatteryVoltage: number;
  cycleCount: number;
  fullyCharged: boolean;
  isCharging: boolean;
  temperature: number;
  timeRemaining: number;
  updateTime: number;
  virtualTemperature: number;
  adapterDetails: AdapterDetails;
  powerTelemetryData: PowerTelemetryData;
}

export interface AdapterDetails {
  adapterVoltage: number;
  isWireless: boolean;
  watts: number;
  name: string;
  current: number;
}

export interface PowerTelemetryData {
  adapterEfficiencyLoss: number;
  batteryPower: number;
  systemCurrentIn: number;
  systemEnergyConsumed: number;
  systemLoad: number;
  systemPowerIn: number;
  systemVoltageIn: number;
}
