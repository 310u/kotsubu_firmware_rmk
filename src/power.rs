use core::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, Ordering};

use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::watch::Watch;
use embassy_time::{Instant, Timer};
use rmk::embassy_futures::select::{Either, select};
use rmk::event::{SleepStateEvent, publish_event};
use rmk::usb::is_usb_configured;

pub const BATTERY_ACTIVE_INTERVAL_SECS: u64 = 10;
pub const BATTERY_IDLE_INTERVAL_SECS: u64 = 60;
pub const BATTERY_SLEEP_INTERVAL_SECS: u64 = 300;
pub const BATTERY_IDLE_AFTER_SECS: u32 = 5;
pub const SLEEP_TIMEOUT_SECS: u32 = 300;
pub const DEEP_SLEEP_TIMEOUT_SECS: u32 = 1800;
pub const DEEP_SLEEP_SKIP_WHEN_USB_CONFIGURED: bool = true;

pub static LAST_ACTIVITY_SEC: AtomicU32 = AtomicU32::new(0);
pub static PRESSED_KEY_COUNT: AtomicU8 = AtomicU8::new(0);
pub static SLEEPING: AtomicBool = AtomicBool::new(false);
pub static ACTIVITY_WATCH: Watch<ThreadModeRawMutex, u32, 2> = Watch::new();

#[inline]
fn now_secs() -> u32 {
    Instant::now().as_secs() as u32
}

pub fn init() {
    LAST_ACTIVITY_SEC.store(now_secs(), Ordering::Release);
}

pub fn note_activity() {
    let now = now_secs();
    LAST_ACTIVITY_SEC.store(now, Ordering::Release);
    ACTIVITY_WATCH.sender().send(now);
}

pub fn note_matrix_event(pressed: bool) {
    if pressed {
        PRESSED_KEY_COUNT.fetch_add(1, Ordering::AcqRel);
    } else {
        let _ = PRESSED_KEY_COUNT.fetch_update(Ordering::AcqRel, Ordering::Acquire, |count| {
            Some(count.saturating_sub(1))
        });
    }
    note_activity();
}

pub fn pressed_keys() -> u8 {
    PRESSED_KEY_COUNT.load(Ordering::Acquire)
}

pub fn idle_seconds() -> u32 {
    now_secs().saturating_sub(LAST_ACTIVITY_SEC.load(Ordering::Acquire))
}

pub fn is_sleeping() -> bool {
    SLEEPING.load(Ordering::Acquire)
}

pub fn battery_interval_secs() -> u64 {
    if is_sleeping() {
        BATTERY_SLEEP_INTERVAL_SECS
    } else if pressed_keys() == 0 && idle_seconds() >= BATTERY_IDLE_AFTER_SECS {
        BATTERY_IDLE_INTERVAL_SECS
    } else {
        BATTERY_ACTIVE_INTERVAL_SECS
    }
}

pub fn should_enter_deep_sleep() -> bool {
    pressed_keys() == 0
        && idle_seconds() >= DEEP_SLEEP_TIMEOUT_SECS
        && (!DEEP_SLEEP_SKIP_WHEN_USB_CONFIGURED || !is_usb_configured())
}

pub fn deep_sleep_wait_secs() -> Option<u64> {
    if pressed_keys() > 0 || (DEEP_SLEEP_SKIP_WHEN_USB_CONFIGURED && is_usb_configured()) {
        return None;
    }

    let idle = idle_seconds();
    if idle >= DEEP_SLEEP_TIMEOUT_SECS {
        Some(0)
    } else {
        Some(u64::from(DEEP_SLEEP_TIMEOUT_SECS - idle))
    }
}

pub async fn sleep_manager_task() -> ! {
    let mut activity_rx = ACTIVITY_WATCH.receiver().unwrap();

    loop {
        if is_sleeping() {
            activity_rx.changed().await;
            if SLEEPING.swap(false, Ordering::AcqRel) {
                publish_event(SleepStateEvent { sleeping: false });
            }
            continue;
        }

        if pressed_keys() > 0 {
            activity_rx.changed().await;
            continue;
        }

        let idle = idle_seconds();
        if idle >= SLEEP_TIMEOUT_SECS {
            if !SLEEPING.swap(true, Ordering::AcqRel) {
                publish_event(SleepStateEvent { sleeping: true });
            }
            continue;
        }

        let wait_secs = u64::from(SLEEP_TIMEOUT_SECS - idle);
        match select(Timer::after_secs(wait_secs), activity_rx.changed()).await {
            Either::First(_) | Either::Second(_) => {}
        }
    }
}
