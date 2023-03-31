#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

#[rtic::app(
    device = rp_pico::hal::pac,
    dispatchers = [TIMER_IRQ_1]
)]
mod app {
    use rp_pico::hal::{
        clocks, gpio,
        gpio::pin::bank0::{Gpio2, Gpio25, Gpio3},
        gpio::pin::PushPullOutput,
        pac,
        sio::Sio,
        watchdog::Watchdog,
        I2C,
    };
    use rp_pico::XOSC_CRYSTAL_FREQ;

    use core::mem::MaybeUninit;
    use embedded_hal::digital::v2::{OutputPin, ToggleableOutputPin};
    use fugit::RateExtU32;
    use rtic_monotonics::rp2040::*;

    use panic_probe as _;

    type I2CBus = I2C<
        pac::I2C1,
        (
            gpio::Pin<Gpio2, gpio::FunctionI2C>,
            gpio::Pin<Gpio3, gpio::FunctionI2C>,
        ),
    >;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        led: gpio::Pin<Gpio25, PushPullOutput>,
        i2c: &'static mut I2CBus,
    }

    #[init(local=[
        // Task local initialized resources are static
        // Here we use MaybeUninit to allow for initialization in init()
        // This enables its usage in driver initialization
        i2c_ctx: MaybeUninit<I2CBus> = MaybeUninit::uninit()
    ])]
    fn init(mut ctx: init::Context) -> (Shared, Local) {
        // Configure the clocks, watchdog
        let token = rtic_monotonics::create_rp2040_monotonic_token!();
        Timer::start(ctx.device.TIMER, &mut ctx.device.RESETS, token);
        let mut watchdog = Watchdog::new(ctx.device.WATCHDOG);
        let clocks = clocks::init_clocks_and_plls(
            XOSC_CRYSTAL_FREQ,
            ctx.device.XOSC,
            ctx.device.CLOCKS,
            ctx.device.PLL_SYS,
            ctx.device.PLL_USB,
            &mut ctx.device.RESETS,
            &mut watchdog,
        )
        .ok()
        .unwrap();

        // Init LED pin
        let sio = Sio::new(ctx.device.SIO);
        let gpioa = rp_pico::Pins::new(
            ctx.device.IO_BANK0,
            ctx.device.PADS_BANK0,
            sio.gpio_bank0,
            &mut ctx.device.RESETS,
        );
        let mut led = gpioa.led.into_push_pull_output();
        led.set_low().unwrap();

        // Init I2C pins
        let sda_pin = gpioa.gpio2.into_mode::<gpio::FunctionI2C>();
        let scl_pin = gpioa.gpio3.into_mode::<gpio::FunctionI2C>();

        // Init I2C itself, using MaybeUninit to overwrite the previously
        // uninitialized i2c_ctx variable without dropping its value
        // (i2c_ctx definined in init local resources above)
        let i2c_tmp: &'static mut _ = ctx.local.i2c_ctx.write(I2C::i2c1(
            ctx.device.I2C1,
            sda_pin,
            scl_pin,
            100.kHz(),
            &mut ctx.device.RESETS,
            &clocks.system_clock,
        ));

        // Spawn heartbeat task
        heartbeat::spawn().ok();

        // Return resources and timer
        (Shared {}, Local { led, i2c: i2c_tmp })
    }

    #[task(local = [i2c, led])]
    async fn heartbeat(ctx: heartbeat::Context) {
        // Flicker the built-in LED
        _ = ctx.local.led.toggle();

        // Congrats, you can use your i2c and have access to it here,
        // now to do something with it!

        // Re-spawn this task after 1 second
        Timer::delay(1000.millis()).await;
    }
}
