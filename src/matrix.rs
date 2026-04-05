use defmt::info;
use embassy_nrf::Peri;
use embassy_nrf::gpio::{Input, Level, Output, OutputDrive, Pin, Port, Pull};
use embassy_nrf::pac;
use embassy_nrf::peripherals::{P0_02, P0_03, P0_04, P0_05, P0_28, P0_29, P1_12, P1_13, P1_15};
use embassy_nrf::power as nrf_power;
use embassy_time::{Duration, Instant, Timer};
use rmk::debounce::default_debouncer::DefaultDebouncer;
use rmk::debounce::{DebounceState, DebouncerTrait};
use rmk::embassy_futures::select::{Either5, select4, select5};
use rmk::event::{KeyboardEvent, publish_event_async};
use rmk::input_device::Runnable;
use rmk::matrix::KeyState;

use crate::power;

enum WaitForKeyResult {
    KeyPressed,
    DeepSleepDeadlineReached,
}

pub struct KotsubuMatrix {
    rows: [Input<'static>; 4],
    direct_cols: [Output<'static>; 2],
    shift_clock: Output<'static>,
    shift_data: Output<'static>,
    shift_latch: Output<'static>,
    debouncer: DefaultDebouncer<4, 10>,
    key_states: [[KeyState; 4]; 10],
    scan_pos: (usize, usize),
    row_pin_ports: [u8; 4],
    stay_awake_until: Option<Instant>,
}

impl KotsubuMatrix {
    const SHIFT_COL_BITS: [u8; 8] = [0, 1, 2, 3, 4, 7, 6, 5];

    fn pin_port<P: Pin>(pin: &P) -> u8 {
        match pin.port() {
            Port::Port0 => pin.pin(),
            Port::Port1 => 32 + pin.pin(),
        }
    }

    pub fn new(
        row0: Peri<'static, P0_28>,
        row1: Peri<'static, P0_29>,
        row2: Peri<'static, P0_04>,
        row3: Peri<'static, P0_05>,
        col8: Peri<'static, P0_02>,
        col9: Peri<'static, P0_03>,
        shift_clock: Peri<'static, P1_13>,
        shift_data: Peri<'static, P1_15>,
        shift_latch: Peri<'static, P1_12>,
    ) -> Self {
        let row_pin_ports = [
            Self::pin_port(&*row0),
            Self::pin_port(&*row1),
            Self::pin_port(&*row2),
            Self::pin_port(&*row3),
        ];
        let mut matrix = Self {
            rows: [
                Input::new(row0, Pull::Down),
                Input::new(row1, Pull::Down),
                Input::new(row2, Pull::Down),
                Input::new(row3, Pull::Down),
            ],
            direct_cols: [
                Output::new(col8, Level::Low, OutputDrive::Standard),
                Output::new(col9, Level::Low, OutputDrive::Standard),
            ],
            shift_clock: Output::new(shift_clock, Level::Low, OutputDrive::Standard),
            shift_data: Output::new(shift_data, Level::Low, OutputDrive::Standard),
            shift_latch: Output::new(shift_latch, Level::Low, OutputDrive::Standard),
            debouncer: DefaultDebouncer::new(),
            key_states: [[KeyState::new(); 4]; 10],
            scan_pos: (0, 0),
            row_pin_ports,
            stay_awake_until: None,
        };
        matrix.shift_latch.set_high();
        matrix
    }

    fn shift_mask(col: usize) -> u8 {
        if col < 8 {
            1 << Self::SHIFT_COL_BITS[col]
        } else {
            0
        }
    }

    async fn write_shift_register(&mut self, value: u8) {
        self.shift_latch.set_low();
        self.shift_clock.set_low();

        for bit in (0..8).rev() {
            if (value >> bit) & 1 == 1 {
                self.shift_data.set_high();
            } else {
                self.shift_data.set_low();
            }
            self.shift_clock.set_high();
            self.shift_clock.set_low();
        }

        self.shift_latch.set_high();
        self.shift_data.set_low();
    }

    async fn clear_columns(&mut self) {
        self.direct_cols[0].set_low();
        self.direct_cols[1].set_low();
        self.write_shift_register(0).await;
    }

    async fn set_all_columns_high(&mut self) {
        self.direct_cols[0].set_high();
        self.direct_cols[1].set_high();
        self.write_shift_register(0xFF).await;
    }

    async fn select_column(&mut self, col: usize) {
        self.clear_columns().await;

        if col < 8 {
            self.write_shift_register(Self::shift_mask(col)).await;
        } else {
            self.direct_cols[col - 8].set_high();
        }
    }

    async fn wait_for_key(&mut self) -> WaitForKeyResult {
        self.set_all_columns_high().await;

        let result = match power::deep_sleep_wait_secs() {
            Some(0) => WaitForKeyResult::DeepSleepDeadlineReached,
            Some(wait_secs) => {
                let [row0, row1, row2, row3] = &mut self.rows;
                match select5(
                    row0.wait_for_high(),
                    row1.wait_for_high(),
                    row2.wait_for_high(),
                    row3.wait_for_high(),
                    Timer::after_secs(wait_secs),
                )
                .await
                {
                    Either5::Fifth(_) => WaitForKeyResult::DeepSleepDeadlineReached,
                    _ => WaitForKeyResult::KeyPressed,
                }
            }
            None => {
                let [row0, row1, row2, row3] = &mut self.rows;
                let _ = select4(
                    row0.wait_for_high(),
                    row1.wait_for_high(),
                    row2.wait_for_high(),
                    row3.wait_for_high(),
                )
                .await;
                WaitForKeyResult::KeyPressed
            }
        };

        self.clear_columns().await;
        if matches!(result, WaitForKeyResult::KeyPressed) {
            self.stay_awake_until = Some(Instant::now() + Duration::from_millis(30));
        }
        result
    }

    fn configure_row_wakeup(&self) {
        use pac::gpio::vals::{Input as GpioInput, Pull as GpioPull, Sense};

        for &pin_port in &self.row_pin_ports {
            let pin = usize::from(pin_port % 32);
            match pin_port / 32 {
                0 => pac::P0.pin_cnf(pin).modify(|w| {
                    w.set_input(GpioInput::CONNECT);
                    w.set_pull(GpioPull::PULLDOWN);
                    w.set_sense(Sense::HIGH);
                }),
                1 => pac::P1.pin_cnf(pin).modify(|w| {
                    w.set_input(GpioInput::CONNECT);
                    w.set_pull(GpioPull::PULLDOWN);
                    w.set_sense(Sense::HIGH);
                }),
                _ => {}
            }
        }
    }

    async fn enter_deep_sleep(&mut self) -> ! {
        info!("Entering deep sleep (System OFF)");
        self.set_all_columns_high().await;
        self.configure_row_wakeup();
        Timer::after_millis(1).await;
        nrf_power::set_system_off();
        loop {
            core::hint::spin_loop();
        }
    }

    async fn read_matrix_event(&mut self) -> KeyboardEvent {
        loop {
            let (start_col, start_row) = self.scan_pos;

            for col in start_col..10 {
                self.select_column(col).await;
                Timer::after_micros(1).await;

                let row_start = if col == start_col { start_row } else { 0 };
                for row in row_start..4 {
                    let pressed = self.rows[row].is_high();
                    let debounce = self.debouncer.detect_change_with_debounce(
                        row,
                        col,
                        pressed,
                        &self.key_states[col][row],
                    );

                    if let DebounceState::Debounced = debounce {
                        self.key_states[col][row].toggle_pressed();
                        let pressed = self.key_states[col][row].pressed;
                        power::note_matrix_event(pressed);
                        self.scan_pos = (col, row);
                        self.clear_columns().await;
                        return KeyboardEvent::key(row as u8, col as u8, pressed);
                    }
                }

                self.clear_columns().await;
            }

            self.scan_pos = (0, 0);
            if let Some(until) = self.stay_awake_until {
                if Instant::now() < until {
                    Timer::after_micros(100).await;
                    continue;
                }
                self.stay_awake_until = None;
            }

            if power::should_enter_deep_sleep() {
                self.enter_deep_sleep().await;
            }

            if power::pressed_keys() == 0 {
                if matches!(
                    self.wait_for_key().await,
                    WaitForKeyResult::DeepSleepDeadlineReached
                ) {
                    continue;
                }
            } else {
                Timer::after_micros(100).await;
            }
        }
    }
}

impl Runnable for KotsubuMatrix {
    async fn run(&mut self) -> ! {
        loop {
            let event = self.read_matrix_event().await;
            publish_event_async(event).await;
        }
    }
}
