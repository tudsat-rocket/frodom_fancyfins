use defmt::error;
use embassy_executor;
use embassy_stm32::spi::{Spi, Config, Phase, Polarity};
use embassy_stm32::time::Hertz;
use embassy_stm32::peripherals::*;
use embassy_time::{Duration, Timer};
use shared_types::FlightMode;
use smart_leds::RGB8;
use ws2812_spi::Ws2812;
use smart_leds::SmartLedsWrite;

const NAVIGATION_LIGHT_PATTERN: [RGB8; 16] = [
    RGB8 { r: 0xff, g: 0x00, b: 0x00 },
    RGB8 { r: 0x00, g: 0xff, b: 0x00 },
    RGB8 { r: 0xff, g: 0x00, b: 0x00 },
    RGB8 { r: 0x00, g: 0xff, b: 0x00 },
    RGB8 { r: 0xff, g: 0x00, b: 0x00 },
    RGB8 { r: 0x00, g: 0xff, b: 0x00 },
    RGB8 { r: 0xff, g: 0x00, b: 0x00 },
    RGB8 { r: 0x00, g: 0xff, b: 0x00 },
    RGB8 { r: 0xff, g: 0x00, b: 0x00 },
    RGB8 { r: 0x00, g: 0xff, b: 0x00 },
    RGB8 { r: 0xff, g: 0x00, b: 0x00 },
    RGB8 { r: 0x00, g: 0xff, b: 0x00 },
    RGB8 { r: 0xff, g: 0x00, b: 0x00 },
    RGB8 { r: 0x00, g: 0xff, b: 0x00 },
    RGB8 { r: 0xff, g: 0x00, b: 0x00 },
    RGB8 { r: 0x00, g: 0xff, b: 0x00 },
];
const NAVIGATION_LIGHT_BLINK_DURATION_MILLIS: u64 = 50;

async fn boot_animation(leds: &mut Ws2812<Spi<'static, SPI3, DMA1_CH1, DMA1_CH2>>) {
    let mut colors = [RGB8::default(); 16];

    for i in 0..16 {
        Timer::after(Duration::from_millis(20)).await;
        colors[i] = RGB8 { r: 0x00, g: 0xff, b: 0x00 };
        let _ = leds.write(colors);
    }

    Timer::after(Duration::from_millis(200)).await;
    let _ = leds.write([RGB8::default(); 16]);
    Timer::after(Duration::from_millis(200)).await;
}

#[embassy_executor::task]
pub async fn run(
    mut flight_mode_subscriber: crate::can::FlightModeSubscriber,
    spi: SPI3,
    led_signal_pin: PB5,
    dma_out: DMA1_CH1,
    dma_in: DMA1_CH2,
) -> ! {
    let mut flight_mode = FlightMode::default();

    let mut config = Config::default();
    config.frequency = Hertz::khz(3500);
    config.mode.polarity = Polarity::IdleLow;
    config.mode.phase = Phase::CaptureOnSecondTransition;
    let spi_bus = embassy_stm32::spi::Spi::new_txonly_nosck(spi, led_signal_pin, dma_out, dma_in, config);

    let mut leds = Ws2812::new(spi_bus);

    boot_animation(&mut leds).await;

    loop {
        if let Some(new_fm) = flight_mode_subscriber.try_next_message_pure() {
            defmt::println!("{:?}", defmt::Debug2Format(&new_fm));
            flight_mode = new_fm;
        }

        if flight_mode >= FlightMode::Burn {
            if let Err(_e) = leds.write(NAVIGATION_LIGHT_PATTERN) {
                error!("Failed to write LED pattern");
            }

            Timer::after(Duration::from_millis(NAVIGATION_LIGHT_BLINK_DURATION_MILLIS)).await;

            if let Err(_e) = leds.write([RGB8::default(); 16]) {
                error!("Failed to write LED pattern");
            }

            Timer::after(Duration::from_millis(1000 - NAVIGATION_LIGHT_BLINK_DURATION_MILLIS)).await;
        } else {
            let color = match flight_mode {
                FlightMode::Idle => RGB8 { r: 0x00, g: 0xff, b: 0x00 },
                FlightMode::HardwareArmed => RGB8 { r: 0xff, g: 0x90, b: 0x00 },
                FlightMode::Armed => RGB8 { r: 0xff, g: 0x00, b: 0x00 },
                FlightMode::ArmedLaunchImminent => RGB8 { r: 0xff, g: 0x00, b: 0x00 }, // TODO
                _ => RGB8::default()
            };

            if let Err(_e) = leds.write([color; 16]) {
                error!("Failed to write LED pattern");
            }

            Timer::after(Duration::from_millis(100)).await;
        }
    }
}
