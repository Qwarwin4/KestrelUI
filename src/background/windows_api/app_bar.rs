use seelen_core::system_state::AppBarEdge;
use std::sync::LazyLock;
use windows::Win32::{
    Foundation::{HWND, LPARAM, RECT},
    Graphics::Gdi::{GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO, MonitorFromWindow},
    UI::{
        Shell::{
            ABE_BOTTOM, ABE_LEFT, ABE_RIGHT, ABE_TOP, ABM_NEW, ABM_REMOVE, ABM_SETPOS,
            ABM_SETSTATE, ABS_ALWAYSONTOP, ABS_AUTOHIDE, APPBARDATA, SHAppBarMessage,
        },
        WindowsAndMessaging::IsWindow,
    },
};

use crate::{error::Result, utils::lock_free::TracedMutex};

// keeps the whole APPBARDATA so lost reservations can be sent again
static REGISTERED_BARS: LazyLock<TracedMutex<Vec<APPBARDATA>>> =
    LazyLock::new(|| TracedMutex::new(Vec::new()));

/// https://learn.microsoft.com/en-us/windows/win32/shell/abm-setstate#parameters
#[derive(Debug, Clone, Copy)]
pub enum AppBarDataState {
    BothOff = 0,
    AutoHide = ABS_AUTOHIDE as isize,
    AlwaysOnTop = ABS_ALWAYSONTOP as isize,
    BothOn = 3,
}

impl From<AppBarDataState> for LPARAM {
    fn from(val: AppBarDataState) -> Self {
        LPARAM(val as isize)
    }
}

impl From<u32> for AppBarDataState {
    fn from(state: u32) -> Self {
        match state {
            0 => AppBarDataState::BothOff,
            ABS_AUTOHIDE => AppBarDataState::AutoHide,
            ABS_ALWAYSONTOP => AppBarDataState::AlwaysOnTop,
            3 => AppBarDataState::BothOn,
            _ => unreachable!(),
        }
    }
}

pub struct AppBarData(pub APPBARDATA);

#[allow(dead_code)]
impl AppBarData {
    pub fn from_handle(hwnd: HWND) -> Self {
        Self(APPBARDATA {
            cbSize: std::mem::size_of::<APPBARDATA>() as u32,
            hWnd: hwnd,
            ..Default::default()
        })
    }

    pub fn set_state(&self, state: AppBarDataState) {
        let mut data = self.0;
        data.lParam = state.into();
        unsafe { SHAppBarMessage(ABM_SETSTATE, &mut data) };
    }

    pub fn set_edge(&mut self, edge: AppBarEdge) {
        self.0.uEdge = match edge {
            AppBarEdge::Left => ABE_LEFT,
            AppBarEdge::Top => ABE_TOP,
            AppBarEdge::Right => ABE_RIGHT,
            AppBarEdge::Bottom => ABE_BOTTOM,
        };
    }

    pub fn set_rect(&mut self, rect: RECT) {
        self.0.rc = rect;
    }

    pub fn register_as_new_bar(&mut self) -> Result<()> {
        let mut data = self.0;
        let mut guard = REGISTERED_BARS.lock();

        match guard.iter_mut().find(|b| b.hWnd == data.hWnd) {
            Some(bar) => *bar = data,
            None => {
                let ok = unsafe { SHAppBarMessage(ABM_NEW, &mut data) };
                if ok == 0 {
                    return Err("Failed to register App Bar".into());
                }
                guard.push(data);
            }
        }

        unsafe { SHAppBarMessage(ABM_SETPOS, &mut data) };
        Ok(())
    }

    pub fn unregister_bar(&mut self) -> Result<()> {
        let mut data = self.0;
        let mut guard = REGISTERED_BARS.lock();
        unsafe { SHAppBarMessage(ABM_REMOVE, &mut data) };
        guard.retain(|b| b.hWnd != data.hWnd);
        Ok(())
    }
}

/// Sends the reservation again for bars whose space is no longer excluded from the
/// work area. Explorer drops it when it restarts, when the native taskbar changes
/// its auto-hide state and on some display changes, and then maximized windows
/// end up under the toolbar/dock.
pub fn reassert_bars(explorer_restarted: bool) {
    let mut guard = REGISTERED_BARS.lock();
    guard.retain(|b| unsafe { IsWindow(Some(b.hWnd)).as_bool() });

    for bar in guard.iter_mut() {
        if explorer_restarted {
            unsafe { SHAppBarMessage(ABM_NEW, bar) };
        } else if reserved(bar) {
            continue;
        }
        log::debug!("re-reserving app bar space for {:?}", bar.hWnd);
        unsafe { SHAppBarMessage(ABM_SETPOS, bar) };
    }
}

fn reserved(bar: &APPBARDATA) -> bool {
    let monitor = unsafe { MonitorFromWindow(bar.hWnd, MONITOR_DEFAULTTONEAREST) };
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if !unsafe { GetMonitorInfoW(monitor, &mut info) }.as_bool() {
        return true;
    }
    let (work, rc) = (info.rcWork, bar.rc);
    match bar.uEdge {
        ABE_TOP => work.top >= rc.bottom,
        ABE_BOTTOM => work.bottom <= rc.top,
        ABE_LEFT => work.left >= rc.right,
        ABE_RIGHT => work.right <= rc.left,
        _ => true,
    }
}
