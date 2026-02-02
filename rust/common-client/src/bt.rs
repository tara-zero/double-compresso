// SPDX-License-Identifier: Apache-2.0 OR MIT

use ::std::cmp::Ordering;
use ::std::collections::{BTreeMap, HashSet};
use ::std::mem::replace;
use ::std::sync::{Arc, Mutex, MutexGuard, Weak};
use ::std::time::Duration;
use ::std::time::Instant;

pub use ::btleplug::api::BDAddr;

use ::anyhow::{Context as _, Error, ensure, format_err};
use ::btleplug::api::{
    Central, CentralEvent, CentralState, Descriptor, Manager as _, Peripheral as _, ScanFilter,
};
use ::btleplug::platform::{Adapter, Manager, Peripheral, PeripheralId};
use ::futures::stream::StreamExt;
use ::never_say_never::Never;
use ::tokio::select;
use ::tokio::sync::mpsc::{UnboundedSender, unbounded_channel};
use ::tokio::task::JoinHandle;
use ::tokio::time::{sleep, sleep_until};
use ::tokio_stream::wrappers::UnboundedReceiverStream;
use ::tracing::{debug, error, info, warn};
use ::uuid::Uuid;

use ::double_compresso_common::bt;

use crate::error::Result;

const SCAN_START_INTERVAL: Duration = Duration::from_secs(10);
const REMOVE_DISCOVERED_DEVICE_AFTER: Duration = Duration::from_secs(15);

const GATT_DESC_FW_VER: Uuid = uuid_from_bytes(bt::GATT_DESC_FW_VER);
const GATT_SERVICE_FW: Uuid = uuid_from_bytes(bt::GATT_SERVICE_FW);
const GATT_CHAR_COMMAND: Uuid = uuid_from_bytes(bt::GATT_CHAR_COMMAND);

const fn uuid_from_bytes(bytes: [u8; 16]) -> Uuid {
    Uuid::from_u128(u128::from_le_bytes(bytes))
}

//
// Initial state.
//

/// Initial state: just list available Bluetooth adapters.
pub struct StateList {
    manager: Manager,
    adapter_names: Vec<String>,
    adapters: Vec<Adapter>,
}

impl StateList {
    pub async fn state_new() -> Result<Self> {
        let mut state = Self {
            manager: Manager::new()
                .await
                .context("Failed to connect to a Bluetooth manager service")?,
            adapter_names: Vec::with_capacity(1),
            adapters: Vec::with_capacity(1),
        };
        state.populate_adapters().await?;
        Ok(state)
    }

    async fn populate_adapters(&mut self) -> Result<()> {
        let raw_adapters = self
            .manager
            .adapters()
            .await
            .context("Failed to list available Bluetooth adapters")?;

        let mut adapters = Vec::with_capacity(raw_adapters.len());

        for adapter in raw_adapters.into_iter() {
            let name = adapter
                .adapter_info()
                .await
                .context("Failed to get a Bluetooth adapter info")?;
            let state = adapter.adapter_state().await.with_context(|| {
                format!("Failed to get a Bluetooth adapter state for {:?}", name)
            })?;
            debug!("Found BT adapter {:?} with state {:?}", name, state);

            let state = match state {
                CentralState::PoweredOff => continue,

                CentralState::PoweredOn => 0,
                // Adapters with unknown state have lower priority.
                CentralState::Unknown => 1,
            };

            adapters.push((state, name, adapter));
        }
        // Sort by state and name.
        adapters.sort_by(|a, b| match a.0.cmp(&b.0) {
            Ordering::Equal => a.1.cmp(&b.1),
            other => other,
        });

        self.adapter_names.clear();
        self.adapters.clear();
        self.adapter_names.reserve_exact(adapters.len());
        self.adapters.reserve_exact(adapters.len());

        for (_, name, adapter) in adapters.into_iter() {
            self.adapter_names.push(name);
            self.adapters.push(adapter);
        }

        Ok(())
    }

    pub fn adapter_names(&self) -> &[String] {
        &self.adapter_names
    }

    pub async fn state_next(
        self,
        adapter: AdapterSelection<'_>,
    ) -> Result<(StateScan, ScanEventStream)> {
        StateScan::new(self, adapter).await
    }
}

//
// Scanning.
//

// TODO: Detect new adapters and automatically change unless selected by name.
/// State: scanning for Double Compresso devices.
pub struct StateScan {
    list: StateList,

    adapter: Option<CurrentAdapter>,
    devices: Devices,
}

impl StateScan {
    async fn new(
        list: StateList,
        adapter_selection: AdapterSelection<'_>,
    ) -> Result<(Self, ScanEventStream)> {
        let (tx_scan_event, rx_scan_event) = unbounded_channel();
        let devices = Devices::new(tx_scan_event);

        let adapter = match adapter_selection {
            AdapterSelection::Any => {
                if list.adapters.is_empty() {
                    None
                } else {
                    Some(CurrentAdapter::by_index(&list, 0, &devices).await?)
                }
            }

            AdapterSelection::Index(index) => {
                Some(CurrentAdapter::by_index(&list, index, &devices).await?)
            }
            AdapterSelection::PartialName(name) => {
                Some(CurrentAdapter::by_name(&list, name, &devices).await?)
            }
        };

        Ok((
            Self {
                list,

                adapter,
                devices,
            },
            ScanEventStream::new(rx_scan_event),
        ))
    }

    pub async fn state_next(
        mut self,
        id: &PeripheralId,
    ) -> ::std::result::Result<(), (Error, Self)> {
        self.state_next_inner(id).await.map_err(|e| (e, self))
    }

    pub async fn state_next_inner(&mut self, id: &PeripheralId) -> Result<()> {
        let current_adapter = self.adapter_mut(id)?;
        current_adapter.stop_scan().await;
        let adapter = &current_adapter.adapter;

        let peripheral = peripheral(adapter, id).await?;
        info!("Connecting to \"{}\"...", id);
        peripheral.connect().await?;
        info!("Connected to \"{}\"!", id);

        let result = self.state_next_inner_connected(&peripheral).await;
        match result {
            Err(error) => {
                disconnect(&peripheral).await;

                // Restart scanning.
                let devices = self.devices.clone();
                self.adapter_mut(id)?.start_scan(&devices).await;

                Err(error)
            }

            ok => ok,
        }
    }

    fn adapter_mut(&mut self, id: &PeripheralId) -> Result<&mut CurrentAdapter> {
        self.adapter.as_mut().ok_or_else(|| {
            format_err!(
                "Failed to connect to device \"{}\": no active Bluetooth adapter",
                id
            )
        })
    }

    pub async fn state_next_inner_connected(&self, peripheral: &Peripheral) -> Result<()> {
        let id = peripheral.id();
        peripheral
            .discover_services()
            .await
            .with_context(|| format!("Failed to discover services for device \"{}\"", id))?;
        let characteristics = peripheral.characteristics();
        debug!(
            "Characteristics of peripheral \"{}\":\n{:#?}",
            id, characteristics
        );

        let ver_str = Descriptor {
            uuid: GATT_DESC_FW_VER,
            service_uuid: GATT_SERVICE_FW,
            characteristic_uuid: GATT_CHAR_COMMAND,
        };
        let ver_str = peripheral
            .read_descriptor(&ver_str)
            .await
            .with_context(|| {
                format!("Failed to read firmware version descriptor from \"{}\"", id)
            })?;
        debug!("Firmware version of device \"{}\": {:?}", id, ver_str);

        let ver_str = String::from_utf8(ver_str).with_context(|| {
            format!(
                "Firmware version of device \"{}\" contains invalid UTF-8",
                id
            )
        })?;
        debug!("Firmware version of device \"{}\": {:?}", id, ver_str);

        let mut ver_str = ver_str.split(',');
        let ota_ver = Self::next_ver_uint(&id, ver_str.next())?;
        let fw_name = Self::next_ver_str(&id, ver_str.next())?;
        let fw_ver = Self::next_ver_str(&id, ver_str.next())?;
        let proto_ver = Self::next_ver_uint(&id, ver_str.next())?;
        ensure!(
            ver_str.next().is_none(),
            "Too many values in version string for device {}",
            id
        );

        info!(
            "OTA v{}, {} v{}, Proto v{} (\"{}\")",
            ota_ver, fw_name, fw_ver, proto_ver, id
        );

        Ok(())
    }

    fn next_ver_uint(id: &PeripheralId, next: Option<&str>) -> Result<u8> {
        Self::next_ver_str(id, next)?
            .parse()
            .map_err(|e| format_err!("Failed to parse version number for device {}: {}", id, e))
    }

    fn next_ver_str<'a>(id: &PeripheralId, next: Option<&'a str>) -> Result<&'a str> {
        next.ok_or_else(|| format_err!("Not enough values in version string for device {}", id))
    }
}

pub enum AdapterSelection<'a> {
    /// Choose any available Bluetooth adapter.
    Any,

    /// Choose the Bluetooth adapter with the given index.
    /// Panics when the index is out of bounds.
    Index(usize),

    /// Choose the Bluetooth adapter with the given name.
    /// Name may be partial.
    /// Errors when the name is not found.
    PartialName(&'a str),
}

impl Default for AdapterSelection<'_> {
    fn default() -> Self {
        AdapterSelection::Any
    }
}

pub enum ScanEvent {
    Found(BDAddr, PeripheralId),
    Lost(BDAddr),
}

struct CurrentAdapter {
    adapter: Adapter,
    name: Arc<String>,
    index: usize,

    event_handler: Option<JoinHandle<()>>,
}

impl CurrentAdapter {
    async fn by_name(list: &StateList, name: &str, devices: &Devices) -> Result<Self> {
        let mut fuzzy_idx = usize::MAX;
        for (index, adapter_name) in list.adapter_names.iter().enumerate() {
            if adapter_name.contains(name) {
                if fuzzy_idx == usize::MAX {
                    fuzzy_idx = index;
                }
                if adapter_name.len() == name.len() {
                    // Exact match.
                    return Self::by_index(list, index, devices).await;
                }
            }
        }
        if fuzzy_idx != usize::MAX {
            return Self::by_index(list, fuzzy_idx, devices).await;
        }
        Err(format_err!("Bluetooth adapter {:?} not found", name))
    }

    async fn by_index(list: &StateList, index: usize, devices: &Devices) -> Result<Self> {
        let adapter = list
            .adapters
            .get(index)
            .expect("Logic error: no Bluetooth adapter with given index")
            .clone();
        let name = Arc::new(
            list.adapter_names
                .get(index)
                .expect("Logic error: no Bluetooth adapter name with given index")
                .clone(),
        );

        let mut adapter = Self {
            adapter,
            name,
            index,
            event_handler: None,
        };
        adapter.start_scan(devices).await;

        Ok(adapter)
    }

    async fn start_scan(&mut self, devices: &Devices) {
        assert!(
            self.event_handler.is_none(),
            "Logic error: already scanning"
        );
        info!("Scanning with Bluetooth adapter {:?}...", self.name);

        self.event_handler = Some(ScanEventHandlerInner::run(
            &self.name,
            self.adapter.clone(),
            devices,
        ));
    }

    async fn stop_scan(&mut self) {
        let event_handler = self
            .event_handler
            .take()
            .expect("Logic error: not scanning");
        info!("Stop scanning with Bluetooth adapter {:?}...", self.name);

        event_handler.abort();
        if let Err(error) = event_handler.await {
            if !error.is_cancelled() {
                panic!(
                    "Logic error: failed to join scan event_hander task: {}",
                    error
                );
            }
        }

        let result =
            self.adapter.stop_scan().await.with_context(|| {
                format!("Failed to stop Bluetooth scan on adapter {:?}", self.name)
            });
        if let Err(error) = result {
            // May be `Operation already in progress`.
            debug!("{:#}", error);
        }
    }
}

struct ScanEventHandlerInner {
    adapter_name: Arc<String>,
    adapter: Adapter,
    pre_scan: HashSet<BDAddr>,
    devices: Devices,
    next_cleanup: ScheduledCleanup,
}

impl ScanEventHandlerInner {
    fn run(adapter_name: &Arc<String>, adapter: Adapter, devices: &Devices) -> JoinHandle<()> {
        tokio::spawn(Self::handle_scan_events(Self {
            adapter_name: adapter_name.clone(),
            adapter,
            pre_scan: HashSet::new(),
            devices: devices.clone(),
            next_cleanup: ScheduledCleanup::new(),
        }))
    }

    async fn handle_scan_events(mut self) {
        match self.adapter.peripherals().await.with_context(|| {
            format!(
                "Failed to get pre-scan peripherals on {:?}, may discover nonexistent devices",
                self.adapter_name
            )
        }) {
            Ok(pre_scan) => self.pre_scan.extend(pre_scan.iter().map(|p| p.address())),
            Err(error) => warn!("{:#}", error),
        }

        let repeatedly_start_scan = tokio::spawn(Self::repeatedly_start_scan(
            self.adapter.clone(),
            self.adapter_name.clone(),
        ));

        let Err(error) = select! {
            can_events_error = self.handle_scan_events_inner() => can_events_error,
            scan_restart_error = repeatedly_start_scan => match scan_restart_error {
                Ok(error) => error,
                Err(error) => if error.is_cancelled() {
                    Err(format_err!("repeatedly_start_scan task was cancelled"))
                } else {
                    panic!(
                        "Logic error: unable to join repeatedly_start_scan task: {}",
                        error
                    )
                }
            },
        };
        self.devices.lock().send_error(error);
    }

    async fn handle_scan_events_inner(&mut self) -> Result<Never> {
        let mut events = self.adapter.events().await?;
        loop {
            match events
                .next()
                .await
                .expect("Logic error: Bluetooth central event stream ended unexpectedly")
            {
                CentralEvent::DeviceDiscovered(id) => {
                    self.discover_device("discovered", &id, true).await?;
                }

                CentralEvent::DeviceUpdated(id) => {
                    self.discover_device("updated", &id, false).await?;
                }

                CentralEvent::DeviceConnected(id) => {
                    self.discover_device("connected", &id, false).await?;
                }

                CentralEvent::DeviceDisconnected(id) => {
                    // Do not update "last seen" times because this may happen on a timeout.
                    debug!("Device \"{}\" disconnected", id);
                }

                CentralEvent::ManufacturerDataAdvertisement {
                    id,
                    manufacturer_data,
                } => {
                    self.discover_device("sent manufacturer data advertisement", &id, false)
                        .await?;
                    debug!(
                        "Manufacturer data for device \"{}\": {:#?}",
                        id, manufacturer_data
                    );
                }

                CentralEvent::ServiceDataAdvertisement { id, service_data } => {
                    self.discover_device("sent service data advertisement", &id, false)
                        .await?;
                    debug!("Service data for device \"{}\": {:#?}", id, service_data);
                }

                CentralEvent::ServicesAdvertisement { id, .. } => {
                    self.discover_device("sent services advertisement", &id, false)
                        .await?;
                }

                CentralEvent::StateUpdate(state) => {
                    debug!(
                        "State of Bluetooth adapter {:?} changed to {:?}",
                        self.adapter_name, state
                    );
                    if state == CentralState::PoweredOff {
                        self.devices.clear_all();
                    }
                    // TODO: Force change adapter if the old one is Off.
                }
            }
        }
    }

    async fn repeatedly_start_scan(adapter: Adapter, name: Arc<String>) -> Result<Never> {
        // TODO: What if this messes up with `pre_scan` logic?
        loop {
            // It seems that this blocks on Linux if adapter is already scanning.
            debug!("Starting scan on {:?}...", name);
            let result = adapter
                .start_scan(ScanFilter {
                    services: vec![GATT_SERVICE_FW],
                })
                .await
                .with_context(|| format!("Failed to start Bluetooth scan on adapter {:?}", name));
            if let Err(error) = result {
                // May be `Operation already in progress`.
                debug!("{:#}", error);
            }
            debug!("Started scan on {:?}", name);

            sleep(SCAN_START_INTERVAL).await;
        }
    }

    async fn discover_device(
        &mut self,
        what: &str,
        id: &PeripheralId,
        on_discovered: bool,
    ) -> Result<()> {
        let peripheral = peripheral(&self.adapter, id).await?;
        let is_connected = peripheral.is_connected().await?;
        let properties = peripheral.properties().await.with_context(|| {
            format!("Failed to get properties of peripheral with ID \"{}\"", id)
        })?;
        debug!(
            "Device \"{}\" ({}) {}, properties:\n{:#?}",
            peripheral.id(),
            if is_connected {
                "connected"
            } else {
                "disconnected"
            },
            what,
            properties,
        );

        if self.pre_scan.remove(&peripheral.address()) && on_discovered {
            debug!(
                "Device \"{}\" existed before scan, may be stale, skipping",
                id
            );
            return Ok(());
        }

        let maybe_next_cleanup = if let Some(properties) = properties {
            if properties.services.contains(&GATT_SERVICE_FW) {
                self.devices.touch(&id, &peripheral.address(), false)
            } else {
                debug!(
                    "Device \"{}\" does not provide expected service, skipping",
                    id
                );
                None
            }
        } else {
            self.devices.touch(&id, &peripheral.address(), true)
        };
        if let Some(next_cleanup) = maybe_next_cleanup {
            self.next_cleanup.schedule(&self.devices, next_cleanup);

            if is_connected {
                disconnect(&peripheral).await;
            }
        }

        Ok(())
    }
}

async fn peripheral(adapter: &Adapter, id: &PeripheralId) -> Result<Peripheral> {
    adapter
        .peripheral(id)
        .await
        .with_context(|| format!("Failed to get Bluetooth peripheral with ID \"{}\"", id))
}

#[derive(Clone)]
struct ScheduledCleanup(Arc<Mutex<Option<ScheduledCleanupInner>>>);
type ScheduledCleanupWeak = Weak<Mutex<Option<ScheduledCleanupInner>>>;

impl ScheduledCleanup {
    fn new() -> Self {
        ScheduledCleanup(Arc::new(Mutex::new(None)))
    }

    fn schedule(&self, devices: &Devices, next_cleanup: Instant) {
        let mut locked = self.lock();

        if let Some(scheduled) = locked.as_ref() {
            if scheduled.time == next_cleanup {
                // Cleanup at this time already scheduled.
                return;
            }
            assert!(
                scheduled.time < next_cleanup,
                "Logic error: scheduled cleanup cannot move to an earlier time"
            );

            scheduled.task.abort();
        }

        *locked = Some(ScheduledCleanupInner {
            task: tokio::spawn(Self::cleanup(
                Arc::downgrade(&self.0),
                devices.clone(),
                next_cleanup,
            )),
            time: next_cleanup,
        });
    }

    async fn cleanup(weak_self: ScheduledCleanupWeak, devices: Devices, time: Instant) {
        sleep_until(time.into()).await;

        let Some(strong_self) = weak_self.upgrade().map(|sc| ScheduledCleanup(sc)) else {
            return;
        };

        let mut locked = strong_self.lock();
        if let Some(next_cleanup) = devices.clear_stale() {
            drop(locked);
            strong_self.schedule(&devices, next_cleanup);
        } else {
            *locked = None;
        }
    }

    fn lock(&self) -> MutexGuard<'_, Option<ScheduledCleanupInner>> {
        self.0
            .lock()
            .expect("Logic error: scheduled cleanup of device list was poisoned by another thread")
    }
}

struct ScheduledCleanupInner {
    task: JoinHandle<()>,
    time: Instant,
}

#[derive(Clone)]
struct Devices(Arc<Mutex<DevicesInner>>);

impl Devices {
    fn new(tx_scan_event: UnboundedSender<Result<ScanEvent>>) -> Self {
        Devices(Arc::new(Mutex::new(DevicesInner {
            by_time: BTreeMap::new(),
            by_addr: BTreeMap::new(),
            tx_scan_event,
        })))
    }
}

impl Devices {
    fn touch(&self, id: &PeripheralId, addr: &BDAddr, only_existing: bool) -> Option<Instant> {
        let mut locked = self.lock();

        if only_existing && !locked.by_addr.contains_key(addr) {
            return None;
        }

        let now = Instant::now();

        if let Some(last_seen) = locked.by_addr.insert(*addr, now) {
            let by_time = locked
                .by_time
                .get_mut(&last_seen)
                .expect("Logic error: no devices last seen at that time");

            by_time.remove(addr);
            if by_time.is_empty() {
                let _ = locked.by_time.remove(&last_seen);
            }
        } else {
            locked.send_event(ScanEvent::Found(*addr, id.clone()));
        }

        if let Some(by_time) = locked.by_time.get_mut(&now) {
            by_time.insert(*addr);
        } else {
            locked.by_time.insert(now, HashSet::from([*addr]));
        }

        Some(
            locked
                .clean_stale_at()
                .expect("Logic error: no last time seen"),
        )
    }

    fn clear_all(&self) {
        let mut locked = self.lock();

        for addr in locked.by_addr.keys() {
            locked.send_event(ScanEvent::Lost(*addr));
        }
        locked.by_time.clear();
        locked.by_addr.clear();
    }

    fn clear_stale(&self) -> Option<Instant> {
        let mut locked = self.lock();

        let oldest = Instant::now() - REMOVE_DISCOVERED_DEVICE_AFTER;
        let keep = locked.by_time.split_off(&oldest);
        for (_, remove_addrs) in replace(&mut locked.by_time, keep) {
            for addr in remove_addrs.into_iter() {
                let _ = locked.by_addr.remove(&addr);
                locked.send_event(ScanEvent::Lost(addr));
            }
        }

        locked.clean_stale_at()
    }

    fn lock(&self) -> MutexGuard<'_, DevicesInner> {
        self.0
            .lock()
            .expect("Logic error: another thread poisoned the list of devices")
    }
}

struct DevicesInner {
    by_time: BTreeMap<Instant, HashSet<BDAddr>>,
    by_addr: BTreeMap<BDAddr, Instant>,
    tx_scan_event: UnboundedSender<Result<ScanEvent>>,
}

impl DevicesInner {
    fn send_event(&self, event: ScanEvent) {
        match &event {
            ScanEvent::Found(addr, id) => info!("Found device {} (\"{}\")", addr.to_string(), id),
            ScanEvent::Lost(addr) => info!("Lost device {}", addr.to_string()),
        }

        self.send_result(Ok(event));
    }

    fn send_error(&self, error: Error) {
        self.send_result(Err(error));
    }

    fn send_result(&self, result: Result<ScanEvent>) {
        self.tx_scan_event
            .send(result)
            .expect("Logic error: scan event receiver dropped before stopping the scanner")
    }

    fn clean_stale_at(&self) -> Option<Instant> {
        self.by_time
            .keys()
            .next()
            .map(|i| *i + REMOVE_DISCOVERED_DEVICE_AFTER + Duration::from_millis(1))
    }
}

pub enum StateAfterScan {
    Speedtest(StateSpeedtest),
}

pub async fn next_scan_event(rx_scan_event: &mut ScanEventStream) -> Result<ScanEvent> {
    rx_scan_event
        .next()
        .await
        .expect("Logic error: scan event stream closed unexpectedly")
}

pub type ScanEventStream = UnboundedReceiverStream<Result<ScanEvent>>;

//
// Speedtest.
//

/// Test peripheral -> central GATT notification throughput.
pub struct StateSpeedtest {
    // TODO: Implement speedtest.
}

async fn disconnect(peripheral: &Peripheral) {
    let id = peripheral.id();
    debug!("Disconnecting from \"{}\"...", id);
    if let Err(error) = peripheral.disconnect().await {
        error!(
            "Failed to disconnect from device \"{}\" after another error: {:?}",
            id, error
        );
    }
    debug!("Disconnected from \"{}\"", id);
}
