use defmt::info;
use embassy_nrf::gpio::Output;
use embassy_time::{Duration, Instant, Timer};
use rmk::ble::BLE_STATUS;
use rmk::embassy_futures::select::{Either, select};
use rmk::event::{
    BatteryStateEvent, BleStatusChangeEvent, ConnectionChangeEvent, ConnectionType,
    EventSubscriber, SleepStateEvent, SubscribableEvent,
};
use rmk::state::{ConnectionType as RuntimeConnectionType, get_connection_type};
use rmk::types::ble::BleState;

use crate::unified_policy::{self, LedColor};

pub struct RgbLed<'a> {
    red: Output<'a>,
    green: Output<'a>,
    blue: Output<'a>,
}

impl<'a> RgbLed<'a> {
    pub fn new(red: Output<'a>, green: Output<'a>, blue: Output<'a>) -> Self {
        Self { red, green, blue }
    }

    pub fn set_color(&mut self, r: bool, g: bool, b: bool) {
        // XIAO BLE RGB LED is active low (0 = ON, 1 = OFF)
        if r {
            self.red.set_low();
        } else {
            self.red.set_high();
        }

        if g {
            self.green.set_low();
        } else {
            self.green.set_high();
        }

        if b {
            self.blue.set_low();
        } else {
            self.blue.set_high();
        }
    }
}

fn get_bat_percentage(bat: BatteryStateEvent) -> Option<u8> {
    match bat {
        BatteryStateEvent::Normal(p) => Some(p),
        BatteryStateEvent::Charged => Some(100),
        _ => None,
    }
}

pub enum RgbEvent {
    ConnChange(ConnectionChangeEvent),
    BleStatusChange(BleStatusChangeEvent),
    BatChange(BatteryStateEvent),
    SleepChange(SleepStateEvent),
}

fn show_temp_color(
    temp_color_until: &mut Option<(Instant, LedColor)>,
    duration: Duration,
    color: LedColor,
) {
    *temp_color_until = Some((Instant::now() + duration, color));
}

async fn boot_self_test(led: &mut RgbLed<'_>) {
    if !unified_policy::LED_BOOT_SELF_TEST_ENABLED {
        return;
    }

    for color in [
        unified_policy::LED_RED,
        unified_policy::LED_GREEN,
        unified_policy::LED_BLUE,
    ] {
        led.set_color(color.0, color.1, color.2);
        Timer::after(Duration::from_millis(unified_policy::LED_SELF_TEST_STEP_MS)).await;
    }
    led.set_color(
        unified_policy::LED_OFF.0,
        unified_policy::LED_OFF.1,
        unified_policy::LED_OFF.2,
    );
}

fn apply_led_state(
    led: &mut RgbLed<'_>,
    temp_color_until: Option<(Instant, LedColor)>,
    is_sleeping: bool,
    is_adv: bool,
    blink_state: bool,
    bat_critical: bool,
    current_profile: u8,
) {
    if is_sleeping {
        led.set_color(
            unified_policy::LED_OFF.0,
            unified_policy::LED_OFF.1,
            unified_policy::LED_OFF.2,
        );
    } else if let Some((_, color)) = temp_color_until {
        led.set_color(color.0, color.1, color.2);
    } else if is_adv {
        if blink_state {
            let pc = profile_color(current_profile);
            led.set_color(pc.0, pc.1, pc.2);
        } else {
            led.set_color(
                unified_policy::LED_OFF.0,
                unified_policy::LED_OFF.1,
                unified_policy::LED_OFF.2,
            );
        }
    } else if bat_critical {
        if blink_state {
            led.set_color(
                unified_policy::LED_RED.0,
                unified_policy::LED_RED.1,
                unified_policy::LED_RED.2,
            );
        } else {
            led.set_color(
                unified_policy::LED_OFF.0,
                unified_policy::LED_OFF.1,
                unified_policy::LED_OFF.2,
            );
        }
    } else {
        led.set_color(
            unified_policy::LED_OFF.0,
            unified_policy::LED_OFF.1,
            unified_policy::LED_OFF.2,
        );
    }
}

#[embassy_executor::task]
pub async fn rgb_widget_task(mut led: RgbLed<'static>) {
    let mut conn_sub = ConnectionChangeEvent::subscriber();
    //#[cfg(feature = "_ble")]
    let mut ble_sub = BleStatusChangeEvent::subscriber();
    let mut bat_sub = BatteryStateEvent::subscriber();
    let mut sleep_sub = SleepStateEvent::subscriber();

    // Turn off initially
    led.set_color(
        unified_policy::LED_OFF.0,
        unified_policy::LED_OFF.1,
        unified_policy::LED_OFF.2,
    );
    boot_self_test(&mut led).await;

    let mut temp_color_until: Option<(Instant, LedColor)> = None;

    // Initial sequence: wait up to 2 seconds for a battery event to show boot status
    if let Ok(bat) = embassy_time::with_timeout(Duration::from_secs(2), bat_sub.next_event()).await
    {
        if let Some(percentage) = get_bat_percentage(bat) {
            info!("Boot battery level: {}%", percentage);
            show_temp_color(
                &mut temp_color_until,
                Duration::from_millis(unified_policy::LED_BOOT_BATTERY_DISPLAY_MS),
                unified_policy::boot_battery_color(percentage),
            );
        } else {
            info!("Boot battery level: unknown yet");
        }
    } else {
        info!("No battery event received at boot within timeout");
    }

    let initial_ble = BLE_STATUS.lock(|c| c.get());
    let mut is_adv = initial_ble.state == BleState::Advertising;
    let mut is_conn = initial_ble.state == BleState::Connected;
    let mut current_profile = initial_ble.profile;
    let mut bat_critical = false;
    let mut is_sleeping = false;
    let mut blink_state = is_adv;
    let mut next_blink_time = if is_adv {
        Instant::now() + Duration::from_millis(unified_policy::LED_BLINK_ON_MS)
    } else {
        Instant::now()
    };

    apply_led_state(
        &mut led,
        temp_color_until,
        is_sleeping,
        is_adv,
        blink_state,
        bat_critical,
        current_profile,
    );

    loop {
        // Calculate sleep timeout
        let now = Instant::now();
        let timeout = if let Some((until, _)) = temp_color_until {
            if until > now {
                until - now
            } else {
                temp_color_until = None;
                Duration::from_millis(0)
            }
        } else if is_adv || bat_critical {
            if next_blink_time > now {
                next_blink_time - now
            } else {
                Duration::from_millis(0)
            }
        } else {
            Duration::from_secs(86400) // effectively wait forever until event
        };

        // Construct futures
        let t_fut = Timer::after(timeout);
        let ev_fut = async {
            match select(
                select(conn_sub.next_event(), ble_sub.next_event()),
                select(bat_sub.next_event(), sleep_sub.next_event()),
            )
            .await
            {
                Either::First(Either::First(conn)) => RgbEvent::ConnChange(conn),
                Either::First(Either::Second(ble)) => RgbEvent::BleStatusChange(ble),
                Either::Second(Either::First(bat)) => RgbEvent::BatChange(bat),
                Either::Second(Either::Second(sleep)) => RgbEvent::SleepChange(sleep),
            }
        };

        match select(ev_fut, t_fut).await {
            Either::First(event) => {
                match event {
                    RgbEvent::ConnChange(conn) => {
                        let is_ble = conn.connection_type == ConnectionType::Ble;
                        info!("Conn change event: is_ble={}", is_ble);
                        if is_ble {
                            show_temp_color(
                                &mut temp_color_until,
                                Duration::from_millis(unified_policy::LED_STATUS_EVENT_DISPLAY_MS),
                                profile_color(current_profile),
                            );
                        } else {
                            show_temp_color(
                                &mut temp_color_until,
                                Duration::from_millis(unified_policy::LED_STATUS_EVENT_DISPLAY_MS),
                                unified_policy::LED_WHITE,
                            );
                        }
                    }
                    RgbEvent::BleStatusChange(ble) => {
                        info!(
                            "BLE change event: profile={}, state={:?}",
                            ble.0.profile, ble.0.state
                        );
                        let profile_changed = current_profile != ble.0.profile;
                        if profile_changed {
                            current_profile = ble.0.profile;
                            show_temp_color(
                                &mut temp_color_until,
                                Duration::from_millis(unified_policy::LED_STATUS_EVENT_DISPLAY_MS),
                                profile_color(current_profile),
                            );
                        }

                        is_adv = ble.0.state == BleState::Advertising;

                        let new_conn = ble.0.state == BleState::Connected;
                        if !is_conn && new_conn {
                            show_temp_color(
                                &mut temp_color_until,
                                Duration::from_millis(
                                    unified_policy::LED_CONNECTION_SUCCESS_DISPLAY_MS,
                                ),
                                profile_color(current_profile),
                            );
                        }
                        is_conn = new_conn;

                        if is_adv {
                            blink_state = true;
                            next_blink_time =
                                Instant::now() + Duration::from_millis(unified_policy::LED_BLINK_ON_MS);
                        }
                    }
                    RgbEvent::BatChange(bat) => {
                        if let Some(percentage) = get_bat_percentage(bat) {
                            let critical = unified_policy::is_battery_critical(percentage);
                            if critical && !bat_critical {
                                show_temp_color(
                                    &mut temp_color_until,
                                    Duration::from_millis(
                                        unified_policy::LED_LOW_BATTERY_DISPLAY_MS,
                                    ),
                                    unified_policy::LED_RED,
                                );
                                bat_critical = true;
                                next_blink_time = Instant::now();
                            } else if !critical && bat_critical {
                                bat_critical = false;
                            } else if critical {
                                show_temp_color(
                                    &mut temp_color_until,
                                    Duration::from_millis(
                                        unified_policy::LED_LOW_BATTERY_DISPLAY_MS,
                                    ),
                                    unified_policy::LED_RED,
                                );
                            }
                        }
                    }
                    RgbEvent::SleepChange(sleep) => {
                        is_sleeping = sleep.sleeping;
                        if is_sleeping {
                            temp_color_until = None;
                            blink_state = false;
                        }
                    }
                }
            }
            Either::Second(_) => {
                // Timer tick
                let now = Instant::now();
                if let Some((until, _)) = temp_color_until {
                    if now >= until {
                        temp_color_until = None;
                    }
                }

                if temp_color_until.is_none() && (is_adv || bat_critical) {
                    if now >= next_blink_time {
                        blink_state = !blink_state;
                        next_blink_time = now
                            + Duration::from_millis(if blink_state {
                                unified_policy::LED_BLINK_ON_MS
                            } else {
                                unified_policy::LED_BLINK_OFF_MS
                            });
                    }
                }
            }
        }

        apply_led_state(
            &mut led,
            temp_color_until,
            is_sleeping,
            is_adv,
            blink_state,
            bat_critical,
            current_profile,
        );
    }
}

fn profile_color(profile: u8) -> (bool, bool, bool) {
    if let Some(color) = unified_policy::profile_color(profile) {
        color
    } else {
        match get_connection_type() {
            RuntimeConnectionType::Usb => unified_policy::LED_WHITE,
            RuntimeConnectionType::Ble => unified_policy::LED_MAGENTA,
        }
    }
}
