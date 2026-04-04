use defmt::info;
use rmk::embassy_futures::select::{select, Either};
use embassy_nrf::gpio::Output;
use embassy_time::{Duration, Instant, Timer};
use rmk::event::{
    BatteryStateEvent, BleStatusChangeEvent, ConnectionChangeEvent, ConnectionType,
    EventSubscriber, SubscribableEvent,
};
use rmk::types::ble::BleState;

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
    TimerTick,
}

#[embassy_executor::task]
pub async fn rgb_widget_task(mut led: RgbLed<'static>) {
    let mut conn_sub = ConnectionChangeEvent::subscriber();
    //#[cfg(feature = "_ble")]
    let mut ble_sub = BleStatusChangeEvent::subscriber();
    let mut bat_sub = BatteryStateEvent::subscriber();

    // Turn off initially
    led.set_color(false, false, false);

    // Initial sequence: wait up to 2 seconds for a battery event to show boot status
    if let Ok(bat) = embassy_time::with_timeout(Duration::from_secs(2), bat_sub.next_event()).await {
        if let Some(percentage) = get_bat_percentage(bat) {
            info!("Boot battery level: {}%", percentage);
            if percentage >= 50 {
                led.set_color(false, true, false); // Green
            } else if percentage >= 20 {
                led.set_color(true, true, false); // Yellow
            } else {
                led.set_color(true, false, false); // Red
            }
            Timer::after(Duration::from_millis(2000)).await;
            led.set_color(false, false, false);
        } else {
            info!("Boot battery level: unknown yet");
        }
    } else {
        info!("No battery event received at boot within timeout");
    }

    let mut is_ble = false;
    let mut is_adv = false;
    let mut is_conn = false;
    let mut current_profile = 0;
    let mut bat_critical = false;

    // Temporary override (e.g. for connection success)
    let mut temp_color_until: Option<(Instant, (bool, bool, bool))> = None;

    let mut blink_state = false;
    let mut next_blink_time = Instant::now();

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
                bat_sub.next_event(),
            )
            .await
            {
                Either::First(Either::First(conn)) => RgbEvent::ConnChange(conn),
                Either::First(Either::Second(ble)) => RgbEvent::BleStatusChange(ble),
                Either::Second(bat) => RgbEvent::BatChange(bat),
            }
        };

        match select(ev_fut, t_fut).await {
            Either::First(event) => {
                match event {
                    RgbEvent::ConnChange(conn) => {
                        is_ble = conn.connection_type == ConnectionType::Ble;
                        info!("Conn change event: is_ble={}", is_ble);
                        if !is_ble {
                            // USB connected: show white briefly
                            temp_color_until = Some((
                                Instant::now() + Duration::from_millis(2000),
                                (true, true, true), // White
                            ));
                        }
                    }
                    RgbEvent::BleStatusChange(ble) => {
                        info!(
                            "BLE change event: profile={}, state={:?}",
                            ble.0.profile, ble.0.state
                        );
                        // Profile changed? Show color
                        if current_profile != ble.0.profile {
                            current_profile = ble.0.profile;
                        }

                        is_adv = ble.0.state == BleState::Advertising;
                        
                        let new_conn = ble.0.state == BleState::Connected;
                        if !is_conn && new_conn {
                            // Just connected: show profile color for 3 secs
                            temp_color_until = Some((
                                Instant::now() + Duration::from_millis(3000),
                                profile_color(current_profile),
                            ));
                        }
                        is_conn = new_conn;

                        if is_adv {
                            next_blink_time = Instant::now(); // reset blink timer immediately
                        }
                    }
                    RgbEvent::BatChange(bat) => {
                        if let Some(percentage) = get_bat_percentage(bat) {
                            let critical = percentage < 20;
                            if critical && !bat_critical {
                                // Just entered critical: show red
                                temp_color_until = Some((
                                    Instant::now() + Duration::from_millis(1500),
                                    (true, false, false), // Red
                                ));
                                bat_critical = true;
                                next_blink_time = Instant::now();
                            } else if !critical && bat_critical {
                                bat_critical = false;
                            } else if critical {
                               // critical state continued, just blink warning
                               temp_color_until = Some((
                                    Instant::now() + Duration::from_millis(1500),
                                    (true, false, false), // Red
                                ));
                            }
                        }
                    }
                    _ => {}
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
                        next_blink_time = now + Duration::from_millis(if blink_state { 200 } else { 800 });
                    }
                }
            }
        }

        // Apply colors
        if let Some((_, color)) = temp_color_until {
            led.set_color(color.0, color.1, color.2);
        } else if is_adv {
            if blink_state {
                let pc = profile_color(current_profile);
                led.set_color(pc.0, pc.1, pc.2);
            } else {
                led.set_color(false, false, false);
            }
        } else if bat_critical {
            if blink_state {
                led.set_color(true, false, false); // Red
            } else {
                led.set_color(false, false, false);
            }
        } else {
            led.set_color(false, false, false);
        }
    }
}

fn profile_color(profile: u8) -> (bool, bool, bool) {
    match profile {
        0 => (true, false, false),  // Red
        1 => (false, true, false),  // Green
        2 => (true, true, false),   // Yellow
        3 => (false, false, true),  // Blue
        _ => (true, false, true),   // Magenta (fallback)
    }
}
