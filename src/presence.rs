use core::str::FromStr;
// The presence manager. It will set up the BLE and scan for beacons
use crate::display_task::DisplayState::Presence;
use crate::display_task::{DisplayChannelSender, PresenceMessage};
use embassy_futures::join::join3;
use embassy_time::{Duration, Instant, Timer};
use esp_wifi::ble::controller::BleConnector;
use heapless::String;
use log::{error, info};
use trouble_host::HostResources;
use trouble_host::advertise::AdStructure::ShortenedLocalName;
use trouble_host::prelude::AdStructure::{CompleteLocalName, Flags, ManufacturerSpecificData};
use trouble_host::prelude::*;

pub type BleControllerType = ExternalController<BleConnector<'static>, 20>;

/// A global company ID that we set here so we can filter beacons for only SoulStar devices
static COMPANY_ID: u16 = 0xBEEF;
/// Needed only to fill a field. We don't use this data when filtering
static PRODUCT_ID: u8 = 0x01;

/// Kick of a process that will advertise our beacon to the work. You must provide a BLE
/// controller and a destination channel for the presence messages we receive.
///
/// # Parameters
/// * `controller` - The BLE controller instance used for managing Bluetooth communications
/// * `channel` - Static mutable reference to a display channel sender for transmitting presence messages
#[embassy_executor::task]
pub async fn start_ble(controller: BleControllerType, channel: &'static mut DisplayChannelSender) {
    // Set up the BLE world. This is shamelessly stolen from the TrouBLE examples
    let mut resources: HostResources<DefaultPacketPool, 0, 0> = HostResources::new();
    let stack = trouble_host::new(controller, &mut resources); //.set_random_address(address);
    let mut host = stack.build();

    // This is the data that will be advertised as our beacon.
    let mut adv_data = [0; 64];
    let len = AdStructure::encode_slice(
        &[
            CompleteLocalName(b"Soul Star Dave"),
            ShortenedLocalName(b"Soul Star"),
            Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
            ManufacturerSpecificData {
                company_identifier: COMPANY_ID,
                payload: &[PRODUCT_ID],
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
        // interval: Duration::from_millis(1000),
        // window: Duration::from_millis(1000),
        ..Default::default()
    };
    info!("SCANNER: Starting scanner");
    // You absolutely have to keep `_session` in scope for the scanner to continue working
    //let scanner =

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
/// transmitting. For this reason, we tell the stack to start advertising at periodic intervals
/// se we get continuous beacons for our presence.
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
        //timeout: Some(Duration::from_secs(15)),
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
                error!("ADVERTISER: Advertiser failed to start: {:?}", e);
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
    r[5] as u32 | (r[4] as u32) << 8 | (r[3] as u32) << 16 | (r[2] as u32) << 24
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

            let _mdf = adv_data.find_map(|a| match a.unwrap() {
                ManufacturerSpecificData {
                    company_identifier: d,
                    payload,
                } => Some((d, payload[0])),
                _ => None,
            });

            let p = PresenceMessage {
                rssi: report.rssi,
                address: addr_to_key(&report.addr),
                last_seen: Instant::now(),
                name: String::from_str(name).unwrap(),
            };
            // This is not an async callback, so we cannot await here. Because we get these beacons
            // regularly, we can just try to send it. If the queue is full, just drop it and let the
            // peripheral send it again.
            if self.channel.try_send(Presence(p)).is_err() {
                info!("BLE_EVENT: Failed to send message")
            }
        }
    }
}
