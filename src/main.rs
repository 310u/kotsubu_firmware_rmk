#![no_std]
#![no_main]

mod board_config;
mod keymap;
mod led_widget;
mod matrix;
mod power;
mod unified_policy;
mod vial;

use defmt::{info, unwrap};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_nrf::gpio::{Level, Output, OutputDrive};
use embassy_nrf::interrupt::InterruptExt;
use embassy_nrf::peripherals::{RNG, SAADC, TWISPI0, USBD};
use embassy_nrf::saadc::{self, AnyInput, Input as _, Saadc};
use embassy_nrf::spim;
use embassy_nrf::usb::Driver;
use embassy_nrf::usb::vbus_detect::HardwareVbusDetect;
use embassy_nrf::{Peri, bind_interrupts, interrupt, pac, rng, usb};
use embassy_time::Timer;
use led_widget::{RgbLed, rgb_widget_task};
use matrix::KotsubuMatrix;
use nrf_mpsl::Flash;
use nrf_sdc::mpsl::MultiprotocolServiceLayer;
use nrf_sdc::{self as sdc, mpsl};
use panic_probe as _;
use rmk::ble::BleTransport;
use rmk::config::{BleBatteryConfig, DeviceConfig, RmkConfig, StorageConfig, VialConfig};
use rmk::embassy_futures::select::{Either, select};
use rmk::event::{BatteryStatusEvent, publish_event};
use rmk::host::HostService;
use rmk::keyboard::Keyboard;
use rmk::types::battery::{BatteryStatus, ChargeState};
use rmk::usb::UsbTransport;
use rmk::watchdog::Nrf52Watchdog;
use rmk::{KeymapData, initialize_keymap_and_storage, run_all};
use static_cell::StaticCell;
use vial::{VIAL_KEYBOARD_DEF, VIAL_KEYBOARD_ID};

bind_interrupts!(struct Irqs {
    USBD => usb::InterruptHandler<USBD>;
    RNG => rng::InterruptHandler<RNG>;
    SAADC => saadc::InterruptHandler;
    TWISPI0 => spim::InterruptHandler<TWISPI0>;
    EGU0_SWI0 => nrf_sdc::mpsl::LowPrioInterruptHandler;
    CLOCK_POWER => nrf_sdc::mpsl::ClockInterruptHandler, usb::vbus_detect::InterruptHandler;
    RADIO => nrf_sdc::mpsl::HighPrioInterruptHandler;
    TIMER0 => nrf_sdc::mpsl::HighPrioInterruptHandler;
    RTC0 => nrf_sdc::mpsl::HighPrioInterruptHandler;
});

#[embassy_executor::task]
async fn mpsl_task(mpsl: &'static MultiprotocolServiceLayer<'static>) -> ! {
    mpsl.run().await
}

#[embassy_executor::task]
async fn sleep_manager() -> ! {
    power::sleep_manager_task().await
}

#[embassy_executor::task]
async fn battery_manager(mut batt_enable: Output<'static>, mut saadc: Saadc<'static, 1>) -> ! {
    let mut activity_rx = power::ACTIVITY_WATCH.receiver().unwrap();
    let mut sample_buf = [0i16; 1];
    loop {
        batt_enable.set_low();
        Timer::after_micros(50).await;
        saadc.sample(&mut sample_buf).await;
        batt_enable.set_high();

        let percent = battery_percent_from_adc(sample_buf[0] as u16);
        publish_event(BatteryStatusEvent(BatteryStatus::Available {
            charge_state: ChargeState::Unknown,
            level: Some(percent),
        }));

        let wait_secs = power::battery_interval_secs();
        if wait_secs == power::BATTERY_ACTIVE_INTERVAL_SECS {
            Timer::after_secs(wait_secs).await;
        } else {
            match select(Timer::after_secs(wait_secs), activity_rx.changed()).await {
                Either::First(_) => {}
                Either::Second(_) => continue,
            }
        }
    }
}

const L2CAP_TXQ: u8 = 3;
const L2CAP_RXQ: u8 = 3;
const L2CAP_MTU: usize = 251;

/// Initializes the SAADC peripheral in single-ended mode on the given pin.
fn init_adc(adc_pin: AnyInput, adc: Peri<'static, SAADC>) -> Saadc<'static, 1> {
    let config = saadc::Config::default();
    let channel_cfg = saadc::ChannelConfig::single_ended(adc_pin.degrade_saadc());
    interrupt::SAADC.set_priority(interrupt::Priority::P3);
    saadc::Saadc::new(adc, Irqs, config, [channel_cfg])
}

fn build_sdc<'d, const N: usize>(
    p: nrf_sdc::Peripherals<'d>,
    rng: &'d mut rng::Rng<embassy_nrf::mode::Async>,
    mpsl: &'d MultiprotocolServiceLayer,
    mem: &'d mut sdc::Mem<N>,
) -> Result<nrf_sdc::SoftdeviceController<'d>, nrf_sdc::Error> {
    sdc::Builder::new()?
        .support_adv()
        .support_peripheral()
        .support_dle_peripheral()
        .support_phy_update_peripheral()
        .support_le_2m_phy()
        .peripheral_count(1)?
        .buffer_cfg(L2CAP_MTU as u16, L2CAP_MTU as u16, L2CAP_TXQ, L2CAP_RXQ)?
        .build(p, rng, mpsl, mem)
}

fn ble_addr() -> [u8; 6] {
    let ficr = pac::FICR;
    let high = u64::from(ficr.deviceid(1).read());
    let addr = high << 32 | u64::from(ficr.deviceid(0).read());
    let addr = addr | 0x0000_c000_0000_0000;
    unwrap!(addr.to_le_bytes()[..6].try_into())
}

fn battery_percent_from_adc(val: u16) -> u8 {
    let val = val as u32;
    // nRF52840 SAADC default: gain=1/6, ref=0.6V, 12-bit
    //   val = V_in * (1/6) / 0.6 * 4096 -> V_in = val * 3600 / 4096 (mV)
    // XIAO BLE voltage divider: V_bat = V_in * 1510 / 510
    //   V_bat_mv = val * 3600 / 4096 * 1510 / 510
    //            = val * 5436600 / 2088960
    //            ~= val * 2603 / 1000
    let v_bat_mv = val * 2603 / 1000;

    if v_bat_mv >= 4200 {
        100
    } else if v_bat_mv <= 3000 {
        0
    } else {
        ((v_bat_mv - 3000) * 100 / 1200) as u8
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("Starting Kotsubu RMK");

    let mut nrf_config = embassy_nrf::config::Config::default();
    nrf_config.dcdc.reg0_voltage = Some(embassy_nrf::config::Reg0Voltage::_3V3);
    nrf_config.dcdc.reg0 = true;
    nrf_config.dcdc.reg1 = true;
    let p = embassy_nrf::init(nrf_config);
    power::init();

    let reset_reasons = pac::POWER.resetreas().read().0;
    if reset_reasons != 0 {
        info!("Reset reasons: 0x{:08x}", reset_reasons);
        if reset_reasons & (1 << 5) != 0 {
            info!("Woke from System OFF");
        }
        pac::POWER.resetreas().write(|w| w.0 = reset_reasons);
    }

    let mpsl_p =
        mpsl::Peripherals::new(p.RTC0, p.TIMER0, p.TEMP, p.PPI_CH19, p.PPI_CH30, p.PPI_CH31);
    let lfclk_cfg = mpsl::raw::mpsl_clock_lfclk_cfg_t {
        source: mpsl::raw::MPSL_CLOCK_LF_SRC_RC as u8,
        rc_ctiv: mpsl::raw::MPSL_RECOMMENDED_RC_CTIV as u8,
        rc_temp_ctiv: mpsl::raw::MPSL_RECOMMENDED_RC_TEMP_CTIV as u8,
        accuracy_ppm: mpsl::raw::MPSL_DEFAULT_CLOCK_ACCURACY_PPM as u16,
        skip_wait_lfclk_started: mpsl::raw::MPSL_DEFAULT_SKIP_WAIT_LFCLK_STARTED != 0,
    };
    static MPSL: StaticCell<MultiprotocolServiceLayer> = StaticCell::new();
    static SESSION_MEM: StaticCell<mpsl::SessionMem<1>> = StaticCell::new();
    let mpsl = MPSL.init(unwrap!(mpsl::MultiprotocolServiceLayer::with_timeslots(
        mpsl_p,
        Irqs,
        lfclk_cfg,
        SESSION_MEM.init(mpsl::SessionMem::new())
    )));
    spawner.spawn(unwrap!(mpsl_task(&*mpsl)));

    let sdc_p = sdc::Peripherals::new(
        p.PPI_CH17, p.PPI_CH18, p.PPI_CH20, p.PPI_CH21, p.PPI_CH22, p.PPI_CH23, p.PPI_CH24,
        p.PPI_CH25, p.PPI_CH26, p.PPI_CH27, p.PPI_CH28, p.PPI_CH29,
    );
    let mut rng = rng::Rng::new(p.RNG, Irqs);
    let mut sdc_mem = sdc::Mem::<4696>::new();
    let sdc = unwrap!(build_sdc(sdc_p, &mut rng, mpsl, &mut sdc_mem));

    let driver = Driver::new(p.USBD, Irqs, HardwareVbusDetect::new(Irqs));
    let flash = Flash::take(mpsl, p.NVMC);

    // Initialize the ADC for battery level detection
    // Keep P0.14 High (disabled) during calibration to avoid driving the ADC pin
    let mut batt_enable = Output::new(p.P0_14, Level::High, OutputDrive::Standard);
    let adc_pin = p.P0_31.degrade_saadc();
    let saadc = init_adc(adc_pin, p.SAADC);
    saadc.calibrate().await; // Wait for ADC calibration

    // Leave the voltage divider disabled until a sample is requested.
    batt_enable.set_high();

    let keyboard_device_config = DeviceConfig {
        vid: board_config::USB_VENDOR_ID,
        pid: board_config::USB_PRODUCT_ID,
        manufacturer: board_config::USB_MANUFACTURER,
        product_name: board_config::USB_PRODUCT_NAME,
        serial_number: board_config::USB_SERIAL_NUMBER,
    };
    let vial_config = VialConfig::new(
        VIAL_KEYBOARD_ID,
        VIAL_KEYBOARD_DEF,
        board_config::VIAL_UNLOCK_KEYS,
    );
    let storage_config = StorageConfig {
        start_addr: board_config::STORAGE_START_ADDR,
        num_sectors: board_config::STORAGE_NUM_SECTORS,
        ..Default::default()
    };
    let rmk_config = RmkConfig {
        device_config: keyboard_device_config,
        vial_config,
        ble_battery_config: BleBatteryConfig::default(),
        storage_config,
    };

    let mut keymap_data = KeymapData::new(keymap::get_default_keymap());
    let key_config = keymap::get_positional_config();
    let mut behavior_config = keymap::get_behavior_config();
    let (keymap, mut storage) = initialize_keymap_and_storage(
        &mut keymap_data,
        flash,
        &storage_config,
        &mut behavior_config,
        &key_config,
    )
    .await;

    let mut shift_spi_config = spim::Config::default();
    shift_spi_config.frequency = spim::Frequency::M4;
    shift_spi_config.mode = spim::MODE_0;
    let shift_spi = spim::Spim::new_txonly(p.TWISPI0, Irqs, p.P1_13, p.P1_15, shift_spi_config);

    let mut matrix = KotsubuMatrix::new(
        p.P0_28, p.P0_29, p.P0_04, p.P0_05, p.P0_02, p.P0_03, shift_spi, p.P1_12,
    );
    let mut keyboard = Keyboard::new(&keymap);

    let rgb_led = RgbLed::new(
        embassy_nrf::gpio::Output::new(
            p.P0_26,
            embassy_nrf::gpio::Level::High,
            embassy_nrf::gpio::OutputDrive::Standard,
        ),
        embassy_nrf::gpio::Output::new(
            p.P0_30,
            embassy_nrf::gpio::Level::High,
            embassy_nrf::gpio::OutputDrive::Standard,
        ),
        embassy_nrf::gpio::Output::new(
            p.P0_06,
            embassy_nrf::gpio::Level::High,
            embassy_nrf::gpio::OutputDrive::Standard,
        ),
    );
    spawner.spawn(unwrap!(rgb_widget_task(rgb_led)));
    spawner.spawn(unwrap!(sleep_manager()));
    spawner.spawn(unwrap!(battery_manager(batt_enable, saadc)));

    let host_service = HostService::new(&keymap, &rmk_config);
    let mut usb_transport =
        UsbTransport::new(driver, rmk_config.device_config).with_host_service(&host_service);
    let mut ble_transport =
        BleTransport::new(sdc, ble_addr(), rmk_config).with_host_service(&host_service);
    let mut watchdog_runner = Nrf52Watchdog::default_runner(p.WDT);

    run_all!(
        matrix,
        storage,
        usb_transport,
        ble_transport,
        keyboard,
        watchdog_runner
    )
    .await;
}
