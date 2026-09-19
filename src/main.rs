use anyhow::Result;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
use embedded_graphics::text::{Baseline, Text};
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::gpio::PinDriver;
use esp_idf_svc::hal::peripherals::Peripherals;
use esp_idf_svc::hal::spi::{
    config::{Config as SpiConfig, DriverConfig},
    Dma, SpiDeviceDriver,
};
use esp_idf_svc::hal::units::Hertz;
use esp_idf_svc::log::EspLogger;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::sys::link_patches;
use esp_idf_svc::wifi::{
    AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi,
};
use mipidsi::interface::SpiInterface;
use mipidsi::options::{Orientation, Rotation};
use mipidsi::{models::ILI9341Rgb565, Builder};
use profont::{PROFONT_12_POINT, PROFONT_24_POINT};

fn check<T, E: core::fmt::Debug>(r: Result<T, E>) -> anyhow::Result<T> {
    r.map_err(|e| anyhow::anyhow!("{e:?}"))
}

fn main() -> Result<()> {
    link_patches();
    EspLogger::initialize_default();

    log::info!("sp32-demo1 starting");

    let peripherals = Peripherals::take()?;
    let sys_loop = EspSystemEventLoop::take()?;
    let nvs = EspDefaultNvsPartition::take()?;

    // --- WiFi (station mode, credentials from build-time env) ---
    let wifi = EspWifi::new(peripherals.modem, sys_loop.clone(), Some(nvs))?;
    let mut wifi = BlockingWifi::wrap(wifi, sys_loop)?;

    let ssid = env!("WIFI_SSID");
    let pass = env!("WIFI_PASS");
    log::info!("Connecting to WiFi '{}'...", ssid);

    wifi.set_configuration(&Configuration::Client(ClientConfiguration {
        ssid: ssid.try_into().expect("SSID too long"),
        password: pass.try_into().expect("WiFi password too long"),
        auth_method: if pass.is_empty() {
            AuthMethod::None
        } else {
            AuthMethod::WPA2Personal
        },
        ..Default::default()
    }))?;

    wifi.start()?;
    wifi.connect()?;
    wifi.wait_netif_up()?;

    let ip_info = wifi.wifi().sta_netif().get_ip_info()?;
    log::info!("WiFi connected! IP: {}", ip_info.ip);

    // --- ILI9341 over VSPI (SCK=18, MOSI=23, CS=5, DC=2, RST=4) ---
    let mut backlight = PinDriver::output(peripherals.pins.gpio21)?;
    backlight.set_high()?;

    let mut rst = PinDriver::output(peripherals.pins.gpio4)?;

    let spi = SpiDeviceDriver::new_single(
        peripherals.spi2,
        peripherals.pins.gpio18,
        peripherals.pins.gpio23,
        Option::<esp_idf_svc::hal::gpio::Gpio19>::None,
        Some(peripherals.pins.gpio5),
        &DriverConfig::new().dma(Dma::Auto(0)),
        &SpiConfig::new().baudrate(Hertz(8_000_000)),
    )?;

    let dc = PinDriver::output(peripherals.pins.gpio2)?;

    let mut buffer = [0u8; 512];
    let di = SpiInterface::new(spi, dc, &mut buffer);

    let mut display = Builder::new(ILI9341Rgb565, di)
        .reset_pin(&mut rst)
        .orientation(Orientation::new().rotate(Rotation::Deg90))
        .init(&mut FreeRtos)
        .map_err(|e| anyhow::anyhow!("display init failed: {e:?}"))?;

    check(display.clear(Rgb565::BLACK))?;

    // Color bars: R, G, B, white — verifies color order and pixel mapping
    let bar_w = 320 / 4;
    for (i, color) in [
        Rgb565::RED,
        Rgb565::GREEN,
        Rgb565::BLUE,
        Rgb565::WHITE,
    ]
    .into_iter()
    .enumerate()
    {
        check(
            Rectangle::new(
                Point::new((i * bar_w) as i32, 150),
                Size::new(bar_w as u32, 40),
            )
            .into_styled(PrimitiveStyle::with_fill(color))
            .draw(&mut display),
        )?;
    }

    check(
        Text::with_baseline(
            "BTC/USD",
            Point::new(24, 60),
            MonoTextStyle::new(&PROFONT_12_POINT, Rgb565::WHITE),
            Baseline::Top,
        )
        .draw(&mut display),
    )?;

    check(
        Text::with_baseline(
            "$67,432",
            Point::new(24, 110),
            MonoTextStyle::new(&PROFONT_24_POINT, Rgb565::YELLOW),
            Baseline::Top,
        )
        .draw(&mut display),
    )?;

    log::info!("Display initialized, test pattern drawn");

    loop {
        FreeRtos::delay_ms(1000);
    }
}
