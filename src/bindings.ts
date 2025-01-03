// @ts-nocheck
// This file was generated by [tauri-specta](https://github.com/oscartbeaumont/tauri-specta). Do not edit this file manually.

/** user-defined commands **/


export const commands = {
async openApp() : Promise<void> {
    await TAURI_INVOKE("open_app");
},
async isMainWindowHidden() : Promise<boolean> {
    return await TAURI_INVOKE("is_main_window_hidden");
},
async openSettings() : Promise<void> {
    await TAURI_INVOKE("open_settings");
},
async getDeviceName(id: string) : Promise<[string, InterfaceType[]] | null> {
    return await TAURI_INVOKE("get_device_name", { id });
},
async getMacName() : Promise<string | null> {
    return await TAURI_INVOKE("get_mac_name");
},
async switchTheme(theme: Theme) : Promise<void> {
    await TAURI_INVOKE("switch_theme", { theme });
}
}

/** user-defined events **/


export const events = __makeEvents__<{
deviceEvent: DeviceEvent,
devicePowerTickEvent: DevicePowerTickEvent,
hidePopoverEvent: HidePopoverEvent,
powerTickEvent: PowerTickEvent,
powerUpdatedEvent: PowerUpdatedEvent,
preferenceEvent: PreferenceEvent,
windowLoadedEvent: WindowLoadedEvent
}>({
deviceEvent: "device-event",
devicePowerTickEvent: "device-power-tick-event",
hidePopoverEvent: "hide-popover-event",
powerTickEvent: "power-tick-event",
powerUpdatedEvent: "power-updated-event",
preferenceEvent: "preference-event",
windowLoadedEvent: "window-loaded-event"
})

/** user-defined constants **/



/** user-defined types **/

export type Action = 
/**
 * A device has attached. The device reference belongs to the
 * client. It must be explicitly released, or else it will leak.
 */
"Attached" | 
/**
 * A device has detached. The device object delivered will be
 * the same as the one delivered in the Attached notification. This
 * device reference does not need to be released.
 */
"Detached" | 
/**
 * This notification is delivered in response to
 * 
 * 1. A call to am::DeviceNotificationUnsubscribe().
 * 2. An error occurred on one of the underlying notification systems
 * (i.e. usbmuxd or mDNSResponder crashed or stopped responding).
 * Unsubcribing and resubscribing may recover the notification system.
 */
"NotificationStopped" | "Paired"
export type AdapterDetails = { adapterVoltage: number | null; isWireless: boolean | null; watts: number | null; name: string | null; current: number | null; description: string | null }
export type DeviceEvent = { udid: string; name: string; interface: InterfaceType; action: Action }
export type DevicePowerTickEvent = { udid: string; io: IORegistry }
export type HidePopoverEvent = null
export type IORegistry = { adapterDetails: AdapterDetails; powerTelemetryData: PowerTelemetryData | null; absoluteCapacity: number; amperage: number; voltage: number; appleRawBatteryVoltage: number | null; appleRawCurrentCapacity: number; appleRawMaxCapacity: number; currentCapacity: number; cycleCount: number; designCapacity: number; fullyCharged: boolean; instantAmperage: number; isCharging: boolean; maxCapacity: number; temperature: number; timeRemaining: number; updateTime: number }
export type InterfaceType = "Unknown" | "USB" | "WiFi"
export type PowerTelemetryData = { adapterEfficiencyLoss: number; batteryPower: number; systemCurrentIn: number; systemEnergyConsumed: number; systemLoad: number; systemPowerIn: number; systemVoltageIn: number }
export type PowerTickEvent = { io: IORegistry; smc: SMCPowerData }
export type PowerUpdatedEvent = string
export type PreferenceEvent = { theme: Theme } | { animationsEnabled: boolean } | { updateInterval: number } | { language: string } | { statusBarItem: StatusBarItem } | { statusBarShowCharging: boolean }
export type SMCPowerData = { batteryRate: number; deliveryRate: number; systemTotal: number; heatpipe: number; brightness: number; fullChargeCapacity: number; currentCapacity: number; chargingStatus: number; timeToEmpty: number; timeToFull: number; temperature: number }
export type StatusBarItem = "system" | "screen" | "heatpipe"
export type Theme = "light" | "dark" | "system"
export type WindowLoadedEvent = null

/** tauri-specta globals **/

import {
	invoke as TAURI_INVOKE,
	Channel as TAURI_CHANNEL,
} from "@tauri-apps/api/core";
import * as TAURI_API_EVENT from "@tauri-apps/api/event";
import { type WebviewWindow as __WebviewWindow__ } from "@tauri-apps/api/webviewWindow";

type __EventObj__<T> = {
	listen: (
		cb: TAURI_API_EVENT.EventCallback<T>,
	) => ReturnType<typeof TAURI_API_EVENT.listen<T>>;
	once: (
		cb: TAURI_API_EVENT.EventCallback<T>,
	) => ReturnType<typeof TAURI_API_EVENT.once<T>>;
	emit: null extends T
		? (payload?: T) => ReturnType<typeof TAURI_API_EVENT.emit>
		: (payload: T) => ReturnType<typeof TAURI_API_EVENT.emit>;
};

export type Result<T, E> =
	| { status: "ok"; data: T }
	| { status: "error"; error: E };

function __makeEvents__<T extends Record<string, any>>(
	mappings: Record<keyof T, string>,
) {
	return new Proxy(
		{} as unknown as {
			[K in keyof T]: __EventObj__<T[K]> & {
				(handle: __WebviewWindow__): __EventObj__<T[K]>;
			};
		},
		{
			get: (_, event) => {
				const name = mappings[event as keyof T];

				return new Proxy((() => {}) as any, {
					apply: (_, __, [window]: [__WebviewWindow__]) => ({
						listen: (arg: any) => window.listen(name, arg),
						once: (arg: any) => window.once(name, arg),
						emit: (arg: any) => window.emit(name, arg),
					}),
					get: (_, command: keyof __EventObj__<any>) => {
						switch (command) {
							case "listen":
								return (arg: any) => TAURI_API_EVENT.listen(name, arg);
							case "once":
								return (arg: any) => TAURI_API_EVENT.once(name, arg);
							case "emit":
								return (arg: any) => TAURI_API_EVENT.emit(name, arg);
						}
					},
				});
			},
		},
	);
}
