//! Software dimming through the display's gamma ramp, for screens whose backlight can't be
//! reached (no DDC/CI, no WMI): cheap monitors, DisplayLink docks, some HDMI adapters.

use windows::Win32::{
    Graphics::Gdi::{CreateDCW, DeleteDC},
    UI::ColorSystem::SetDeviceGammaRamp,
};
use windows_core::{PCWSTR, w};

use crate::{error::Result, windows_api::string_utils::WindowsString};

/// Windows rejects ramps that stray too far from identity (about half brightness unless
/// `GdiIcmGammaRange` is raised), so the factor is stepped up until the driver accepts it.
/// `device` is the GDI name from `MONITORINFOEXW::szDevice`, e.g. `\\.\DISPLAY1`.
/// Returns the factor that was applied.
pub fn set_gamma_dimming(device: &str, factor: f64) -> Result<f64> {
    let name = WindowsString::from(device);
    let hdc = unsafe { CreateDCW(w!("DISPLAY"), name.as_pcwstr(), PCWSTR::null(), None) };
    if hdc.is_invalid() {
        return Err(format!("can't open display device {device}").into());
    }

    let mut factor = factor.clamp(0.0, 1.0);
    let applied = loop {
        let ramp = ramp(factor);
        if unsafe { SetDeviceGammaRamp(hdc, ramp.as_ptr().cast()) }.as_bool() {
            break Some(factor);
        }
        if factor >= 1.0 {
            break None;
        }
        factor = (factor + 0.05).min(1.0);
    };

    unsafe {
        let _ = DeleteDC(hdc);
    }
    applied.ok_or_else(|| format!("{device} rejected every gamma ramp").into())
}

/// Linear ramp scaled by `factor`, the same curve for red, green and blue.
fn ramp(factor: f64) -> [u16; 768] {
    let mut ramp = [0u16; 768];
    for i in 0..256 {
        let value = (i as f64 * 257.0 * factor).round().min(65535.0) as u16;
        ramp[i] = value;
        ramp[256 + i] = value;
        ramp[512 + i] = value;
    }
    ramp
}
