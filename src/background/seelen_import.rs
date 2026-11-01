//! One-time import of an existing Seelen UI setup (NSIS or Microsoft Store install)
//! so switching to KestrelUI keeps settings, themes, widgets, icon packs and wallpapers.

use std::path::{Path, PathBuf};

use tauri::{Manager, path::BaseDirectory};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

use crate::{app::get_app_handle, error::Result, utils::constants::SEELEN_COMMON};

const MARKER: &str = ".seelen-import";
const SKIP: &[&str] = &["logs", "snapshots", MARKER];
const IMPORTED: &str = "imported v2";
const KESTREL_THEME: &str = "@kestrel/neon-moss";

pub fn run() -> Result<()> {
    let data = SEELEN_COMMON.app_data_dir();
    let marker = data.join(MARKER);
    // the first alpha imported without re-enabling the KestrelUI theme, fix those setups once
    if std::fs::read_to_string(&marker).is_ok_and(|m| m == "imported") {
        enable_kestrel_theme(&data.join("settings.json"))?;
        std::fs::write(&marker, IMPORTED)?;
    }
    if marker.exists() || data.join("settings.json").exists() {
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
        enable_kestrel_theme(&data.join("settings.json"))?;
    }
    std::fs::write(marker, if accepted { IMPORTED } else { "declined" })?;
    Ok(())
}

/// Seelen's settings list only Seelen themes, without this the imported setup looks exactly
/// like Seelen UI. User themes stay active, ours goes on top of them
fn enable_kestrel_theme(settings: &Path) -> Result<()> {
    let Ok(text) = std::fs::read_to_string(settings) else {
        return Ok(());
    };
    let mut json: serde_json::Value = serde_json::from_str(&text)?;
    let Some(obj) = json.as_object_mut() else {
        return Ok(());
    };
    let themes = obj
        .entry("activeThemes")
        .or_insert_with(|| serde_json::json!(["@default/theme"]));
    if let Some(list) = themes.as_array_mut()
        && !list.iter().any(|t| t == KESTREL_THEME)
    {
        list.push(KESTREL_THEME.into());
        std::fs::write(settings, serde_json::to_string_pretty(&json)?)?;
    }
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
