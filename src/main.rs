use anyhow::Result;
use embedded_graphics::geometry::OriginDimensions;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rectangle;
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
use esp_idf_svc::http::client::{Configuration as HttpConfig, EspHttpConnection};
use esp_idf_svc::http::Method;
use esp_idf_svc::log::EspLogger;
use esp_idf_svc::nvs::EspDefaultNvsPartition;
use esp_idf_svc::sys::link_patches;
use esp_idf_svc::tls::X509;
use esp_idf_svc::wifi::{
    AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi,
};
use mipidsi::interface::SpiInterface;
use mipidsi::options::{ColorOrder, Orientation, Rotation};
use mipidsi::{models::ILI9341Rgb565, Builder};
use profont::{PROFONT_12_POINT, PROFONT_24_POINT};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::Duration;

const SPOT_URL: &str = "https://api.coinbase.com/v2/prices/BTC-USD/spot";

// Self-signed GTS Root R1 (pki.goog), the trust anchor for the RSA chain
// served to RSA-only TLS clients by api.coinbase.com. The file MUST keep its
// trailing NUL byte: X509::pem_until_nul scans for it.
const COINBASE_ROOT_CA: &[u8] = include_bytes!("../certs/coinbase-root-ca.pem");

/// Logical (rotated) display width in pixels: 320x240 landscape.
const LOGICAL_WIDTH: i32 = 320;

const REFRESH_INTERVAL: Duration = Duration::from_secs(10);

// --- 6.1 shared application state ---

#[derive(Clone, Debug, PartialEq, Eq)]
enum AppState {
    /// Booting / no price fetched yet.
    Waiting,
    /// Latest successful fetch, ready-to-render string (e.g. "$67,432").
    Price { display: String },
    /// Last fetch failed (and, for rendering, no price is known).
    Error,
}

type SharedState = Arc<Mutex<AppState>>;

fn lock_state(state: &SharedState) -> MutexGuard<'_, AppState> {
    state.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

fn check<T, E: core::fmt::Debug>(r: Result<T, E>) -> anyhow::Result<T> {
    r.map_err(|e| anyhow::anyhow!("{e:?}"))
}

/// Connect in station mode, retrying forever until the network is up.
/// Credentials are baked in at build time via the WIFI_SSID / WIFI_PASS env
/// vars (see .cargo/config.toml).
fn connect_wifi_with_retries(wifi: &mut BlockingWifi<EspWifi<'static>>) -> Result<()> {
    wifi.start()?;
    loop {
        match wifi.connect().and_then(|_| wifi.wait_netif_up()) {
            Ok(_) => {
                let ip = wifi.wifi().sta_netif().get_ip_info()?.ip;
                log::info!("WiFi connected! IP: {ip}");
                return Ok(());
            }
            Err(e) => {
                log::warn!("WiFi connect failed ({e:?}); retrying in 5 s");
                let _ = wifi.disconnect();
                FreeRtos::delay_ms(5000);
            }
        }
    }
}

/// One HTTPS GET of the Coinbase spot price. Returns the raw response body.
/// TLS is handled by ESP-IDF's mbedTLS, validating the server chain against
/// the embedded GTS Root R4 anchor.
fn fetch_price_raw() -> Result<String> {
    let mut conn = EspHttpConnection::new(&HttpConfig {
        server_certificate: Some(X509::pem_until_nul(COINBASE_ROOT_CA)),
        // The Wokwi sim runs the emulated CPU far slower than real time; the
        // TLS handshake alone takes ~20-30 sim-seconds. 60 s keeps the sim
        // happy while remaining a sane timeout on hardware.
        timeout: Some(core::time::Duration::from_secs(60)),
        ..Default::default()
    })?;

    conn.initiate_request(Method::Get, SPOT_URL, &[("User-Agent", "sp32-ticker")])?;
    conn.initiate_response()?;

    let status = conn.status();
    if !(200..300).contains(&status) {
        anyhow::bail!("HTTP {status} from {SPOT_URL}");
    }

    let mut body = Vec::new();
    let mut buf = [0u8; 512];
    loop {
        let n = conn.read(&mut buf)?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&buf[..n]);
        if body.len() > 4096 {
            anyhow::bail!("response too large");
        }
    }
    Ok(String::from_utf8_lossy(&body).into_owned())
}

// --- 6.2 fetch task: fetch -> update state -> sleep 10 s -> repeat ---

fn fetch_loop(state: SharedState) {
    loop {
        match fetch_price_raw()
            .and_then(|body| price::parse_spot_response(&body).map_err(|e| anyhow::anyhow!("{e}")))
        {
            Ok(value) => {
                let display = price::format_price(value);
                log::info!("BTC/USD spot: {display}");
                *lock_state(&state) = AppState::Price { display };
            }
            Err(e) => {
                log::error!("Price update failed: {e:?}");
                *lock_state(&state) = AppState::Error;
            }
        }
        thread::sleep(REFRESH_INTERVAL);
    }
}

// --- 6.3 UI task: full-frame redraws from shared state ---

/// Draw-target wrapper that doubles every pixel, turning the 24pt font into
/// an effective 48pt for the big price (profont has no larger sizes).
struct Scaled2x<'a, D>(&'a mut D);

impl<D> OriginDimensions for Scaled2x<'_, D>
where
    D: DrawTarget<Color = Rgb565>,
{
    fn size(&self) -> Size {
        self.0.bounding_box().size
    }
}

impl<D> DrawTarget for Scaled2x<'_, D>
where
    D: DrawTarget<Color = Rgb565>,
    D::Error: core::fmt::Debug,
{
    type Color = Rgb565;
    type Error = D::Error;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<Self::Color>>,
    {
        self.0.draw_iter(pixels.into_iter().flat_map(|Pixel(p, c)| {
            let (x, y) = (p.x * 2, p.y * 2);
            [
                Point::new(x, y),
                Point::new(x + 1, y),
                Point::new(x, y + 1),
                Point::new(x + 1, y + 1),
            ]
            .into_iter()
            .map(move |p| Pixel(p, c))
        }))
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Self::Color) -> Result<(), Self::Error> {
        self.0.fill_solid(
            &Rectangle::new(
                Point::new(area.top_left.x * 2, area.top_left.y * 2),
                area.size * 2,
            ),
            color,
        )
    }
}

fn draw_centered<D>(display: &mut D, text: &str, y: i32, style: MonoTextStyle<Rgb565>)
where
    D: DrawTarget<Color = Rgb565>,
    D::Error: core::fmt::Debug,
{
    let char_w = style.font.character_size.width as i32;
    let x = (LOGICAL_WIDTH - text.len() as i32 * char_w) / 2;
    Text::with_baseline(text, Point::new(x.max(0), y), style, Baseline::Top)
        .draw(display)
        .map_err(|e| anyhow::anyhow!("{e:?}"))
        .unwrap();
}

/// 48pt text (24pt doubled), centered, at `visual_y` in real pixels.
fn draw_big_centered<D>(display: &mut D, text: &str, visual_y: i32, style: MonoTextStyle<Rgb565>)
where
    D: DrawTarget<Color = Rgb565>,
    D::Error: core::fmt::Debug,
{
    let char_w = style.font.character_size.width as i32;
    let visual_w = text.len() as i32 * char_w * 2;
    let visual_x = ((LOGICAL_WIDTH - visual_w) / 2).max(0);
    let mut scaled = Scaled2x(display);
    Text::with_baseline(
        text,
        Point::new(visual_x / 2, visual_y / 2),
        style,
        Baseline::Top,
    )
    .draw(&mut scaled)
    .map_err(|e| anyhow::anyhow!("{e:?}"))
    .unwrap();
}

fn draw_frame<D>(display: &mut D, frame: &AppState)
where
    D: DrawTarget<Color = Rgb565>,
    D::Error: core::fmt::Debug,
{
    check(display.clear(Rgb565::BLACK)).unwrap();
    let label = MonoTextStyle::new(&PROFONT_12_POINT, Rgb565::WHITE);
    match frame {
        AppState::Waiting => {
            draw_centered(display, "BTC / USD", 40, label);
            draw_centered(display, "Waiting for first price...", 110, label);
        }
        AppState::Price { display: price } => {
            draw_centered(display, "BTC / USD", 40, label);
            draw_big_centered(
                display,
                price,
                96,
                MonoTextStyle::new(&PROFONT_24_POINT, Rgb565::YELLOW),
            );
        }
        AppState::Error => {
            draw_big_centered(
                display,
                "ERROR",
                88,
                MonoTextStyle::new(&PROFONT_24_POINT, Rgb565::RED),
            );
            draw_centered(display, "API unreachable, retrying...", 150, label);
        }
    }
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
    log::info!("sp32-demo1 starting; WiFi SSID configured: '{ssid}'");

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

    connect_wifi_with_retries(&mut wifi)?;

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
        &DriverConfig::new().dma(Dma::Auto(512)),
        &SpiConfig::new().baudrate(Hertz(8_000_000)),
    )?;

    let dc = PinDriver::output(peripherals.pins.gpio2)?;

    let mut buffer = [0u8; 512];
    let di = SpiInterface::new(spi, dc, &mut buffer);

    let mut display = Builder::new(ILI9341Rgb565, di)
        .reset_pin(&mut rst)
        .color_order(ColorOrder::Bgr)
        .orientation(Orientation::new().rotate(Rotation::Deg270).flip_horizontal())
        .init(&mut FreeRtos)
        .map_err(|e| anyhow::anyhow!("display init failed: {e:?}"))?;

    log::info!("Display initialized");

    // 6.1: shared state between the fetch and UI tasks
    let state: SharedState = Arc::new(Mutex::new(AppState::Waiting));

    // 6.2: fetch task on its own thread (TLS handshake needs stack headroom)
    let fetch_state = state.clone();
    thread::Builder::new()
        .stack_size(16 * 1024)
        .spawn(move || fetch_loop(fetch_state))
        .expect("spawn fetch task");

    // 6.3: UI task on the main thread: redraw only on state change; if a
    // fetch fails after a price is known, keep showing the last known price
    let mut last_drawn: Option<AppState> = None;
    loop {
        let current = lock_state(&state).clone();
        let target = match &current {
            AppState::Waiting => Some(AppState::Waiting),
            AppState::Price { .. } => Some(current.clone()),
            AppState::Error => match &last_drawn {
                Some(AppState::Price { .. }) => None,
                _ => Some(AppState::Error),
            },
        };
        if target.is_some() && target != last_drawn {
            draw_frame(&mut display, target.as_ref().unwrap());
            last_drawn = target;
        }
        FreeRtos::delay_ms(200);
    }
}
