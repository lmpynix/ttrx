// module radio
// Radio setup and control. 
extern crate nrf52840_pac as np;
use cortex_m::{asm};

// Start-of-Frame Delimiter byte. Nonstandard since we don't comply with 802.15.4 MAC.
const FSBYTE: u8 = 0xEC;
// Desired Frequency for transmission/reception
const TX_FREQ: u16 = 2440; 
// Frequency Computation: a const function
const fn compute_frequency_values(mhz: u16) -> (u8, bool) {
    let map: bool;
    let frequency: u8;
    if mhz < 2400 {
        // We can use the LOW frequency range
        map = true;
        // Base value is 2360, subtract and throw into frequency
        frequency = (mhz - 2360) as u8;
    } else {
        // We must use the high frequency range
        map = false;
        // base value is 2400, subtract etc. 
        frequency = (mhz - 2400) as u8;
    }

    (frequency, map)
}
// Do frequency value things. 
const FREQUENCY_REG: u8 = compute_frequency_values(TX_FREQ).0;
const MAP_REG: bool = compute_frequency_values(TX_FREQ).1;

// Max packet length.  Default is probably fine. 
const MAX_PACKET_LEN: u8 = 128;

// CCA parameters from the radio crate
const CRC_POLYNOMIAL: u32 = 0x00011021;
const CCA_ED_THRESHOLD_DEFAULT: u8 = 20;
const CCA_CORR_THRESHOLD_DEFAULT: u8 = 20;
const CCA_CORR_LIMIT_DEFAULT: u8 = 2;
// CRC parameters from the radio crate

pub fn init(p: &np::RADIO) -> () {
    let r = p;
    // Ensure radio is turned on, also give it a power cycle
    r.power.write( |w| w.power().clear_bit());
    asm::delay(100);
    r.power.write( |w| w.power().set_bit());
    asm::delay(100);
    // Enable 802.15.4 mode
    r.mode.write(|w| w.mode().ieee802154_250kbit());
    r.modecnf0.write(|w| w.ru().set_bit().dtx().center());
    // Configure CRC
    r.crccnf.write(|w| w.len().two());
    r.crccnf.write(|w| w.skipaddr().ieee802154());
    r.crcpoly.write(|w| unsafe { w.crcpoly().bits(CRC_POLYNOMIAL) });
    r.crcinit.write(|w| unsafe { w.crcinit().bits(0) });
    // Configure the packet layout
    r.pcnf0.write(|w| w.plen()._32bit_zero());
    r.pcnf0.write(|w| unsafe { w.lflen().bits(8) });
    r.pcnf0.write(|w| w.crcinc().set_bit());
    r.pcnf1.write(|w| unsafe { w.maxlen().bits(MAX_PACKET_LEN)});
    // Set up CCA according to the crate's method for now
    unsafe {
        r.ccactrl.write(|w| {
            w.ccamode()
                .ed_mode()
                .ccaedthres()
                .bits(CCA_ED_THRESHOLD_DEFAULT)
                .ccacorrthres()
                .bits(CCA_CORR_THRESHOLD_DEFAULT)
                .ccacorrthres()
                .bits(CCA_CORR_LIMIT_DEFAULT)
        });
    }
    // Set up frequency registers with the constants given above
    //r.frequency.write(|w| unsafe { w.frequency().bits(FREQUENCY_REG)});
    //r.frequency.write(|w| if MAP_REG {w.map().low()} else {w.map().default()});
    r.frequency.write(|w| unsafe { w.frequency().bits(80)});
    r.frequency.write(|w| w.map().default());
    // Change sfd to custom value
    // r.sfd.write(|w| unsafe { w.sfd().bits(FSBYTE)});
    // Set the transmission power to 0 dBm. 
    r.txpower.write(|w| w.txpower()._0d_bm());
    // Hopefully we're done now?
}

pub fn init_blr(p: &np::RADIO) {
    let r = p;
    // Ensure radio is turned on, also give it a power cycle to reset some stuff
    r.power.write(|w| w.power().clear_bit());
    asm::delay(100);
    r.power.write(|w| w.power().set_bit());
    asm::delay(100);
    // Enable slow BT-LR mode
    r.mode.write(|w| w.mode().ble_lr125kbit());
    r.modecnf0.write(|w| w.ru().set_bit().dtx().center());
    // Configure CRC to not exist (hopefully not a problem in this mode)
    r.crccnf.write(|w| w.len().disabled());
    // Configure the packet layout
    r.pcnf0.write( |w| unsafe {
        w.lflen().bits(8)
         .s0len().clear_bit()
         .s1len().bits(0)
         .s1incl().clear_bit()
         .plen().long_range()
         .crcinc().clear_bit()
         .termlen().bits(2)
    });
    // Configure packet size and other stuff
    r.pcnf1.write( |w| unsafe {
        w.maxlen().bits(MAX_PACKET_LEN)
         .statlen().bits(0)
         .balen().bits(2)
         .endian().clear_bit()
         .whiteen().clear_bit()
    });
    // Configure addressing
    r.base0.write(|w| unsafe { w.base0().bits(0xECEB) });
    r.prefix0.write(|w| unsafe { w.ap0().bits(0x61) });
    // Set transmit frequency
    //r.frequency.write(|w| unsafe { w.frequency().bits(80)});
    //r.frequency.write(|w| w.map().default());
    r.frequency.write(|w| unsafe { w.frequency().bits(30).map().low()});
    // Set the transmission power to 8 dBm
    r.txpower.write(|w| w.txpower().pos8d_bm());
}

pub fn disable_radio(p: &np::RADIO) -> () {
    let r = p;
    if !r.state.read().state().is_disabled() {
        // Set the disable bit
        r.tasks_disable.write(|w| w.tasks_disable().set_bit());
        while !r.events_disabled.read().events_disabled().bit_is_set() {
            asm::nop();
        }
        r.events_disabled.reset();
    }
}

pub fn xmit(p: &np::RADIO, data: &[u8]) -> () {
    let r = p;
    // Ensure the radio is disabled before we begin
    disable_radio(p);
    let xmit_len: usize = data.len();
    if xmit_len >= MAX_PACKET_LEN as usize {
        return;
    }
    let mut byte_buffer: [u8; MAX_PACKET_LEN as usize] = [0; MAX_PACKET_LEN as usize];
    byte_buffer[0] = (xmit_len+2) as u8;
    // byte_buffer[1..(xmit_len)].copy_from_slice(data);
    for (i, c) in data.iter().enumerate() {
        byte_buffer[i+1] = *c;
    }
    // Configure the packet pointer
    let pptr = &mut byte_buffer as *mut _ as u32;
    r.packetptr.write(|w| unsafe { w.bits(pptr) });

    /* Configure shortcuts for state progression. 
     * This includes a CCA check, so if the channel is busy
     * this will just silently fail.  Need to fix this. 
     * State/signal progression is as follows: 
     * RX enable 
     * RX ramp-up
     * CCA
     * CCA result
     * CCA idle
     * TX enable
     * TX start
     * TX
     * PHYEND*/
    r.shorts.reset();
    r.shorts.write(|w| {
            w.rxready_ccastart()
            .enabled()
            .ccaidle_txen()
            .enabled()
            .txready_start()
            .enabled()
            .ccabusy_disable()
            .enabled()
            .phyend_disable()
            .enabled()
            .disabled_rxen()
            .enabled()
    });
    // Start the transmission by starting the reception. 
    //r.tasks_rxen.write(|w| w.tasks_rxen().set_bit());
    // DEBUG: Transmit with no CCA, then disable
    r.tasks_txen.write(|w| w.tasks_txen().set_bit());
    asm::delay(0x00FFFFFF);
    disable_radio(p);
}

pub fn xmit_explicit(p: &np::RADIO, data: &[u8], wait: bool) -> () {
    let r = p;
    disable_radio(p);
    let xmit_len: usize = data.len();
    if xmit_len >= MAX_PACKET_LEN as usize {
        return;
    }
    let mut byte_buffer: [u8; MAX_PACKET_LEN as usize] = [0; MAX_PACKET_LEN as usize];
    byte_buffer[0] = (xmit_len+2) as u8;
    // byte_buffer[1..(xmit_len)].copy_from_slice(data);
    for (i, c) in data.iter().enumerate() {
        byte_buffer[i+1] = *c;
    }
    // Configure the packet pointer
    let pptr = &mut byte_buffer as *mut _ as u32;
    r.packetptr.write(|w| unsafe { w.bits(pptr) });
    /*
     * Since this is the explicit transmit function, manually progress through the state changes.
     * Since we assumedly start in DISABLED, trigger TXEN manually, wait for the transmitter to warm up,
     * then transmit the data in the buffer.  This is similar to the old transmit_simple() in ttrx_net. 
     */
    // Disable all shortcuts
    r.shorts.reset();
    // Warm up the transmitter
    r.tasks_txen.write(|w| w.tasks_txen().set_bit());
    // Wait until the transmitter has warmed up
    while r.events_txready.read().events_txready().bit_is_clear() {};
    r.events_txready.reset();
    // Optionally, wait a while with the carrier on
    if wait {
        asm::delay(0x00FFFFFF);
    };
    // Start transmission
    r.tasks_start.write(|w| w.tasks_start().set_bit());
    // Wait for transmission to end
    while r.events_phyend.read().events_phyend().bit_is_clear() {};
    r.events_phyend.reset();
    // Disable the radio
    // asm::delay(0xFF);
    disable_radio(p);
}

pub fn infinite_carrier(p: &np::RADIO) -> ! {
    let r = p;
    disable_radio(p);
    r.shorts.reset();
    loop {
        r.tasks_txen.write(|w| w.tasks_txen().set_bit());
        asm::delay(0xFFF);
        if r.state.read().state().is_tx_idle() { break };
    };
    loop {}
}