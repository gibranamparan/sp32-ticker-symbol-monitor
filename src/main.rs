use anyhow::Result;
use embedded_graphics::geometry::OriginDimensions;
use embedded_graphics::mono_font::MonoTextStyle;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};
use embedded_graphics::text::{Baseline, Text};
use epd_waveshare::epd2in7_v2::*;
use epd_waveshare::prelude::{Color, DisplayRotation, WaveshareDisplay};
use esp_idf_svc::eventloop::EspSystemEventLoop;
use esp_idf_svc::hal::delay::FreeRtos;
use esp_idf_svc::hal::gpio::{PinDriver, Pull};
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
use esp_idf_svc::sntp::EspSntp;
use esp_idf_svc::sys::link_patches;
use esp_idf_svc::tls::X509;
use esp_idf_svc::wifi::{
    AuthMethod, BlockingWifi, ClientConfiguration, Configuration, EspWifi,
};
use profont::{PROFONT_12_POINT, PROFONT_24_POINT};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread;
use std::time::Duration;

mod logo;

const SPOT_URL: &str = "https://api.coinbase.com/v2/prices/BTC-USD/spot";

// Self-signed GTS Root R1 (pki.goog), the trust anchor for the RSA chain
// served to RSA-only TLS clients by api.coinbase.com. The file MUST keep its
// trailing NUL byte: X509::pem_until_nul scans for it.
const COINBASE_ROOT_CA: &[u8] = include_bytes!("../certs/coinbase-root-ca.pem");

/// Logical (rotated) display width in pixels: 264x176 landscape.
const LOGICAL_WIDTH: i32 = 264;

// Ticker frame geometry (three equal-height rows), mirroring wireframes/wireframe.html.
const FRAME_HEIGHT: i32 = 176;
const ROW_HEIGHT: i32 = FRAME_HEIGHT / 3;
const LOGO_LABEL_GAP: i32 = 10;
const BOTTOM_INSET: i32 = 4;
/// The timestamp is inset further so the last digit never clips.
const TIMESTAMP_RIGHT_INSET: i32 = 20;
/// Pair label shown next to the logo (row 1) and on the waiting screen.
const PAIR_LABEL: &str = "BTC/USD";
/// Display timezone as a UTC offset (e.g. `UTC-6`, `UTC+5:30`); required at
/// build time. SNTP keeps the clock in UTC and this offset is applied for
/// display only.
const TIMEZONE: &str = env!("TIMEZONE");

/// Epoch seconds above which the clock counts as synchronised (2020-09-13);
/// before SNTP sync the clock sits near 1970.
const CLOCK_SYNCED_EPOCH: i64 = 1_600_000_000;

/// Seconds since the Unix epoch, or `None` while the clock is unsynchronised.
fn now_epoch() -> Option<i64> {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs() as i64;
    (secs > CLOCK_SYNCED_EPOCH).then_some(secs)
}

/// Price refresh cadence, baked in at build time (FETCH_INTERVAL_SECS env,
/// default 10 s — see .cargo/config.toml).
fn refresh_interval() -> Duration {
    let secs: u64 = env!("FETCH_INTERVAL_SECS")
        .parse()
        .expect("FETCH_INTERVAL_SECS must be an integer number of seconds");
    assert!(secs > 0, "FETCH_INTERVAL_SECS must be > 0");
    Duration::from_secs(secs)
}

// --- 6.1 shared application state ---

#[derive(Clone, Debug, PartialEq, Eq)]
enum AppState {
    /// Booting / no price fetched yet.
    Waiting,
    /// Latest successful fetch; `updated_at` is the epoch second of that fetch,
    /// or `None` while the device clock is not yet synchronised.
    Price {
        display: String,
        updated_at: Option<i64>,
    },
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
/// the embedded GTS Root R1 anchor.
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

fn fetch_loop(state: SharedState, tz_offset_secs: i32) {
    loop {
        match fetch_price_raw()
            .and_then(|body| price::parse_spot_response(&body).map_err(|e| anyhow::anyhow!("{e}")))
        {
            Ok(value) => {
                let display = price::format_price(value);
                let updated_at = now_epoch();
                let stamp = updated_at
                    .map(|epoch| price::format_timestamp(epoch + tz_offset_secs as i64))
                    .unwrap_or_else(|| "--".to_string());
                log::info!(
                    "BTC/USD spot: {display} at {stamp} (clock synced: {})",
                    updated_at.is_some()
                );
                *lock_state(&state) = AppState::Price {
                    display,
                    updated_at,
                };
            }
            Err(e) => {
                log::error!("Price update failed: {e:?}");
                *lock_state(&state) = AppState::Error;
            }
        }
        thread::sleep(refresh_interval());
    }
}

// --- 6.3 UI task: full-frame redraws from shared state ---

/// Draw-target wrapper that doubles every pixel, turning the 24pt font into
/// an effective 48pt for the big price (profont has no larger sizes).
struct Scaled2x<'a, D>(&'a mut D);

impl<D> OriginDimensions for Scaled2x<'_, D>
where
    D: DrawTarget,
{
    fn size(&self) -> Size {
        self.0.bounding_box().size
    }
}

impl<D> DrawTarget for Scaled2x<'_, D>
where
    D: DrawTarget,
    D::Error: core::fmt::Debug,
{
    type Color = D::Color;
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

fn draw_centered<D>(display: &mut D, text: &str, y: i32, style: MonoTextStyle<Color>)
where
    D: DrawTarget<Color = Color>,
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
fn draw_big_centered<D>(display: &mut D, text: &str, visual_y: i32, style: MonoTextStyle<Color>)
where
    D: DrawTarget<Color = Color>,
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

/// Draws the 1-bit logo bitmap with its top-left corner at (`x`, `y`).
fn draw_logo<D>(display: &mut D, x: i32, y: i32)
where
    D: DrawTarget<Color = Color>,
    D::Error: core::fmt::Debug,
{
    let bytes_per_row = ((logo::LOGO_WIDTH + 7) / 8) as usize;
    let pixels = (0..logo::LOGO_HEIGHT).flat_map(move |row| {
        (0..logo::LOGO_WIDTH).filter_map(move |col| {
            let byte = logo::LOGO_BITMAP[row as usize * bytes_per_row + (col / 8) as usize];
            let black = byte & (0x80 >> (col % 8)) != 0;
            black.then(|| Pixel(Point::new(x + col as i32, y + row as i32), Color::Black))
        })
    });
    display
        .draw_iter(pixels)
        .map_err(|e| anyhow::anyhow!("{e:?}"))
        .unwrap();
}

/// Text right-aligned to `right_edge` with its top at `y`.
fn draw_right_aligned<D>(
    display: &mut D,
    text: &str,
    right_edge: i32,
    y: i32,
    style: MonoTextStyle<Color>,
) where
    D: DrawTarget<Color = Color>,
    D::Error: core::fmt::Debug,
{
    let char_w = style.font.character_size.width as i32;
    let x = (right_edge - text.len() as i32 * char_w).max(0);
    Text::with_baseline(text, Point::new(x, y), style, Baseline::Top)
        .draw(display)
        .map_err(|e| anyhow::anyhow!("{e:?}"))
        .unwrap();
}

fn draw_frame<D>(display: &mut D, frame: &AppState, tz_offset_secs: i32)
where
    D: DrawTarget<Color = Color>,
    D::Error: core::fmt::Debug,
{
    // Full-frame redraw on a white background (e-paper is monochrome).
    check(display.fill_solid(&display.bounding_box(), Color::White)).unwrap();
    let body = MonoTextStyle::new(&PROFONT_12_POINT, Color::Black);

    match frame {
        AppState::Waiting => {
            draw_centered(display, PAIR_LABEL, 30, body);
            draw_centered(display, "Waiting for first price...", 90, body);
        }
        AppState::Price {
            display: price_text,
            updated_at,
        } => {
            // Row 1: logo + pair label (h2), centered as a group
            let pair = MonoTextStyle::new(&PROFONT_24_POINT, Color::Black);
            let pair_h = pair.font.character_size.height as i32;
            let pair_w = PAIR_LABEL.len() as i32 * pair.font.character_size.width as i32;
            let group_w = logo::LOGO_WIDTH as i32 + LOGO_LABEL_GAP + pair_w;
            let group_x = (LOGICAL_WIDTH - group_w) / 2;
            let logo_y = (ROW_HEIGHT - logo::LOGO_HEIGHT as i32) / 2;
            draw_logo(display, group_x, logo_y);
            check(
                Text::with_baseline(
                    PAIR_LABEL,
                    Point::new(group_x + logo::LOGO_WIDTH as i32 + LOGO_LABEL_GAP, (ROW_HEIGHT - pair_h) / 2),
                    pair,
                    Baseline::Top,
                )
                .draw(display),
            )
            .unwrap();

            // Row 2: price (h1, 24pt doubled), bottom-aligned in its row
            let price_h = 2 * PROFONT_24_POINT.character_size.height as i32;
            draw_big_centered(
                display,
                price_text,
                2 * ROW_HEIGHT - price_h,
                MonoTextStyle::new(&PROFONT_24_POINT, Color::Black),
            );

            // Row 3: last update in the configured timezone, end- and bottom-aligned
            let stamp = match updated_at {
                Some(epoch) => price::format_timestamp(*epoch + tz_offset_secs as i64),
                None => "--".to_string(),
            };
            let stamp_h = body.font.character_size.height as i32;
            draw_right_aligned(
                display,
                &stamp,
                LOGICAL_WIDTH - TIMESTAMP_RIGHT_INSET,
                FRAME_HEIGHT - stamp_h - BOTTOM_INSET,
                body,
            );
        }
        AppState::Error => {
            draw_big_centered(
                display,
                "ERROR",
                60,
                MonoTextStyle::new(&PROFONT_24_POINT, Color::Black),
            );
            draw_centered(display, "API unreachable, retrying...", 130, body);
        }
    }
}

fn main() -> Result<()> {
    link_patches();
    EspLogger::initialize_default();

    log::info!("sp32-demo1 starting");

    // Fail fast on a bad display timezone: the offset is required configuration.
    let tz_offset_secs = price::parse_utc_offset(TIMEZONE)
        .unwrap_or_else(|e| panic!("TIMEZONE invalid: {e}"));
    log::info!("Timezone: {TIMEZONE} ({tz_offset_secs} s from UTC)");

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

    // Wall-clock time for the on-screen "last update" timestamp (UTC). The
    // service re-syncs periodically on its own; before the first sync the UI
    // shows a placeholder rather than a fabricated time.
    let _sntp = EspSntp::new_default()?;
    log::info!("SNTP started");

    // --- GDEY027T91 e-paper on the driver board's fixed e-paper wiring:
    // SCK=GPIO13, MOSI=GPIO14, CS=GPIO15, DC=GPIO27, RST=GPIO26, BUSY=GPIO25 ---
    let mut spi = SpiDeviceDriver::new_single(
        peripherals.spi2,
        peripherals.pins.gpio13,
        peripherals.pins.gpio14,
        Option::<esp_idf_svc::hal::gpio::Gpio12>::None,
        Some(peripherals.pins.gpio15),
        &DriverConfig::new().dma(Dma::Disabled),
        &SpiConfig::new().baudrate(Hertz(8_000_000)),
    )?;

    let busy = PinDriver::input(peripherals.pins.gpio25, Pull::Floating)?;
    let dc = PinDriver::output(peripherals.pins.gpio27)?;
    let rst = PinDriver::output(peripherals.pins.gpio26)?;
    let mut delay = FreeRtos;

    let mut epd = Epd2in7::new(&mut spi, busy, dc, rst, &mut delay, None)
        .map_err(|e| anyhow::anyhow!("e-paper init failed: {e:?}"))?;

    // The framebuffer (176/8 * 264 = 5808 bytes) lives on the heap: keeping it
    // inline on main's 8 KB stack overflows it at function entry.
    let mut display = Box::new(Display2in7::default());
    display.set_rotation(DisplayRotation::Rotate90);
    log::info!(
        "E-paper display initialized ({}x{})",
        display.size().width,
        display.size().height
    );

    // PANEL_TEST=1 build: alternate black/white stripes every 5 s to prove the
    // panel can update, one full refresh per cycle
    if env!("PANEL_TEST") == "1" {
        log::info!("PANEL TEST: alternating stripes every 5 s");
        let mut phase = false;
        loop {
            for (row, y) in (0..176).step_by(22).enumerate() {
                let black = (row % 2 == 0) != phase;
                let color = if black { Color::Black } else { Color::White };
                check(
                    Rectangle::new(Point::new(0, y), Size::new(264, 22))
                        .into_styled(PrimitiveStyle::with_fill(color))
                        .draw(&mut *display),
                )?;
            }
            check(epd.update_frame(&mut spi, display.buffer(), &mut delay))?;
            check(epd.display_frame(&mut spi, &mut delay))?;
            log::info!("PANEL TEST: phase {phase} displayed");
            phase = !phase;
            FreeRtos::delay_ms(5000);
        }
    }

    // 6.1: shared state between the fetch and UI tasks
    let state: SharedState = Arc::new(Mutex::new(AppState::Waiting));

    // 6.2: fetch task on its own thread (TLS handshake needs stack headroom)
    let fetch_state = state.clone();
    thread::Builder::new()
        .stack_size(16 * 1024)
        .spawn(move || fetch_loop(fetch_state, tz_offset_secs))
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
            draw_frame(&mut *display, target.as_ref().unwrap(), tz_offset_secs);
            check(epd.update_frame(&mut spi, display.buffer(), &mut delay))?;
            check(epd.display_frame(&mut spi, &mut delay))?;
            log::info!("E-paper refreshed");
            last_drawn = target;
        }
        FreeRtos::delay_ms(200);
    }
}
