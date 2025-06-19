// The presence manager. It will set up the BLE and scan for beacons
use crate::display_task::DisplayState::Presence;
use crate::display_task::{DisplayChannelSender, PresenceMessage};
use embassy_futures::join::join3;
use embassy_time::{Duration, Timer};
use esp_wifi::ble::controller::BleConnector;
use log::info;
use trouble_host::prelude::*;
use trouble_host::{Address, HostResources};

pub type BleControllerType = ExternalController<BleConnector<'static>, 20>;

/// A global company ID that we set here so we can filter beacons for only SoulStar devices
static COMPANY_ID: u16 = 0xBEEF;
/// Needed only to fill a field. We don't use this data when filtering
static PRODUCT_ID: u8 = 0x01;

/// kick of a process that will advertise our beacon to the work. You must provide a BLE
/// controller and a destination channel for the presence messages we receive.
///
/// # Parameters
/// * `controller` - The BLE controller instance used for managing Bluetooth communications
/// * `channel` - Static mutable reference to a display channel sender for transmitting presence messages
#[embassy_executor::task]
pub async fn start_ble(controller: BleControllerType, channel: &'static mut DisplayChannelSender) {
    // TODO: Make this really random
    let address: Address = Address::random([0xff, 0x8f, 0x1a, 0x05, 0xe4, 0xff]);

    // Set up the BLE world. This is shamelessly stolen from the TrouBLE examples
    let mut resources: HostResources<DefaultPacketPool, 0, 0> = HostResources::new();
    let stack = trouble_host::new(controller, &mut resources).set_random_address(address);
    let mut host = stack.build();

    // This is the data that will be advertised as our beacon.
    let mut adv_data = [0; 31];
    let len = AdStructure::encode_slice(
        &[
            AdStructure::CompleteLocalName(b"Soul Star C"),
            AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
            AdStructure::ManufacturerSpecificData {
                company_identifier: COMPANY_ID,
                payload: &[PRODUCT_ID],
            },
        ],
        &mut adv_data[..],
    )
    .unwrap();

    // Prepare the scanner and a handler to catch its events.
    let scanner = Scanner::new(host.central);
    let handler = ScanHandler { channel };

    info!("BLE: Starting advertise/scan tasks");
    // I used a join over the 3 processes that must run to transmit a beacon, scan for other beacons 
    // and host the primary stack runner. This will run until all three tasks are complete which 
    // should never terminate.
    let _ = join3(
        host.runner.run_with_handler(&handler),
        advertiser(&mut host.peripheral, &adv_data, len),
        scanner_task(scanner),
    )
    .await;
    info!("BLE: Completed advertising, most likely as the result of an error");
}

/// Our beacon broadcasting future. It runs forever.
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
    info!("ADVERTISER: Starting Advertisement task");
    let _advertiser = peripheral
        .advertise(
            &params,
            Advertisement::NonconnectableScannableUndirected {
                adv_data: &adv_data[..len],
                scan_data: &[],
            },
        )
        .await
        .unwrap();
    loop {
        Timer::after(Duration::from_secs(1)).await;
    }
}

/// Runs a continuous BLE scanning task that searches for nearby devices.
/// The scanner runs indefinitely in a loop, processing any discovered devices through
/// the associated event handler.
///
/// # Parameters
/// * `scanner` - The BLE scanner instance to use for device discovery
async fn scanner_task(mut scanner: Scanner<'_, BleControllerType, DefaultPacketPool>) {
    let config = ScanConfig {
        active: true,
        phys: PhySet::M1,
        interval: Duration::from_secs(1),
        window: Duration::from_secs(1),
        ..Default::default()
    };
    info!("SCANNER: Starting scanner");
    scanner.scan(&config).await.unwrap();
    // Scan forever
    loop {
        Timer::after(Duration::from_secs(1)).await;
    }
}

/// State for our event handler. In this case, we just need to tell it where to send the
/// presence messages that we infer from the received device advertisements
struct ScanHandler {
    channel: &'static DisplayChannelSender,
}

impl EventHandler for ScanHandler {
    fn on_adv_reports(&self, mut it: LeAdvReportsIter<'_>) {
        while let Some(Ok(report)) = it.next() {
            info!("BLE: discovered: {:?} {:?}", report.addr, report.rssi);
            let p = PresenceMessage {
                rssi: report.rssi,
                address: 12,
            };
            // This is not an async callback, so we cannot await here. Because we get these beacons
            // regularly, we can just try to send it. If the queue is full, just drop it and let the
            // peripheral send it again.
            if self.channel.try_send(Presence(p)).is_err() {
                info!("BLE: Failed to send message")
            }
        }
    }
}
