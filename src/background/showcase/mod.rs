//! Config showcase. Every entry is a public GitHub repository tagged `kestrelui-config`
//! with a `kestrelui.json` manifest at its root; the settings app finds them through
//! GitHub search. Browsing and installing need no account.

mod github;

use std::{io::Cursor, path::PathBuf, time::Duration};

use seelen_core::{
    resource::{SluResource, SluResourceFile},
    state::{Dialog, DialogContent, Plugin, Settings, Theme, Widget},
    system_state::{GithubDeviceCode, GithubUser, ShowcaseSubmission},
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    cli::uri::{
        downloading_dialog, error_dialog, install_resource_file, update_dialog_to_added_resource,
    },
    error::Result,
    state::application::{AppSettings, FULL_STATE},
    utils::{constants::SEELEN_COMMON, http},
    widgets::trigger_dialog_backend,
};

pub const TOPIC: &str = "kestrelui-config";
pub const MANIFEST: &str = "kestrelui.json";
const MAX_SNAPSHOTS: usize = 10;
const MAX_FILE: usize = 20 * 1024 * 1024;

#[derive(Debug, Clone, Deserialize)]
pub struct Manifest {
    pub kind: String,
    #[serde(default)]
    pub resources: Vec<String>,
    pub settings: Option<String>,
}

fn raw_url(repo: &str, path: &str) -> Result<String> {
    let safe = |s: &str, extra: &str| {
        !s.is_empty()
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || extra.contains(c))
    };
    let ok_repo = repo
        .split_once('/')
        .is_some_and(|(owner, name)| safe(owner, "-") && safe(name, "-_.") && !name.contains(".."));
    let ok_path = safe(path, "-_./") && !path.starts_with('/') && !path.contains("..");
    if !ok_repo || !ok_path {
        return Err(format!("showcase: rejected {repo}/{path}").into());
    }
    Ok(format!(
        "https://raw.githubusercontent.com/{repo}/HEAD/{path}"
    ))
}

async fn fetch(url: &str) -> Result<Vec<u8>> {
    let res = http::get(url)
        .timeout(Duration::from_secs(120))
        .send()
        .await?;
    if !res.status().is_success() {
        return Err(format!("showcase: {url} -> {}", res.status()).into());
    }
    let bytes = res.bytes().await?;
    if bytes.len() > MAX_FILE {
        return Err(format!("showcase: {url} is too big").into());
    }
    Ok(bytes.to_vec())
}

fn snapshots_dir() -> PathBuf {
    SEELEN_COMMON.app_data_dir().join("snapshots")
}

pub(crate) fn snapshot_settings() -> Result<()> {
    let dir = snapshots_dir();
    std::fs::create_dir_all(&dir)?;
    let name = format!(
        "{}.json",
        seelen_core::chrono::Utc::now().format("%Y%m%d-%H%M%S")
    );
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

pub(crate) fn apply_settings(patch: serde_json::Value) -> Result<()> {
    let mut current = serde_json::to_value(&FULL_STATE.load().settings)?;
    merge(&mut current, patch);
    let mut next: Settings = serde_json::from_value(current)?;
    next.sanitize()?;
    FULL_STATE.store(std::sync::Arc::new(AppSettings { settings: next }));
    FULL_STATE.load().write_settings()
}

/// Installs every resource of the repo's manifest and merges its settings patch on
/// top of the current settings. A snapshot is taken first so the change can be undone.
pub async fn install(repo: &str) -> Result<Vec<SluResourceFile>> {
    let manifest: Manifest = serde_json::from_slice(&fetch(&raw_url(repo, MANIFEST)?).await?)?;

    let mut files = Vec::new();
    let mut bundles = Vec::new();
    for path in &manifest.resources {
        let bytes = fetch(&raw_url(repo, path)?).await?;
        if path.ends_with(".yml") || path.ends_with(".yaml") {
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

    let patch = match &manifest.settings {
        Some(path) => Some(serde_json::from_slice::<serde_json::Value>(
            &fetch(&raw_url(repo, path)?).await?,
        )?),
        None => None,
    };

    for file in &files {
        install_resource_file(file).await?;
    }
    let slug = repo.replace('/', "-").to_lowercase();
    for (i, bytes) in bundles.iter().enumerate() {
        install_bundle(&manifest.kind, &format!("showcase-{slug}-{i}.yml"), bytes).await?;
    }
    if let Some(patch) = patch {
        snapshot_settings()?;
        apply_settings(github::shareable(patch))?;
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

pub async fn install_with_dialog(repo: &str) -> Result<()> {
    let dialog_id = Uuid::new_v4();
    trigger_dialog_backend(downloading_dialog(dialog_id))?;
    match install(repo).await {
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

pub(crate) fn ensure_settings(webview: &tauri::WebviewWindow) -> Result<()> {
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

    pub async fn showcase_github_user() -> Result<Option<GithubUser>> {
        github::user().await
    }

    pub async fn showcase_github_login(webview: tauri::WebviewWindow) -> Result<GithubDeviceCode> {
        ensure_settings(&webview)?;
        github::start_login().await
    }

    pub async fn showcase_github_login_wait(webview: tauri::WebviewWindow) -> Result<GithubUser> {
        ensure_settings(&webview)?;
        github::wait_login().await
    }

    pub fn showcase_github_logout(webview: tauri::WebviewWindow) -> Result<()> {
        ensure_settings(&webview)?;
        github::logout();
        Ok(())
    }

    pub async fn showcase_capture_preview(webview: tauri::WebviewWindow) -> Result<String> {
        ensure_settings(&webview)?;
        // the settings window would cover the desktop it is supposed to show
        webview.hide()?;
        tokio::time::sleep(Duration::from_millis(600)).await;
        let shot = github::capture_preview();
        webview.show()?;
        webview.set_focus().ok();
        shot
    }

    pub async fn showcase_publish(
        webview: tauri::WebviewWindow,
        submission: ShowcaseSubmission,
    ) -> Result<String> {
        ensure_settings(&webview)?;
        github::publish(submission).await
    }
}
