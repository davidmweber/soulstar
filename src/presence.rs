use core::cell::RefCell;
// The presence manager. It will set up the BLE and scan for beacons
use crate::BleControllerType;
use embassy_futures::join::join3;
use embassy_time::{Duration, Timer};
use heapless::Deque;
use log::info;
use trouble_host::prelude::*;
use trouble_host::{Address, Host, HostResources};

/// kick of a process that will advertise our beacon to the work.
#[embassy_executor::task]
pub async fn start_ble(controller: BleControllerType) {
    // TODO: Make this really random
    let address: Address = Address::random([0xff, 0x8f, 0x1a, 0x05, 0xe4, 0xff]);

    // Set up the BLE world. This is shamelessly stolen from the TrouBLE examples
    let mut resources: HostResources<DefaultPacketPool, 0, 0> = HostResources::new();
    let stack = trouble_host::new(controller, &mut resources).set_random_address(address);
    let Host {
        mut peripheral,
        central,
        mut runner,
        ..
    } = stack.build();
    
    // This is the data that will be advertised as our beacon. 
    let mut adv_data = [0; 31];
    let len = AdStructure::encode_slice(
        &[
            AdStructure::CompleteLocalName(b"Soul Star C"),
            AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
        ],
        &mut adv_data[..],
    )
    .unwrap();

    // Prepare the scanner and a handler to catch its events.
    // TODO: Create a channel to get the new presence messages to the display task
    let scanner = Scanner::new(central);
    let handler = ScanHandler{
        seen: RefCell::new(Deque::new()),
    };
    
    info!("BLE: Starting advertising");
    let _ = join3(runner.run_with_handler(&handler), advertiser(&mut peripheral, &adv_data, len), scanner_task(scanner)).await;
    info!("BLE: Completed advertising, most likely as the result of an error");
}

/// Our beacon broadcasting future. It runs forever.
async fn advertiser(
    peripheral: &mut Peripheral<'_, BleControllerType, DefaultPacketPool>,
    adv_data: &[u8],
    len: usize,
) {
    let params = AdvertisementParameters {
        primary_phy: Default::default(),
        secondary_phy: Default::default(),
        tx_power: TxPower::Minus40dBm,
        timeout: None,
        max_events: None,
        interval_min: Duration::from_millis(200),
        interval_max: Duration::from_millis(500),
        channel_map: None,
        filter_policy: Default::default(),
        fragment: false,
    };
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

async fn scanner_task(mut scanner: Scanner<'_, BleControllerType, DefaultPacketPool> ) {
    let mut config = ScanConfig::default();
    config.active = true;
    config.phys = PhySet::M1;
    config.interval = Duration::from_secs(1);
    config.window = Duration::from_secs(1);
    let mut _session = scanner.scan(&config).await.unwrap();
    // Scan forever
    loop {
        Timer::after(Duration::from_secs(1)).await;
    }
}

/// TODO: We
struct ScanHandler {
    seen: RefCell<Deque<BdAddr, 128>>,
}

impl EventHandler for ScanHandler {
    fn on_adv_reports(&self, mut it: LeAdvReportsIter<'_>) {
        let mut seen = self.seen.borrow_mut();
        while let Some(Ok(report)) = it.next() {
            if seen.iter().find(|b| b.raw() == report.addr.raw()).is_none() {
                info!("BLE: discovered: {:?} {:?}", report.addr, report.rssi);
                if seen.is_full() {
                    seen.pop_front();
                }
                seen.push_back(report.addr).unwrap();
            }
        }
    }
}
