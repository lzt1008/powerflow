// Public shape samples: Kyome22/SystemInfoKit, commit
// 326b3517925567292ab30908d067638f932b0587, MeasuredValues/AppleSmartBattery.
// Identifiers omitted; text-dump IsCharging 0/1 restored to CFBoolean.
// M5 Pro/macOS 27 fixture captured locally on 2026-10-31.
use tpower::{
    de::{repr, IORegistry},
    provider::NormalizedResource,
};

fn parse(xml: &[u8]) -> NormalizedResource {
    let parsed: repr::IORegistry = plist::from_bytes(xml).unwrap();
    NormalizedResource::from(&IORegistry::from(parsed))
}

#[test]
fn legacy_m4_and_m5_and_nested_m2_and_m5_capacities() {
    for xml in [
        include_bytes!("fixtures/MacBook_Air_M4_macOS_26_appleAdapter.plist").as_slice(),
        include_bytes!("fixtures/MacBook_Pro_M5_macOS_26_appleAdapter.plist").as_slice(),
        include_bytes!("fixtures/MacBook_Air_M2_macOS_27_appleAdapter.plist").as_slice(),
        include_bytes!("fixtures/MacBook_Pro_M5_Pro_macOS_27.plist").as_slice(),
    ] {
        let r = parse(xml);
        assert!(r.current_capacity > 0, "{r:?}");
        assert!(r.max_capacity > 100, "must be mAh, not percentage: {r:?}");
        assert!(r.design_capacity > 100, "{r:?}");
        assert!((0.0..=110.0).contains(&r.absolute_battery_level));
        assert!(
            (r.absolute_battery_level - r.current_capacity as f32 / r.max_capacity as f32 * 100.)
                .abs()
                < 0.01
        );
    }
}

#[test]
fn missing_raw_capacity_keeps_reported_percentage() {
    let r = parse(br#"<plist version="1.0"><dict><key>CurrentCapacity</key><integer>73</integer></dict></plist>"#);
    assert_eq!(r.battery_level, 73);
    assert_eq!(r.absolute_battery_level, 73.0);
}

#[test]
fn empty_nested_battery_is_not_replaced_with_stale_legacy_capacity() {
    let r = parse(br#"<plist version="1.0"><dict><key>AppleRawCurrentCapacity</key><integer>4000</integer><key>BatteryData</key><dict><key>RemainingCapacity</key><integer>0</integer><key>FullChargeCapacity</key><integer>8000</integer><key>DesignCapacity</key><integer>8500</integer></dict></dict></plist>"#);
    assert_eq!(r.current_capacity, 0);
    assert_eq!(r.absolute_battery_level, 0.0);
}

#[test]
fn malformed_registry_returns_parse_error_instead_of_reinterpreting_error_memory() {
    use tpower::ffi::core_foundation::{base::TCFType, dictionary::CFDictionary};
    let dict = tpower::cfdic! { "CurrentCapacity" = "invalid" };
    // Exercise the same CFDictionary -> plist path as the live IOKit reader.
    let dict = unsafe { CFDictionary::wrap_under_get_rule(dict.as_concrete_TypeRef()) };
    assert!(tpower::util::dict_into::<repr::IORegistry>(dict).is_err());
}
