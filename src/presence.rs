//! The presence manager. It will set up the BLE and scan for beacons as well as generate the
//! advertisements telling others we are in range.

use crate::display_task::DisplayState::Presence;
use crate::display_task::{DisplayChannelSender, PresenceMessage};
use crate::soul_config;
use core::str::FromStr;
use defmt::{error, info, trace, Debug2Format};
use embassy_futures::join::join3;
use embassy_time::{Duration, Instant, Timer};
use esp_wifi::ble::controller::BleConnector;
use heapless::String;
use smart_leds::RGB8;
use trouble_host::HostResources;
use trouble_host::prelude::AdStructure::{CompleteLocalName, Flags, ManufacturerSpecificData};
use trouble_host::prelude::*;

pub type BleControllerType = ExternalController<BleConnector<'static>, 20>;

/// A global company ID that we set here so we can filter beacons for only SoulStar devices
const COMPANY_ID: u16 = 0xBEEF;

/// Kick of a process that will advertise our beacon to the work. You must provide a BLE
/// controller and a destination channel for the presence messages we receive.
///
/// # Parameters
/// * `controller` - The BLE controller instance used for managing Bluetooth communications
/// * `channel` - Static mutable reference to a display channel sender for transmitting presence messages
#[embassy_executor::task]
pub async fn start_ble(controller: BleControllerType, channel: &'static mut DisplayChannelSender) {
    info!("SCANNER: Starting scanner tasks");
    // Set up the BLE world. This is shamelessly stolen from the TrouBLE examples
    let mut resources: HostResources<DefaultPacketPool, 0, 0> = HostResources::new();
    let stack = trouble_host::new(controller, &mut resources); //.set_random_address(address);
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
        ],
        &mut adv_data[..],
    )
    .unwrap();

    // Prepare the scanner and a handler to catch its events.
    let mut scanner = Scanner::new(host.central);
    let handler = ScanHandler { channel };

    let config = ScanConfig {
        active: true,
        // phys: PhySet::M1M2,
        interval: Duration::from_millis(1000),
        window: Duration::from_millis(500),
        ..Default::default()
    };

    info!("BLE: Starting BLE tasks",);
    // I used a join over the 3 processes that must run to transmit a beacon, scan for other beacons
    // and host the primary stack runner. This will run until all three tasks are complete which
    // should never terminate.
    // The trick is to NOT await the scanner and advertiser tasks. They won't return from their
    // await until the host runner has started.
    let _ = join3(
        host.runner.run_with_handler(&handler),
        advertiser(&mut host.peripheral, &adv_data, len),
        scanner.scan(&config),
    )
    .await;
    info!("BLE: Completed advertising, most likely as the result of an error");
}

/// Our beacon broadcasting future. For some reason, the advertisement beacon stops transmitting.
/// The most likely cause is a connection attempt to the device which will stop the beacon from
/// transmitting. We tell the stack to start advertising at periodic intervals so we get continuous
/// beacons for our presence.
///
/// # Parameters
/// * `peripheral` - The BLE peripheral device used for advertising
/// * `adv_data` - The advertisement data to broadcast
/// * `len` - Length of the advertisement data
async fn advertiser(
    peripheral: &mut Peripheral<'_, BleControllerType, DefaultPacketPool>,
    adv_data: &[u8],
    len: usize,
) {
    let params = AdvertisementParameters {
        interval_min: Duration::from_millis(200),
        interval_max: Duration::from_millis(500),
        ..Default::default()
    };
    let advert = Advertisement::NonconnectableScannableUndirected {
        adv_data: &adv_data[..len],
        scan_data: &[],
    };
    info!("ADVERTISER: Starting Advertisement task");
    loop {
        let _advertiser = match peripheral.advertise(&params, advert).await {
            Ok(session) => session,
            Err(e) => {
                // We need to use defmt::Debug2Format because the BleConnectorError does not
                // implement Format even though we have enabled the defmt feature in esp-wifi crate.
                error!("ADVERTISER: Advertiser failed to start: {:?}", defmt::Debug2Format(&e));
                panic!();
            }
        };
        Timer::after(Duration::from_secs(15)).await;
        info!("ADVERTISER: Re-initializing advertisement transmission");
    }
}

/// We want to use the lowest 4 bytes of the MAC address in the beacon as a key that
/// uniquely identifies the sender. We don't really care about anything else.
fn addr_to_key(addr: &BdAddr) -> u32 {
    let r = addr.raw();
    r[5] as u32 | (r[4] as u32) << 8 | (r[3] as u32) << 16 | ((r[2] ^ r[0]) as u32) << 24
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
            if let Some((COMPANY_ID, colour)) = mdf  && colour.len() == 3 {
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
