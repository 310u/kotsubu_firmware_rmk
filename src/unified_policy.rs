#![allow(dead_code)]

pub type LedColor = (bool, bool, bool);

pub const BLE_PROFILE_COUNT: usize = 5;
pub const PASSKEY_ENTRY_ENABLED: bool = false;
pub const PASSKEY_ENTRY_TIMEOUT_SECS: u32 = 120;

pub const BATTERY_GOOD_THRESHOLD_PERCENT: u8 = 50;
pub const BATTERY_CRITICAL_THRESHOLD_PERCENT: u8 = 20;

pub const LED_OFF: LedColor = (false, false, false);
pub const LED_RED: LedColor = (true, false, false);
pub const LED_GREEN: LedColor = (false, true, false);
pub const LED_BLUE: LedColor = (false, false, true);
pub const LED_YELLOW: LedColor = (true, true, false);
pub const LED_MAGENTA: LedColor = (true, false, true);
pub const LED_WHITE: LedColor = (true, true, true);

pub const LED_BOOT_SELF_TEST_ENABLED: bool = true;
pub const LED_SELF_TEST_STEP_MS: u64 = 120;
pub const LED_BOOT_BATTERY_DISPLAY_MS: u64 = 1200;
pub const LED_STATUS_EVENT_DISPLAY_MS: u64 = 1200;
pub const LED_CONNECTION_SUCCESS_DISPLAY_MS: u64 = 3000;
pub const LED_LOW_BATTERY_DISPLAY_MS: u64 = 1500;
pub const LED_BLINK_ON_MS: u64 = 200;
pub const LED_BLINK_OFF_MS: u64 = 800;

pub const DEEP_SLEEP_REQUIRES_SLEEP_STATE: bool = true;
pub const DEEP_SLEEP_SKIP_WHEN_USB_CONFIGURED: bool = true;
pub const DEEP_SLEEP_TIMEOUT_SECS: u32 = 1800;

pub const fn boot_battery_color(percentage: u8) -> LedColor {
    if percentage >= BATTERY_GOOD_THRESHOLD_PERCENT {
        LED_GREEN
    } else if percentage >= BATTERY_CRITICAL_THRESHOLD_PERCENT {
        LED_YELLOW
    } else {
        LED_RED
    }
}

pub const fn is_battery_critical(percentage: u8) -> bool {
    percentage < BATTERY_CRITICAL_THRESHOLD_PERCENT
}

pub const fn profile_color(profile: u8) -> Option<LedColor> {
    match profile {
        0 => Some(LED_RED),
        1 => Some(LED_GREEN),
        2 => Some(LED_YELLOW),
        3 => Some(LED_BLUE),
        4 => Some(LED_MAGENTA),
        _ => None,
    }
}
