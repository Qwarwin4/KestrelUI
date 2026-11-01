use std::{
    sync::{
        Arc, LazyLock,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use seelen_core::system_state::MonitorId;
use wmi::WMIConnection;

use crate::{
    error::{Result, ResultLogExt},
    event_manager,
    modules::monitors::{
        application::{MonitorManager, MonitorManagerEvent},
        brightness::domain::{
            WmiMonitorBrightness, WmiMonitorBrightnessEvent, WmiMonitorBrightnessMethods,
            WmiSetBrightnessPayload,
        },
    },
    utils::lock_free::{SyncVec, TracedMutex},
    windows_api::{
        MonitorEnumerator,
        monitor::{
            Monitor,
            brightness::{DdcciBrightnessValues, PhysicalMonitorHandle},
            gamma::set_gamma_dimming,
        },
    },
};

/// `WBEM_E_NOT_SUPPORTED`: the provider has no instances to offer on this hardware.
const WBEM_E_NOT_SUPPORTED: i32 = 0x8004100C_u32 as i32;

/// DDC/CI has no change notifications, so external monitors are polled at this interval to
/// pick up changes made from the monitor's own OSD. Each read costs one I2C round trip
/// (~60 ms on a healthy monitor) on a background thread.
const DDCCI_POLL_INTERVAL: Duration = Duration::from_secs(5);

/// How long a panel gets to apply a written value before it is read back.
const DDCCI_VERIFY_DELAY: Duration = Duration::from_millis(400);
/// Ignored writes in a row before a monitor is switched to software dimming.
const DDCCI_MAX_MISSES: u8 = 2;

/// Monitors without DDC/CI are probed again every 6 polls (30 s), 10 times after a scan.
const REPROBE_EVERY_POLLS: u32 = 6;
const REPROBE_COUNT: u32 = 10;

/// Serializes every DDC/CI transaction (polling and writes) *together with* the publication
/// of its result into `BrightnessManager::ddcci`. Holding the lock across both steps means a
/// read that started before a write can never overwrite the written value afterwards.
/// Concurrent transactions over the same I2C bus can also corrupt each other.
static DDCCI_LOCK: TracedMutex<()> = TracedMutex::new(());

/// Some screen is dimmed through its gamma ramp, which outlives the process: it has to be
/// put back on exit (see `restore_software_dimming`).
static SOFT_DIMMED: AtomicBool = AtomicBool::new(false);

/// Lowest gamma factor asked for. Drivers usually refuse much below half anyway, and a
/// black screen at 0% would be a trap.
const SOFT_MIN_FACTOR: f64 = 0.15;

/// Puts back every screen dimmed in software. Called when KestrelUI exits.
pub fn restore_software_dimming() {
    if SOFT_DIMMED.swap(false, Ordering::AcqRel) {
        for panel in BrightnessManager::instance().soft.to_vec() {
            set_gamma_dimming(&panel.device, 1.0).log_error();
        }
    }
}

/// Tracks the brightness of every panel, keyed by the stable id of the monitor showing it.
/// A monitor usually has a single panel, but a cloned/mirrored group has one per display.
pub struct BrightnessManager {
    /// Internal displays (laptops), managed through WMI.
    wmi: SyncVec<WmiPanel>,
    /// External displays, managed through DDC/CI. Written only while holding `DDCCI_LOCK`.
    ddcci: SyncVec<DdcciPanel>,
    /// Displays with neither, dimmed through the gamma ramp.
    soft: SyncVec<SoftPanel>,
    /// Monitors that answer DDC/CI reads but ignore writes (seen on cheap panels, or with
    /// DDC/CI switched off in the OSD). They get software dimming for the rest of the session.
    ddcci_ignored: SyncVec<MonitorId>,
    worker_tx: crossbeam_channel::Sender<WorkerMessage>,
}

#[derive(Debug, Clone)]
pub enum BrightnessManagerEvent {
    Changed,
}

event_manager!(BrightnessManager, BrightnessManagerEvent);

impl BrightnessManager {
    pub fn instance() -> &'static Self {
        static INSTANCE: LazyLock<BrightnessManager> = LazyLock::new(|| {
            let m = BrightnessManager::new();
            m.init().log_error();
            m
        });
        &INSTANCE
    }

    fn new() -> Self {
        let (worker_tx, worker_rx) = crossbeam_channel::unbounded();
        std::thread::spawn(move || Self::worker(worker_rx));
        Self {
            wmi: SyncVec::new(),
            ddcci: SyncVec::new(),
            soft: SyncVec::new(),
            ddcci_ignored: SyncVec::new(),
            worker_tx,
        }
    }

    fn init(&self) -> Result<()> {
        MonitorManager::subscribe(|event| match event {
            MonitorManagerEvent::ViewAdded(_)
            | MonitorManagerEvent::ViewRemoved(_)
            | MonitorManagerEvent::ViewsChanged => {
                BrightnessManager::instance().request_rescan();
            }
        });
        self.request_rescan();
        self.listen_wmi()
    }

    /// WMI does notify brightness changes (keyboard keys, Windows settings, power plans).
    fn listen_wmi(&self) -> Result<()> {
        let wmi = WMIConnection::with_namespace_path("ROOT\\WMI")?;
        if Self::wmi_query(&wmi)?.is_none() {
            log::debug!("Monitor brightness is not supported through WMI on this system");
            return Ok(());
        }

        std::thread::spawn(move || {
            let wmi = WMIConnection::with_namespace_path("ROOT\\WMI")?;
            for event in wmi.notification::<WmiMonitorBrightnessEvent>()? {
                let Ok(event) = event else {
                    continue;
                };
                BrightnessManager::instance().on_wmi_event(event);
            }
            Result::Ok(())
        });
        Ok(())
    }

    fn on_wmi_event(&self, event: WmiMonitorBrightnessEvent) {
        let mut known = false;
        let mut changed = false;
        self.wmi.for_each(|panel| {
            if panel.instance_name == event.instance_name {
                known = true;
                changed |= panel.percent != event.brightness;
                panel.percent = event.brightness;
            }
        });

        if !known {
            self.request_rescan();
        } else if changed {
            self.emit_changed();
        }
    }

    fn request_rescan(&self) {
        // the worker owns the receiver for the whole process lifetime, so this can't fail
        let _ = self.worker_tx.send(WorkerMessage::Rescan);
    }

    /// Background loop: pairs the panels with the monitors on demand and polls the DDC/CI ones.
    /// `instance()` must not be called from here until the first message arrives, since the
    /// worker is spawned while the singleton is still being constructed.
    fn worker(rx: crossbeam_channel::Receiver<WorkerMessage>) {
        // block until init sends the first rescan, i.e. until the singleton exists
        if rx.recv().is_err() {
            return;
        }
        Self::scan();
        let mut reprobes_left = REPROBE_COUNT;
        let mut polls_to_reprobe = REPROBE_EVERY_POLLS;
        loop {
            match rx.recv_timeout(DDCCI_POLL_INTERVAL) {
                Ok(WorkerMessage::Rescan) => {
                    // collapse bursts of display events into a single scan
                    while rx.try_recv().is_ok() {}
                    Self::scan();
                    reprobes_left = REPROBE_COUNT;
                }
                Ok(WorkerMessage::Verify(first)) => {
                    // give the panel time to apply the value before reading it back; a slider
                    // drag sends one request per step, they are checked once
                    std::thread::sleep(DDCCI_VERIFY_DELAY);
                    let mut ids = vec![first];
                    let mut rescan = false;
                    while let Ok(message) = rx.try_recv() {
                        match message {
                            WorkerMessage::Verify(id) if !ids.contains(&id) => ids.push(id),
                            WorkerMessage::Verify(_) => {}
                            WorkerMessage::Rescan => rescan = true,
                        }
                    }
                    if rescan {
                        Self::scan();
                    }
                    for id in ids {
                        Self::ddcci_verify(&id);
                    }
                }
                Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
                    Self::ddcci_poll();
                    // DDC/CI on cheap monitors often fails right after boot, wake or an input
                    // switch: monitors left on software dimming are probed again for a while
                    if reprobes_left > 0 && Self::instance().has_reprobable() {
                        polls_to_reprobe -= 1;
                        if polls_to_reprobe == 0 {
                            polls_to_reprobe = REPROBE_EVERY_POLLS;
                            reprobes_left -= 1;
                            Self::scan();
                        }
                    }
                }
                Err(crossbeam_channel::RecvTimeoutError::Disconnected) => return,
            }
        }
    }

    /// Enumerates the connected monitors and looks for the panels behind each of them.
    fn scan() {
        let monitors = match MonitorEnumerator::enumerate_win32() {
            Ok(monitors) => monitors,
            Err(err) => {
                log::warn!("Failed to enumerate monitors for brightness: {err}");
                return;
            }
        };
        let targets: Vec<ScanTarget> = monitors
            .into_iter()
            .filter_map(|monitor| {
                let (id, name) = monitor.get_stable_info().ok()?;
                Some(ScanTarget { monitor, id, name })
            })
            .collect();

        let manager = Self::instance();
        let wmi_changed = manager.wmi_scan(&targets);
        let ddcci_changed = manager.ddcci_scan(&targets);
        let soft_changed = manager.soft_scan(&targets);
        if wmi_changed || ddcci_changed || soft_changed {
            manager.emit_changed();
        }
    }

    /// Returns `None` when no monitor exposes brightness through WMI (the usual case for
    /// desktops with external displays), which is not an error.
    fn wmi_query(wmi: &WMIConnection) -> Result<Option<Vec<WmiMonitorBrightness>>> {
        match wmi.query() {
            Ok(instances) => Ok(Some(instances)),
            Err(wmi::WMIError::HResultError { hres }) if hres == WBEM_E_NOT_SUPPORTED => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    /// WMI identifies the panels by PnP device, so each instance is paired with the monitor
    /// scanning out to that device. Instances without a connected monitor (e.g. a panel still
    /// cached by the provider) are skipped.
    fn wmi_read(targets: &[ScanTarget]) -> Result<Vec<WmiPanel>> {
        let wmi = WMIConnection::with_namespace_path("ROOT\\WMI")?;
        let instances = Self::wmi_query(&wmi)?.unwrap_or_default();
        if instances.is_empty() {
            return Ok(Vec::new());
        }

        let devices: Vec<(&ScanTarget, Vec<String>)> = targets
            .iter()
            .map(|target| {
                let paths = target.monitor.target_device_paths().unwrap_or_default();
                let ids = paths.iter().map(|p| pnp_id_from_device_path(p)).collect();
                (target, ids)
            })
            .collect();

        let mut found = Vec::new();
        for instance in instances {
            let device_id = pnp_id_from_wmi_instance(&instance.instance_name);
            let Some((target, _)) = devices.iter().find(|(_, ids)| ids.contains(&device_id)) else {
                log::debug!(
                    "WMI brightness instance without a connected monitor: {}",
                    instance.instance_name
                );
                continue;
            };
            log::debug!(
                "WMI brightness available on {} ({}): {}%",
                target.name,
                target.id,
                instance.current_brightness
            );
            found.push(WmiPanel {
                monitor_id: target.id.clone(),
                instance_name: instance.instance_name,
                percent: instance.current_brightness.min(100),
            });
        }
        Ok(found)
    }

    /// Returns whether the known WMI panels changed.
    fn wmi_scan(&self, targets: &[ScanTarget]) -> bool {
        let found = match Self::wmi_read(targets) {
            Ok(found) => found,
            Err(err) => {
                log::warn!("Failed to read WMI brightness: {err}");
                return false;
            }
        };
        let changed = self.wmi.to_vec() != found;
        self.wmi.replace(found);
        changed
    }

    fn wmi_set(instance_name: &str, percent: u8) -> Result<()> {
        let wmi = WMIConnection::with_namespace_path("ROOT\\WMI")?;

        let instances = wmi.query::<WmiMonitorBrightnessMethods>()?;

        let obj = instances
            .into_iter()
            .find(|v| v.instance_name == instance_name)
            .ok_or("Instance not found")?;

        wmi.exec_instance_method::<WmiMonitorBrightnessMethods, ()>(
            obj.__path,
            "WmiSetBrightness",
            WmiSetBrightnessPayload {
                timeout: 0,
                brightness: percent,
            },
        )?;
        Ok(())
    }

    /// Reads the brightness of a panel and validates the answer. Panels that don't speak
    /// DDC/CI fail the read; panels that answer nonsense are treated the same way, since
    /// writing to them could leave the panel in a bad state. Caller must hold `DDCCI_LOCK`.
    fn ddcci_read(handle: &PhysicalMonitorHandle, api: DdcApi) -> Result<DdcciBrightnessValues> {
        let values = match api {
            DdcApi::Mccs => handle.ddcci_get_brightness()?,
            DdcApi::Vcp => handle.vcp_get_brightness()?,
        };
        if values.max <= values.min || values.current < values.min || values.current > values.max {
            return Err(format!("panel reported an invalid brightness range: {values:?}").into());
        }
        Ok(values)
    }

    /// The high level call first, then the raw VCP request that more monitors answer.
    fn ddcci_probe(handle: &PhysicalMonitorHandle) -> Result<(DdcciBrightnessValues, DdcApi)> {
        match Self::ddcci_read(handle, DdcApi::Mccs) {
            Ok(values) => Ok((values, DdcApi::Mccs)),
            Err(mccs_err) => Self::ddcci_read(handle, DdcApi::Vcp)
                .map(|values| (values, DdcApi::Vcp))
                .map_err(|vcp_err| format!("{mccs_err}; VCP 0x10: {vcp_err}").into()),
        }
    }

    /// Returns whether the known DDC/CI panels changed.
    fn ddcci_scan(&self, targets: &[ScanTarget]) -> bool {
        let _guard = DDCCI_LOCK.lock();
        let mut found = Vec::new();
        for target in targets {
            let ScanTarget { monitor, id, name } = target;
            if self.ddcci_ignored.contains(id) {
                continue;
            }
            let Ok(panels) = monitor.physical_monitors() else {
                continue;
            };
            for handle in panels {
                let description = handle.description();
                match Self::ddcci_probe(&handle) {
                    Ok((values, api)) => {
                        log::debug!(
                            "DDC/CI brightness available on {name} / {description} ({id}) via {api:?}: {values:?}"
                        );
                        found.push(DdcciPanel {
                            monitor_id: id.clone(),
                            handle: Arc::new(handle),
                            values,
                            api,
                            misses: 0,
                        });
                    }
                    Err(err) => {
                        log::debug!(
                            "DDC/CI brightness unavailable on {name} / {description} ({id}): {err}"
                        );
                    }
                }
            }
        }
        // stable sort: the panels of a cloned group keep their order
        found.sort_by(|a, b| a.monitor_id.0.cmp(&b.monitor_id.0));

        let previous = self.ddcci.to_vec();
        let changed = previous.len() != found.len()
            || previous
                .iter()
                .zip(&found)
                .any(|(a, b)| a.monitor_id != b.monitor_id || a.values != b.values);
        self.ddcci.replace(found);
        changed
    }

    /// Re-reads the known DDC/CI panels to catch changes made from the monitor's OSD.
    /// A failed read (monitor asleep or unplugged) keeps the last known value; hotplug is
    /// handled by the rescan triggered from `MonitorManager`.
    fn ddcci_poll() {
        let manager = Self::instance();
        let guard = DDCCI_LOCK.lock();
        let mut changed = false;
        for known in manager.ddcci.to_vec() {
            let Ok(values) = Self::ddcci_read(&known.handle, known.api) else {
                continue;
            };
            if values != known.values {
                changed = true;
                manager.ddcci.for_each(|panel| {
                    if Arc::ptr_eq(&panel.handle, &known.handle) {
                        panel.values = values;
                    }
                });
            }
        }
        drop(guard);

        if changed {
            manager.emit_changed();
        }
    }

    /// Returns whether the monitor has any DDC/CI panel.
    fn ddcci_set(&self, id: &MonitorId, percent: u8) -> Result<bool> {
        let guard = DDCCI_LOCK.lock();
        // resolve under the lock: a rescan may have replaced the handles meanwhile
        let panels: Vec<DdcciPanel> = self
            .ddcci
            .to_vec()
            .into_iter()
            .filter(|panel| &panel.monitor_id == id)
            .collect();

        for target in &panels {
            let raw = target.raw_from_percent(percent);
            let written = match target.api {
                DdcApi::Mccs => target.handle.ddcci_set_brightness(raw),
                DdcApi::Vcp => target.handle.vcp_set_brightness(raw),
            };
            if let Err(err) = written {
                log::debug!("DDC/CI brightness write failed on {id}: {err}");
                continue;
            }
            self.ddcci.for_each(|panel| {
                if Arc::ptr_eq(&panel.handle, &target.handle) {
                    panel.values.current = raw;
                }
            });
        }
        drop(guard);

        if !panels.is_empty() {
            self.emit_changed();
            // failed writes are caught here too: the read back won't match
            let _ = self.worker_tx.send(WorkerMessage::Verify(id.clone()));
        }
        Ok(!panels.is_empty())
    }

    /// Reads the panels back after a write and compares with the last value written. A panel
    /// that keeps reporting something else is not obeying DDC/CI, so its monitor falls back
    /// to software dimming.
    fn ddcci_verify(id: &MonitorId) {
        let manager = Self::instance();
        let guard = DDCCI_LOCK.lock();
        let mut ignoring = None;
        for known in manager.ddcci.to_vec() {
            if &known.monitor_id != id {
                continue;
            }
            let expected = known.values.current;
            let obeyed = Self::ddcci_read(&known.handle, known.api)
                .is_ok_and(|values| values.current.abs_diff(expected) <= 1);
            manager.ddcci.for_each(|panel| {
                if Arc::ptr_eq(&panel.handle, &known.handle) {
                    panel.misses = if obeyed { 0 } else { panel.misses + 1 };
                    if panel.misses >= DDCCI_MAX_MISSES {
                        ignoring = Some(panel.percent());
                    }
                }
            });
        }
        drop(guard);

        if let Some(percent) = ignoring {
            log::warn!(
                "Monitor {id} ignores DDC/CI brightness writes, switching to software dimming"
            );
            manager.ddcci_ignored.push(id.clone());
            Self::scan();
            manager.soft_set(id, percent).log_error();
        }
    }

    fn emit_changed(&self) {
        Self::send(BrightnessManagerEvent::Changed);
    }

    /// Brightness of the monitor as a percentage, `None` if it can't be controlled.
    pub fn get_brightness(&self, id: &MonitorId) -> Option<u8> {
        self.wmi
            .find_and_clone(|panel| &panel.monitor_id == id)
            .map(|panel| panel.percent)
            .or_else(|| {
                self.ddcci
                    .find_and_clone(|panel| &panel.monitor_id == id)
                    .map(|panel| panel.percent())
            })
            .or_else(|| {
                self.soft
                    .find_and_clone(|panel| &panel.monitor_id == id)
                    .map(|panel| panel.percent)
            })
    }

    /// Sets the brightness of every panel behind the monitor. `percent` goes from 0 to 100
    /// and is mapped to whatever range each panel works with.
    pub fn set_brightness(&self, id: &MonitorId, percent: u8) -> Result<()> {
        let percent = percent.min(100);

        let mut supported = false;
        for panel in self.wmi.to_vec() {
            if &panel.monitor_id == id {
                supported = true;
                // the cached value is updated by the WMI notification
                Self::wmi_set(&panel.instance_name, percent)?;
            }
        }
        supported |= self.ddcci_set(id, percent)?;

        if !supported {
            return self.soft_set(id, percent);
        }
        Ok(())
    }

    /// Monitors that answered neither WMI nor DDC/CI get software dimming. Returns whether
    /// the known software panels changed.
    fn soft_scan(&self, targets: &[ScanTarget]) -> bool {
        static SCANNED: AtomicBool = AtomicBool::new(false);
        let first_scan = !SCANNED.swap(true, Ordering::AcqRel);
        let wmi = self.wmi.to_vec();
        let ddcci = self.ddcci.to_vec();
        let previous = self.soft.to_vec();

        let mut found = Vec::new();
        for target in targets {
            let hardware = wmi.iter().any(|p| p.monitor_id == target.id)
                || ddcci.iter().any(|p| p.monitor_id == target.id);
            if hardware {
                continue;
            }
            let Ok(info) = target.monitor.info() else {
                continue;
            };
            let device =
                crate::windows_api::string_utils::WindowsString::from_slice(&info.szDevice)
                    .to_string();
            let percent = previous
                .iter()
                .find(|p| p.monitor_id == target.id)
                .map_or(100, |p| p.percent);
            // a display change can reset the ramp, put the dimming back. On the first scan the
            // ramp is reset instead, in case a crash left the screen dimmed
            if percent < 100 {
                set_gamma_dimming(&device, soft_factor(percent)).log_error();
            } else if first_scan {
                set_gamma_dimming(&device, 1.0).log_error();
            }
            log::debug!(
                "Software brightness on {} ({}) through {device}",
                target.name,
                target.id
            );
            found.push(SoftPanel {
                monitor_id: target.id.clone(),
                device,
                percent,
            });
        }

        // monitors that left the list (now reachable over DDC/CI, or unplugged) get their
        // gamma back, the hardware brightness takes over
        for gone in previous
            .iter()
            .filter(|p| !found.iter().any(|f| f.monitor_id == p.monitor_id))
        {
            if gone.percent < 100 {
                set_gamma_dimming(&gone.device, 1.0).log_error();
            }
        }

        let changed = previous != found;
        self.soft.replace(found);
        changed
    }

    /// Some monitor is on software dimming without having been caught ignoring DDC/CI.
    fn has_reprobable(&self) -> bool {
        let ignored = self.ddcci_ignored.to_vec();
        self.soft.any(|panel| !ignored.contains(&panel.monitor_id))
    }

    fn soft_set(&self, id: &MonitorId, percent: u8) -> Result<()> {
        let panel = self
            .soft
            .find_and_clone(|panel| &panel.monitor_id == id)
            .ok_or("Monitor does not support brightness control")?;
        set_gamma_dimming(&panel.device, soft_factor(percent))?;
        if percent < 100 {
            SOFT_DIMMED.store(true, Ordering::Release);
        }
        self.soft.for_each(|p| {
            if &p.monitor_id == id {
                p.percent = percent;
            }
        });
        self.emit_changed();
        Ok(())
    }
}

fn soft_factor(percent: u8) -> f64 {
    (percent.min(100) as f64 / 100.0).max(SOFT_MIN_FACTOR)
}

/// Which dxva2 call a DDC/CI panel answers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DdcApi {
    /// `GetMonitorBrightness` / `SetMonitorBrightness`, needs a capabilities string
    Mccs,
    /// raw VCP feature 0x10
    Vcp,
}

/// A display dimmed through its gamma ramp, brightness is the percent last asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
struct SoftPanel {
    monitor_id: MonitorId,
    /// GDI device name, e.g. `\\.\DISPLAY1`
    device: String,
    percent: u8,
}

enum WorkerMessage {
    /// A display was added/removed, re-enumerate monitors.
    Rescan,
    /// Check that the panels of a monitor took the brightness just written.
    Verify(MonitorId),
}

/// A connected monitor to look for panels on.
struct ScanTarget {
    monitor: Monitor,
    id: MonitorId,
    name: String,
}

/// One panel reachable through WMI, which already works with percentages.
#[derive(Debug, Clone, PartialEq, Eq)]
struct WmiPanel {
    monitor_id: MonitorId,
    /// e.g. `DISPLAY\BOE0900\4&10fd3ab1&0&UID265988_0`
    instance_name: String,
    percent: u8,
}

/// One physical panel reachable through DDC/CI. Brightness is exposed as a percentage
/// regardless of the raw range reported by the panel.
#[derive(Debug, Clone)]
struct DdcciPanel {
    monitor_id: MonitorId,
    handle: Arc<PhysicalMonitorHandle>,
    values: DdcciBrightnessValues,
    api: DdcApi,
    /// writes in a row the panel didn't apply
    misses: u8,
}

impl DdcciPanel {
    fn percent(&self) -> u8 {
        let DdcciBrightnessValues { min, current, max } = self.values;
        let span = max.saturating_sub(min).max(1);
        let current = current.clamp(min, max) - min;
        ((current as u64 * 100 + span as u64 / 2) / span as u64) as u8
    }

    fn raw_from_percent(&self, percent: u8) -> u32 {
        let DdcciBrightnessValues { min, max, .. } = self.values;
        let span = max.saturating_sub(min) as u64;
        min + ((percent.min(100) as u64 * span + 50) / 100) as u32
    }
}

/// PnP device instance id out of a device interface path:
/// `\\?\DISPLAY#BOE0900#4&10fd3ab1&0&UID265988#{e6f07b5f-...}` becomes
/// `DISPLAY\BOE0900\4&10FD3AB1&0&UID265988`.
fn pnp_id_from_device_path(path: &str) -> String {
    let path = path.strip_prefix(r"\\?\").unwrap_or(path);
    let path = path.split_once("#{").map_or(path, |(id, _guid)| id);
    path.replace('#', "\\").to_uppercase()
}

/// PnP device instance id out of a WMI instance name, which is the id plus an `_<index>`
/// suffix: `DISPLAY\BOE0900\4&10fd3ab1&0&UID265988_0` becomes
/// `DISPLAY\BOE0900\4&10FD3AB1&0&UID265988`.
fn pnp_id_from_wmi_instance(instance_name: &str) -> String {
    let id = match instance_name.rsplit_once('_') {
        Some((id, index)) if !index.is_empty() && index.bytes().all(|b| b.is_ascii_digit()) => id,
        _ => instance_name,
    };
    id.to_uppercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wmi_instance_matches_device_path() {
        let path =
            r"\\?\DISPLAY#BOE0900#4&10fd3ab1&0&UID265988#{e6f07b5f-ee97-4a90-b076-33f57bf4eaa7}";
        let instance = r"DISPLAY\BOE0900\4&10fd3ab1&0&UID265988_0";
        assert_eq!(
            pnp_id_from_device_path(path),
            r"DISPLAY\BOE0900\4&10FD3AB1&0&UID265988"
        );
        assert_eq!(
            pnp_id_from_wmi_instance(instance),
            pnp_id_from_device_path(path)
        );
    }

    #[test]
    fn wmi_instance_without_index_is_kept() {
        assert_eq!(
            pnp_id_from_wmi_instance(r"DISPLAY\LGD_05E5\4&abc&0&UID8388688"),
            r"DISPLAY\LGD_05E5\4&ABC&0&UID8388688"
        );
    }
}
