// The presence manager. It will set up the BLE and scan for beacons
use crate::BleControllerType;
use embassy_futures::join::join;
use embassy_time::{Duration, Timer};
use log::info;
use trouble_host::prelude::*;
use trouble_host::{Address, Host, HostResources};

/// kick of a process that will advertise our beacon to the work.
#[embassy_executor::task]
pub async fn start_ble(controller: BleControllerType) {
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
            AdStructure::CompleteLocalName(b"Soul Star C"),
            AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
        ],
        &mut adv_data[..],
    )
    .unwrap();

    info!("BLE: Starting advertising");
    let _ = join(runner.run(), advertiser(&mut peripheral, &adv_data, len)).await;
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
