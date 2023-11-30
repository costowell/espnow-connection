#![no_std]
#![no_main]

use core::str::from_utf8;

use esp_backtrace as _;
use esp_println::println;
use esp_wifi::{
    current_millis,
    esp_now::{PeerInfo, BROADCAST_ADDRESS},
    EspWifiInitFor,
};
use hal::{clock::ClockControl, peripherals::Peripherals, prelude::*, Rng};
use heapless::FnvIndexMap;

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();

    let clocks = ClockControl::max(system.clock_control).freeze();

    let timer = hal::timer::TimerGroup::new(peripherals.TIMG1, &clocks).timer0;
    let init = esp_wifi::initialize(
        EspWifiInitFor::Wifi,
        timer,
        Rng::new(peripherals.RNG),
        system.radio_clock_control,
        &clocks,
    )
    .unwrap();

    let wifi = peripherals.WIFI;
    let mut esp_now = esp_wifi::esp_now::EspNow::new(&init, wifi).unwrap();

    println!("esp-now version {}", esp_now.get_version().unwrap());

    let mut next_send_time = current_millis() + 1000;
    let mut peers = FnvIndexMap::<[u8; 6], Option<u64>, 16>::new();
    loop {
        let r = esp_now.receive();

        if let Some(r) = r {
            if let Ok(data) = from_utf8(r.get_data()) {
                if r.info.dst_address == BROADCAST_ADDRESS && data == "SEARCH" {
                    if !esp_now.peer_exists(&r.info.src_address) {
                        esp_now
                            .add_peer(PeerInfo {
                                peer_address: r.info.src_address,
                                lmk: None,
                                channel: None,
                                encrypt: false,
                            })
                            .unwrap();
                        println!("Adding a new peer: {:?}", r.info.src_address);

                        // Add to ping_map as false
                        match peers.insert(r.info.src_address, None) {
                            Ok(_) => println!("Successfully added peer: {:?}", r.info.src_address),
                            Err(err) => println!("Failed to add peer: {:?}", err),
                        }
                    }
                } else if r.info.dst_address != BROADCAST_ADDRESS {
                    if data == "PING" {
                        let _status = esp_now.send(&r.info.src_address, b"PONG").unwrap().wait();
                        // println!("Sending to {:?}: PONG", r.info.src_address);
                    } else if data == "PONG" {
                        // Get the value from ping map and set it to None
                        if let Ok(Some(Some(millis))) = peers.insert(r.info.src_address, None) {
                            println!(
                                "Ping of {} ms with {:?}",
                                current_millis() - millis,
                                &r.info.src_address
                            );
                        }
                    }
                }
            }
        }

        if current_millis() >= next_send_time {
            next_send_time = current_millis() + 1000;

            // Find more peers
            let _status = esp_now.send(&BROADCAST_ADDRESS, b"SEARCH").unwrap().wait();
            // println!("Send broadcast status: {:?}", status);

            // Ping peers
            for (addr, data) in peers.iter_mut() {
                if data.is_none() {
                    data.replace(current_millis());
                    let _status = esp_now.send(addr, b"PING").unwrap().wait();
                    // println!("Sending to {:?}: PING", addr);
                } else if current_millis() - data.unwrap() > 10000 {
                    match esp_now.remove_peer(addr) {
                        Ok(_) => println!("Successfully removed inactive peer."),
                        Err(_) => println!("Failed to remove inactive peer."),
                    }
                }
            }
        }
    }
}
