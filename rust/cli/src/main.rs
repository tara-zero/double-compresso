// SPDX-License-Identifier: Apache-2.0 OR MIT

use btleplug::api::bleuuid::BleUuid;
use btleplug::api::{Central, CentralEvent, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::Manager;
use futures::stream::StreamExt;
use log::{debug, info};
use uuid::Uuid;

const GATT_SERVICE_FW: Uuid = Uuid::from_u128(u128::from_le_bytes(
    ::double_compresso_common::bt::GATT_SERVICE_FW,
));

#[cfg(target_os = "windows")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[tokio::main]
async fn main() {
    env_logger::init_from_env(env_logger::Env::new().filter_or("RUST_LOG", "info"));

    let manager = Manager::new().await.unwrap();
    let mut adapters = manager.adapters().await.unwrap();
    info!("Found {} Bluetooth adapters:", adapters.len());
    for adapter in adapters.iter() {
        let info = adapter.adapter_info().await.unwrap();
        info!("    {}", info);
    }

    let adapter = adapters.pop().unwrap();
    let mut events = adapter.events().await.unwrap();
    // TODO: Scan filtering is inconsistent across OSes.
    adapter.start_scan(ScanFilter::default()).await.unwrap();

    while let Some(event) = events.next().await {
        match event {
            CentralEvent::DeviceDiscovered(id) => {
                let peripheral = adapter.peripheral(&id).await.unwrap();
                let properties = peripheral.properties().await.unwrap();
                let local_name = properties
                    .as_ref()
                    .and_then(|p| p.local_name.as_ref())
                    .map(|ln| ln.as_str())
                    .unwrap_or_else(|| "");
                let services = properties
                    .as_ref()
                    .map(|p| p.services.as_slice())
                    .unwrap_or_else(|| &[]);

                if services.contains(&GATT_SERVICE_FW) {
                    info!(
                        "BLE device discovered: {:?} ({:?})",
                        local_name,
                        id.to_string()
                    );
                }
            }

            CentralEvent::StateUpdate(state) => {
                debug!("BLE adapter state update: {:?}", state);
            }

            CentralEvent::DeviceConnected(id) => {
                debug!("BLE device connected: {:?}", id.to_string());
            }

            CentralEvent::DeviceUpdated(id) => {
                debug!("BLE device updated: {:?}", id.to_string());
            }

            CentralEvent::DeviceDisconnected(id) => {
                debug!("BLE device disconnected: {:?}", id.to_string());
            }

            CentralEvent::ManufacturerDataAdvertisement {
                id,
                manufacturer_data,
            } => {
                debug!(
                    "BLE manufacturer data advertisement: {:?}, {:?}",
                    id.to_string(),
                    manufacturer_data
                );
            }

            CentralEvent::ServiceDataAdvertisement { id, service_data } => {
                debug!(
                    "BLE service data advertisement: {:?}, {:?}",
                    id.to_string(),
                    service_data
                );
            }

            CentralEvent::ServicesAdvertisement { id, services } => {
                let services: Vec<String> =
                    services.into_iter().map(|s| s.to_short_string()).collect();
                debug!(
                    "BLE services advertisement: {:?}, {:?}",
                    id.to_string(),
                    services
                );
            }
        }
    }
}
