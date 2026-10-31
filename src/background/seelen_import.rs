//! One-time import of an existing Seelen UI setup (NSIS or Microsoft Store install)
//! so switching to WinUIRev keeps settings, themes, widgets, icon packs and wallpapers.

use std::path::{Path, PathBuf};

use tauri::{Manager, path::BaseDirectory};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

use crate::{app::get_app_handle, error::Result, utils::constants::SEELEN_COMMON};

const MARKER: &str = ".seelen-import";
const SKIP: &[&str] = &["logs", "snapshots", MARKER];

pub fn run() -> Result<()> {
    let data = SEELEN_COMMON.app_data_dir();
    if data.join(MARKER).exists() || data.join("settings.json").exists() {
        return Ok(());
    }
    let Some(src) = find_seelen_dir() else {
        return Ok(());
    };

    let accepted = get_app_handle()
        .dialog()
        .message(t!("seelen_import.body", path = src.display().to_string()))
        .title(t!("seelen_import.title"))
        .kind(MessageDialogKind::Info)
        .buttons(MessageDialogButtons::YesNo)
        .blocking_show();

    if accepted {
        log::info!("importing Seelen UI data from {src:?}");
        copy_dir(&src, data)?;
    }
    std::fs::write(
        data.join(MARKER),
        if accepted { "imported" } else { "declined" },
    )?;
    Ok(())
}

fn find_seelen_dir() -> Option<PathBuf> {
    let path = get_app_handle().path();
    let roaming = path
        .resolve("com.seelen.seelen-ui", BaseDirectory::Config)
        .ok();
    // old store builds kept roaming data inside the package container
    let store = path
        .resolve(
            "Packages/Seelen.SeelenUI_p6yyn03m1894e/LocalCache/Roaming/com.seelen.seelen-ui",
            BaseDirectory::LocalData,
        )
        .ok();

    [roaming, store]
        .into_iter()
        .flatten()
        .find(|dir| dir.join("settings.json").is_file())
}

fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)?.flatten() {
        let name = entry.file_name();
        if SKIP.iter().any(|s| name == *s) {
            continue;
        }
        let from = entry.path();
        let to = dst.join(&name);
        if entry.file_type()?.is_dir() {
            copy_dir(&from, &to)?;
        } else if !to.exists() {
            std::fs::copy(&from, &to)?;
        }
    }
    Ok(())
}
