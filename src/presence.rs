//! The presence manager. It will set up the BLE and scan for beacons as well as generate the
//! advertisements telling others we are in range.

use crate::configuration::{COMPANY_ID, TX_POWER};
use crate::display_task::DisplayState::Presence;
use crate::display_task::{DisplayChannelSender, PresenceMessage};
use crate::soul_config;
use core::str::FromStr;
use defmt::{Debug2Format, info, trace};
use embassy_futures::join::join3;
use embassy_time::{Duration, Instant};
use esp_wifi::ble::controller::BleConnector;
use heapless::String;
use smart_leds::RGB8;
use trouble_host::HostResources;
use trouble_host::prelude::AdStructure::{CompleteLocalName, Flags, ManufacturerSpecificData, Unknown};
use trouble_host::prelude::*;

pub type BleControllerType = ExternalController<BleConnector<'static>, 20>;

/// Kick of a process that will advertise our beacon to the work. You must provide a BLE
/// controller and a destination channel for the presence messages we receive. It will advertise
/// its name, our manufacturing code with a custom colour and the transmitter power.
///
/// # Parameters
/// * `controller` - The BLE controller instance used for managing Bluetooth communications
/// * `channel` - Static mutable reference to a display channel sender for transmitting presence messages
/// * `address` - The address to use when advertising. It is normally a random address.
#[embassy_executor::task]
pub async fn start_ble(controller: BleControllerType, channel: &'static mut DisplayChannelSender, address: &'static Address) {
    info!("SCANNER: Starting scanner and advertisement task");
    info!("SCANNER: Using randomised MAC address: {:?}", address);
    // Set up the BLE world. This is shamelessly stolen from the TrouBLE examples
    let mut resources: HostResources<DefaultPacketPool, 0, 0> = HostResources::new();
    let stack = trouble_host::new(controller, &mut resources).set_random_address(*address);
    let mut host = stack.build();

    // This is the data that will be advertised as our beacon.
    let mut adv_data = [0; 64];
    let len = AdStructure::encode_slice(
        &[
            CompleteLocalName(soul_config::ADVERTISED_NAME.as_bytes()),
            Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
            ManufacturerSpecificData {
                company_identifier: COMPANY_ID,
                payload: &soul_config::COLOUR,
            },
            Unknown { // Transmitter power advertised as part of the beacon.
                ty: 0x0A,
                data: &[TX_POWER as u8]
            }
        ],
        &mut adv_data[..],
    )
    .unwrap();
    let params = AdvertisementParameters {
        interval_min: Duration::from_millis(200),
        interval_max: Duration::from_millis(500),
        tx_power: TX_POWER,
        ..Default::default()
    };
    let advert = Advertisement::NonconnectableScannableUndirected {
        adv_data: &adv_data[..len],
        scan_data: &[],
    };
    let advertiser = host.peripheral.advertise(&params, advert);

    // Prepare the scanner and a handler to catch its events.
    let mut scanner = Scanner::new(host.central);
    let handler = ScanHandler { channel };

    let config = ScanConfig {
        active: true,
        interval: Duration::from_millis(1000),
        window: Duration::from_millis(500),
        ..Default::default()
    };

    // I used a join over the 3 processes that must run to transmit a beacon, scan for other beacons
    // and host the primary stack runner. This will run until all three tasks are complete which
    // should never terminate.
    // The trick is to NOT await the scanner and advertiser tasks. They won't return from their
    // await until the host runner has started.
    let _ = join3(host.runner.run_with_handler(&handler), advertiser, scanner.scan(&config)).await;
    info!("BLE: Completed advertising, most likely as the result of an error");
}

/// We want a u32 that sort of uniquely identifies the sender's "MAC" address. As we set this
/// to some random value, we will have unique key for the hash that we store
fn addr_to_key(addr: &BdAddr) -> u32 {
    let r = addr.raw();
    r[5] as u32 | (r[4] as u32) << 8 | ((r[3] ^ r[1]) as u32) << 16 | ((r[2] ^ r[0]) as u32) << 24
}

/// State for our event handler. In this case, we just need to tell it where to send the
/// presence messages that we infer from the received device advertisements. Note that this
/// is called from the ble host runner and not from [scanner_task].
struct ScanHandler {
    channel: &'static DisplayChannelSender,
}

impl EventHandler for ScanHandler {
    fn on_adv_reports(&self, mut it: LeAdvReportsIter) {
        while let Some(Ok(report)) = it.next() {
            let mut adv_data = AdStructure::decode(report.data);
            let name = adv_data
                .find_map(|a| match a.unwrap() {
                    CompleteLocalName(d) => str::from_utf8(d).ok(),
                    _ => None,
                })
                .unwrap_or("<Unknown>");

            let mdf = adv_data.find_map(|a| match a.unwrap() {
                ManufacturerSpecificData {
                    company_identifier: d,
                    payload,
                } => Some((d, payload)),
                _ => None,
            });

            // We filter here for our beacons only and simply drop any others we don't\
            // recognise. We use our manufacturing code to do this.
            if let Some((COMPANY_ID, colour)) = mdf
                && colour.len() == 3
            {
                trace!("Advertisement: Advertisement found: {:?} {:?} {:?}", Debug2Format(&name), mdf, &report.addr);
                let p = PresenceMessage {
                    rssi: report.rssi,
                    address: addr_to_key(&report.addr),
                    last_seen: Instant::now(),
                    name: String::from_str(name).unwrap(),
                    color: RGB8::new(colour[0], colour[1], colour[2]),
                };
                // This is not an async callback, so we cannot await here. Because we get these beacons
                // regularly, we can just try to send it. If the queue is full, just drop it and let the
                // peripheral send it again.
                if self.channel.try_send(Presence(p)).is_err() {
                    info!("BLE_EVENT: Failed to send message")
                }
            } // Don't care about else conditions but could log it for posterity.
        }
    }
}
