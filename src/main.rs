#![no_std]
#![no_main]
#![allow(dead_code)]

#[macro_use(entry)]
extern crate cortex_m_rt as rt;
extern crate cortex_m;
use cortex_m::{peripheral::{syst, Peripherals}, asm};
use nrf52840_pac as np;
use nrf52840_hal::{clocks};
use panic_halt as _;

mod radio;

#[entry]
fn ResetHandler() -> ! {
    main();
    loop {}
}

fn main() {
    let p = np::Peripherals::take().unwrap();
    // TODO: Clock stuff?
    let _clocks = clocks::Clocks::new(p.CLOCK)
            .enable_ext_hfosc()
            .set_lfclk_src_external(clocks::LfOscConfiguration::NoExternalNoBypass)
            .start_lfclk();
    // Initialize the radio
    radio::init_blr(&p.RADIO);
    // Make some data to send
    let data_string = "TTRX TEST DEVICE DE AA9LP ABCDEFGHIJKLMNOPQRSTUVQXYZ 0123456789ABCDEF";
    let mut data: [u8; 120] = [0; 120];
    for (i, c) in data_string.chars().enumerate() {
        data[i] = c as u8;
    }
    //radio::infinite_carrier(&p.RADIO);
    // Transmit every so often
    loop {
        radio::xmit_explicit(&p.RADIO, &data, false);
        // for _ in 1..0x00FFFFFF {};
        asm::delay(0x00FFFFFF);
    }

}