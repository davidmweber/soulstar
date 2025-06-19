// The presence manager. It will set up the BLE and scan for beacons
use log::info;
use trouble_host::{Address, Host, HostResources};
use trouble_host::prelude::*;
use crate::BleControllerType;
use embassy_futures::join::join;
use embassy_time::{Duration, Timer};

/// kick of a process that will advertise our beacon to the work. Once this function completes,
/// the BLE stack should start to advertise itself
/// TODO: Make this a self terminating task for parallel startup.
/// 
pub async fn start_ble_beacon(controller:  BleControllerType) {

    // TODO: Make this really random
    let address: Address = Address::random([0xff, 0x8f, 0x1a, 0x05, 0xe4, 0xff]);
    info!("Our address = {:?}", address);

    let mut resources: HostResources<DefaultPacketPool, 0, 0> = HostResources::new();
    let stack = trouble_host::new(controller, &mut resources).set_random_address(address);
    let Host {
        mut peripheral,
        mut runner,
        ..
    } = stack.build();

    let mut adv_data = [0; 31];
    let len = AdStructure::encode_slice(
        &[
            AdStructure::CompleteLocalName(b"Soul Star"),
            AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
        ],
        &mut adv_data[..],
    ).unwrap();

    info!("Starting advertising");
    let _ = join(runner.run(), async {
        loop {
            let mut params = AdvertisementParameters::default();
            params.interval_min = Duration::from_millis(100);
            params.interval_max = Duration::from_millis(100);
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
                info!("Still running");
                Timer::after(Duration::from_secs(60)).await;
            }
        }
    }).await;
}