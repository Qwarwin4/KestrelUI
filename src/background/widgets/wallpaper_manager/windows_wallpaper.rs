//! The wallpaper set in Windows, handed to the wallpaper manager as a regular image resource
//! so it can stand in wherever no KestrelUI collection is configured.

use std::{
    path::{Path, PathBuf},
    sync::Once,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use crossbeam_channel::RecvTimeoutError;
use seelen_core::{
    handlers::SeelenEvent,
    state::{Wallpaper, WallpaperKind},
};
use windows::Win32::UI::WindowsAndMessaging::{SPI_SETDESKWALLPAPER, WM_SETTINGCHANGE};

use crate::{
    app::emit_to_webviews,
    error::Result,
    utils::{constants::SEELEN_COMMON, spawn_named_thread},
    windows_api::{WindowsApi, event_window::subscribe_to_background_window},
};

const ID: &str = "@windows/wallpaper";
/// served by the asset protocol from any folder, see tauri.conf.json
const SERVABLE: &[&str] = &["jpg", "jpeg", "png", "webp", "gif"];

/// The image Windows draws right now. When the configured file is gone (slideshows,
/// Spotlight, a deleted picture) TranscodedWallpaper still holds what is on screen
fn source() -> Option<PathBuf> {
    if let Ok(path) = WindowsApi::get_wallpaper()
        && path.is_file()
    {
        return Some(path);
    }
    let transcoded = PathBuf::from(std::env::var_os("APPDATA")?)
        .join(r"Microsoft\Windows\Themes\TranscodedWallpaper");
    transcoded.is_file().then_some(transcoded)
}

fn signature() -> Option<(PathBuf, SystemTime)> {
    let src = source()?;
    let modified = std::fs::metadata(&src).and_then(|m| m.modified()).ok()?;
    Some((src, modified))
}

/// bmp, TranscodedWallpaper (no extension) and the like are copied into our temp folder,
/// which the asset protocol allows. The mtime in the name busts the webview cache
fn servable(src: &Path) -> Result<PathBuf> {
    let ext = src
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase);
    if ext.as_deref().is_some_and(|e| SERVABLE.contains(&e)) {
        return Ok(src.to_path_buf());
    }

    let stamp = std::fs::metadata(src)?
        .modified()?
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let dir = SEELEN_COMMON.app_temp_dir().join("windows-wallpaper");
    let dst = dir.join(format!("{stamp}.{}", ext.as_deref().unwrap_or("jpg")));
    if !dst.exists() {
        std::fs::create_dir_all(&dir)?;
        for old in std::fs::read_dir(&dir)?.flatten() {
            let _ = std::fs::remove_file(old.path());
        }
        std::fs::copy(src, &dst)?;
    }
    Ok(dst)
}

pub fn current() -> Result<Option<Wallpaper>> {
    let Some(src) = source() else {
        // solid color background
        return Ok(None);
    };
    let file = servable(&src)?;
    let (Some(dir), Some(name)) = (file.parent(), file.file_name()) else {
        return Ok(None);
    };

    let mut wallpaper = Wallpaper {
        id: ID.into(),
        r#type: WallpaperKind::Image,
        filename: Some(name.to_string_lossy().into_owned()),
        ..Default::default()
    };
    wallpaper.metadata.internal.path = dir.to_path_buf();
    Ok(Some(wallpaper))
}

/// Started by the first request, emits `WallpaperWindowsChanged` when the picture changes
pub fn watch() {
    static STARTED: Once = Once::new();
    STARTED.call_once(|| {
        let (tx, rx) = crossbeam_channel::unbounded::<()>();
        subscribe_to_background_window(move |msg, w_param, _| {
            if msg == WM_SETTINGCHANGE && w_param as u32 == SPI_SETDESKWALLPAPER.0 {
                let _ = tx.send(());
            }
            Ok(())
        });

        spawn_named_thread("Windows Wallpaper", move || {
            let mut last = signature();
            loop {
                // slideshows don't always broadcast the change, so look every half minute too
                if let Err(RecvTimeoutError::Disconnected) =
                    rx.recv_timeout(Duration::from_secs(30))
                {
                    break;
                }
                let now = signature();
                if now == last {
                    continue;
                }
                last = now;
                match current() {
                    Ok(wallpaper) => {
                        emit_to_webviews(SeelenEvent::WallpaperWindowsChanged, wallpaper)
                    }
                    Err(err) => log::error!("Failed to read the Windows wallpaper: {err:?}"),
                }
            }
        });
    });
}
