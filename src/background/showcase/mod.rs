//! Config showcase: a GitHub repository with an `index.json` that lists ready-made
//! presets and community resources. No accounts, no tracking — plain HTTPS reads.

use std::{io::Cursor, path::PathBuf, time::Duration};

use seelen_core::{
    resource::{SluResource, SluResourceFile},
    state::{Dialog, DialogContent, Plugin, Settings, Theme, Widget},
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    cli::uri::{downloading_dialog, error_dialog, install_resource_file, update_dialog_to_added_resource},
    error::Result,
    state::application::{AppSettings, FULL_STATE},
    utils::{constants::SEELEN_COMMON, http},
    widgets::trigger_dialog_backend,
};

pub const REGISTRY: &str = "https://raw.githubusercontent.com/Qwarwin4/winuirev-showcase/main/";
const MAX_SNAPSHOTS: usize = 10;

#[derive(Debug, Clone, Deserialize)]
pub struct Index {
    pub version: u32,
    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub id: String,
    pub kind: String,
    #[serde(default)]
    pub resources: Vec<Asset>,
    pub settings: Option<Asset>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Asset {
    pub path: String,
    pub sha256: String,
}

fn asset_url(path: &str) -> Result<String> {
    let ok = !path.is_empty()
        && !path.starts_with('/')
        && !path.contains("..")
        && !path.contains("://")
        && path
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_./".contains(c));
    if !ok {
        return Err(format!("showcase: rejected asset path {path:?}").into());
    }
    Ok(format!("{REGISTRY}{path}"))
}

async fn fetch(url: &str) -> Result<Vec<u8>> {
    let res = http::get(url).timeout(Duration::from_secs(120)).send().await?;
    if !res.status().is_success() {
        return Err(format!("showcase: {url} -> {}", res.status()).into());
    }
    Ok(res.bytes().await?.to_vec())
}

async fn fetch_asset(asset: &Asset) -> Result<Vec<u8>> {
    let bytes = fetch(&asset_url(&asset.path)?).await?;
    let hash = Sha256::digest(&bytes);
    let hex: String = hash.iter().map(|b| format!("{b:02x}")).collect();
    if !hex.eq_ignore_ascii_case(&asset.sha256) {
        return Err(format!("showcase: checksum mismatch for {}", asset.path).into());
    }
    Ok(bytes)
}

pub async fn fetch_index() -> Result<Index> {
    let bytes = fetch(&format!("{REGISTRY}index.json")).await?;
    let index: Index = serde_json::from_slice(&bytes)?;
    if index.version != 1 {
        return Err("showcase: unsupported index version, update WinUIRev".into());
    }
    Ok(index)
}

fn snapshots_dir() -> PathBuf {
    SEELEN_COMMON.app_data_dir().join("snapshots")
}

fn snapshot_settings() -> Result<()> {
    let dir = snapshots_dir();
    std::fs::create_dir_all(&dir)?;
    let name = format!("{}.json", seelen_core::chrono::Utc::now().format("%Y%m%d-%H%M%S"));
    let json = serde_json::to_vec_pretty(&FULL_STATE.load().settings)?;
    std::fs::write(dir.join(name), json)?;

    let mut old = list_snapshots()?;
    while old.len() > MAX_SNAPSHOTS {
        std::fs::remove_file(old.remove(0)).ok();
    }
    Ok(())
}

fn list_snapshots() -> Result<Vec<PathBuf>> {
    let dir = snapshots_dir();
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut files: Vec<_> = std::fs::read_dir(dir)?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "json"))
        .collect();
    files.sort();
    Ok(files)
}

fn merge(base: &mut serde_json::Value, patch: serde_json::Value) {
    match (base, patch) {
        (serde_json::Value::Object(b), serde_json::Value::Object(p)) => {
            for (k, v) in p {
                merge(b.entry(k).or_insert(serde_json::Value::Null), v);
            }
        }
        (b, p) => *b = p,
    }
}

fn apply_settings(patch: serde_json::Value) -> Result<()> {
    let mut current = serde_json::to_value(&FULL_STATE.load().settings)?;
    merge(&mut current, patch);
    let mut next: Settings = serde_json::from_value(current)?;
    next.sanitize()?;
    FULL_STATE.store(std::sync::Arc::new(AppSettings { settings: next }));
    FULL_STATE.load().write_settings()
}

/// Installs every resource of the entry and merges its settings patch on top of
/// the current settings. A snapshot is taken first so the change can be undone.
pub async fn install(id: &str) -> Result<Vec<SluResourceFile>> {
    let index = fetch_index().await?;
    let entry = index
        .entries
        .into_iter()
        .find(|e| e.id == id)
        .ok_or_else(|| format!("showcase: entry {id:?} not found"))?;

    let mut files = Vec::new();
    let mut bundles = Vec::new();
    for asset in &entry.resources {
        let bytes = fetch_asset(asset).await?;
        if asset.path.ends_with(".yml") {
            bundles.push(bytes);
            continue;
        }
        let mut file =
            tokio::task::spawn_blocking(move || SluResourceFile::decode(Cursor::new(bytes)))
                .await??;
        file.resource.sanitize();
        file.resource.verify()?;
        files.push(file);
    }

    let patch = match &entry.settings {
        Some(asset) => Some(serde_json::from_slice::<serde_json::Value>(&fetch_asset(asset).await?)?),
        None => None,
    };

    for file in &files {
        install_resource_file(file).await?;
    }
    for (i, bytes) in bundles.iter().enumerate() {
        install_bundle(&entry.kind, &format!("showcase-{}-{i}.yml", entry.id), bytes).await?;
    }
    if let Some(patch) = patch {
        snapshot_settings()?;
        apply_settings(patch)?;
    }
    Ok(files)
}

/// Installs a resource bundled with `slu resource bundle` (plain yml).
async fn install_bundle(kind: &str, name: &str, bytes: &[u8]) -> Result<()> {
    let dir = match kind {
        "theme" => SEELEN_COMMON.user_themes_path(),
        "widget" => SEELEN_COMMON.user_widgets_path(),
        "plugin" => SEELEN_COMMON.user_plugins_path(),
        _ => return Err(format!("showcase: yml bundles are not supported for {kind}").into()),
    };
    let path = dir.join(name);
    tokio::fs::write(&path, bytes).await?;

    let checked = match kind {
        "theme" => Theme::load(&path).await.map(|_| ()),
        "widget" => Widget::load(&path).await.map(|_| ()),
        _ => Plugin::load(&path).await.map(|_| ()),
    };
    if let Err(err) = checked {
        tokio::fs::remove_file(&path).await.ok();
        return Err(format!("showcase: invalid {kind} bundle: {err}").into());
    }
    Ok(())
}

fn done_dialog(id: Uuid) -> Dialog {
    Dialog {
        identifier: id,
        title: vec![DialogContent::Text {
            value: t!("resource.added").to_string(),
            styles: None,
        }],
        ..Default::default()
    }
}

pub async fn install_with_dialog(id: &str) -> Result<()> {
    let dialog_id = Uuid::new_v4();
    trigger_dialog_backend(downloading_dialog(dialog_id))?;
    match install(id).await {
        Ok(files) => {
            match files.first() {
                Some(file) => update_dialog_to_added_resource(dialog_id, &file.resource)?,
                None => trigger_dialog_backend(done_dialog(dialog_id))?,
            }
            Ok(())
        }
        Err(err) => {
            trigger_dialog_backend(error_dialog(dialog_id, &format!("{err:?}")))?;
            Err(err)
        }
    }
}

/// Restores the most recent settings snapshot and drops it.
pub fn undo() -> Result<bool> {
    let mut snaps = list_snapshots()?;
    let Some(last) = snaps.pop() else {
        return Ok(false);
    };
    let mut settings: Settings = serde_json::from_slice(&std::fs::read(&last)?)?;
    settings.sanitize()?;
    FULL_STATE.store(std::sync::Arc::new(AppSettings { settings }));
    FULL_STATE.load().write_settings()?;
    std::fs::remove_file(last)?;
    Ok(true)
}

fn ensure_settings(webview: &tauri::WebviewWindow) -> Result<()> {
    use crate::widgets::webview::WidgetWebviewLabel;
    use seelen_core::resource::WidgetId;
    let label = WidgetWebviewLabel::try_from_raw(webview.label())
        .map_err(|_| "Permission denied: caller is not a widget webview.")?;
    if label.widget_id != WidgetId::known_settings() {
        return Err("Permission denied: only the settings app can use the showcase.".into());
    }
    Ok(())
}

impl crate::tauri_handlers::Handlers {
    pub async fn showcase_install(webview: tauri::WebviewWindow, id: String) -> Result<()> {
        ensure_settings(&webview)?;
        let files = install(&id).await?;
        log::info!("showcase: installed {id} ({} resources)", files.len());
        Ok(())
    }

    pub fn showcase_undo(webview: tauri::WebviewWindow) -> Result<bool> {
        ensure_settings(&webview)?;
        undo()
    }
}
