use std::path::{Path, PathBuf};

use seelen_core::{
    resource::{IconPackId, ResourceId, ResourceKind, ResourceText, SluResource, SluResourceFile},
    state::{
        Icon, IconPack, IconPackEntry, Plugin, SharedIconPackEntry, Theme, UniqueIconPackEntry,
        Wallpaper, Widget,
    },
};
use tauri_plugin_dialog::DialogExt;

use crate::{
    app::get_app_handle,
    cli::uri::{install_resource_file, path_by_resource_kind},
    error::Result,
    resources::RESOURCES,
    state::application::FULL_STATE,
    utils::date_based_hex_id,
    widgets::permissions::{WidgetPerm, request_widget_permission},
    windows_api::WindowsApi,
};

const RESOURCE_FILES: &[&str] = &["yml", "yaml", "json", "slu"];
const ICON_IMAGES: &[&str] = &["png", "jpg", "jpeg", "webp", "gif", "svg", "ico"];
const USER_CUSTOM_ICONS: &str = "@user/custom-icons";

impl crate::tauri_handlers::Handlers {
    /// Copies a resource from disk (an unpacked folder or a single yml/json/slu file)
    /// into the user folder of its kind, loads and enables it.
    pub async fn import_local_resource(
        webview: tauri::WebviewWindow,
        kind: ResourceKind,
        folder: bool,
    ) -> Result<Option<ResourceId>> {
        request_widget_permission(&webview, WidgetPerm::ModifyConfiguration)?;
        if kind == ResourceKind::SoundPack {
            return Err("Sound packs are not supported yet".into());
        }

        // icon packs and wallpapers point to files next to their metadata,
        // so a lone yml would lose them: those come as a folder or a .slu bundle
        let extensions: &'static [&'static str] = match kind {
            ResourceKind::IconPack | ResourceKind::Wallpaper => &["slu"],
            _ => RESOURCE_FILES,
        };
        let Some(src) = pick(folder, extensions).await? else {
            return Ok(None);
        };

        let target = if has_extension(&src, &["slu"]) {
            let file = SluResourceFile::load(&src).await?;
            if file.resource.kind != kind {
                return Err(
                    format!("This file is a {:?}, not a {kind:?}", file.resource.kind).into(),
                );
            }
            install_resource_file(&file).await?
        } else {
            validate(&kind, &src).await?;
            copy_to_user_folder(&kind, &src).await?
        };

        let id = RESOURCES.load(&kind, &target).await?;
        RESOURCES.emit_kind_changed(&kind)?;
        if let Some(id) = &id {
            enable(kind, id.clone());
        }
        Ok(id)
    }

    /// Turns a folder of pictures into an icon pack. A file name says what the picture is for:
    /// `chrome.png` or `chrome.exe.png` is the icon of chrome.exe, `.pdf.png` the icon of pdf files.
    pub async fn create_icon_pack_from_images(
        webview: tauri::WebviewWindow,
    ) -> Result<Option<ResourceId>> {
        request_widget_permission(&webview, WidgetPerm::ModifyConfiguration)?;
        let Some(src) = pick(true, &[]).await? else {
            return Ok(None);
        };

        let name = src
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "Icons".to_owned());
        let slug = slugify(&name).unwrap_or_else(date_based_hex_id);
        let dir = path_by_resource_kind(&ResourceKind::IconPack).join(format!("local-{slug}"));

        let entries = {
            let dir = dir.clone();
            tokio::task::spawn_blocking(move || copy_icons(&src, &dir)).await??
        };
        if entries.is_empty() {
            let _ = tokio::fs::remove_dir_all(&dir).await;
            return Err(format!("No pictures found, supported: {}", ICON_IMAGES.join(", ")).into());
        }

        let mut pack = IconPack {
            id: IconPackId::from(format!("@user/icons-{slug}").as_str()),
            entries,
            ..Default::default()
        };
        pack.metadata.display_name = ResourceText::En(name);
        pack.metadata.internal.path = dir.clone();
        pack.save().await?;

        let id = RESOURCES.load(&ResourceKind::IconPack, &dir).await?;
        RESOURCES.emit_icon_packs();
        if let Some(id) = &id {
            enable(ResourceKind::IconPack, id.clone());
        }
        Ok(id)
    }

    pub fn open_user_resource_folder(kind: ResourceKind) -> Result<()> {
        let dir = path_by_resource_kind(&kind);
        std::fs::create_dir_all(dir)?;
        WindowsApi::execute(dir.to_string_lossy().to_string(), None, None, false)
    }

    /// Rereads the user folder of a kind, for files dropped there by hand.
    pub async fn reload_user_resources(kind: ResourceKind) -> Result<()> {
        RESOURCES.load_all_of_type(kind, false).await?;
        RESOURCES.emit_kind_changed(&kind)
    }
}

async fn pick(folder: bool, extensions: &'static [&'static str]) -> Result<Option<PathBuf>> {
    let picked = tokio::task::spawn_blocking(move || {
        let dialog = get_app_handle().dialog().file();
        if folder {
            dialog.blocking_pick_folder()
        } else {
            dialog
                .add_filter("KestrelUI", extensions)
                .blocking_pick_file()
        }
    })
    .await?;
    Ok(picked.and_then(|p| p.simplified().into_path().ok()))
}

/// Loads the resource where it is, so a broken one never replaces a working copy.
async fn validate(kind: &ResourceKind, path: &Path) -> Result<()> {
    match kind {
        ResourceKind::Theme => {
            Theme::load(path).await?;
        }
        ResourceKind::Widget => {
            Widget::load(path).await?;
        }
        ResourceKind::Plugin => {
            Plugin::load(path).await?;
        }
        ResourceKind::IconPack => {
            IconPack::load(path).await?;
        }
        ResourceKind::Wallpaper => {
            Wallpaper::load(path).await?;
        }
        ResourceKind::SoundPack => return Err("Sound packs are not supported yet".into()),
    }
    Ok(())
}

async fn copy_to_user_folder(kind: &ResourceKind, src: &Path) -> Result<PathBuf> {
    let root = path_by_resource_kind(kind);
    // already in place, e.g. picked from the folder opened with "open folder"
    if src.starts_with(root) {
        return Ok(src.to_path_buf());
    }

    let dst = root.join(src.file_name().ok_or("Invalid path")?);
    let src = src.to_path_buf();
    let to = dst.clone();
    tokio::task::spawn_blocking(move || -> Result<()> {
        // importing the same thing again updates it
        if to.is_dir() {
            std::fs::remove_dir_all(&to)?;
        }
        if src.is_dir() {
            copy_dir(&src, &to)
        } else {
            std::fs::copy(&src, &to)?;
            Ok(())
        }
    })
    .await??;
    Ok(dst)
}

fn copy_dir(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)?.flatten() {
        let to = dst.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &to)?;
        } else {
            std::fs::copy(entry.path(), &to)?;
        }
    }
    Ok(())
}

fn copy_icons(src: &Path, dir: &Path) -> Result<Vec<IconPackEntry>> {
    if dir.exists() {
        std::fs::remove_dir_all(dir)?;
    }
    let icons_dir = dir.join("icons");
    std::fs::create_dir_all(&icons_dir)?;

    let mut entries = Vec::new();
    for file in std::fs::read_dir(src)?.flatten() {
        let path = file.path();
        if !path.is_file() || !has_extension(&path, ICON_IMAGES) {
            continue;
        }
        let Some(stem) = path.file_stem().map(|s| s.to_string_lossy().to_lowercase()) else {
            continue;
        };
        let file_name = file.file_name().to_string_lossy().to_string();
        std::fs::copy(&path, icons_dir.join(&file_name))?;

        let icon = Icon {
            base: Some(format!("icons/{file_name}")),
            is_aproximately_square: true,
            ..Default::default()
        };

        let entry = match stem.strip_prefix('.') {
            Some(extension) if !extension.is_empty() => {
                IconPackEntry::Shared(SharedIconPackEntry {
                    extension: extension.to_owned(),
                    icon,
                })
            }
            _ => IconPackEntry::Unique(UniqueIconPackEntry {
                umid: None,
                path: Some(PathBuf::from(if stem.ends_with(".exe") {
                    stem
                } else {
                    format!("{stem}.exe")
                })),
                redirect: None,
                icon: Some(icon),
                source_mtime: None,
            }),
        };
        entries.push(entry);
    }
    Ok(entries)
}

/// Enables a freshly loaded resource unless it's already on (a re-import of the same thing).
fn enable(kind: ResourceKind, id: ResourceId) {
    let theme_on = kind == ResourceKind::Theme
        && FULL_STATE
            .load()
            .settings
            .active_themes
            .iter()
            .any(|t| **t == id);
    match kind {
        ResourceKind::IconPack => {
            let pack_id = IconPackId::from(id);
            FULL_STATE.rcu(move |state| {
                let mut state = state.cloned();
                let packs = &mut state.settings.active_icon_packs;
                packs.retain(|p| p != &pack_id);
                // the last pack wins, keep the per-app icons picked by hand above the new pack
                let at = packs
                    .iter()
                    .position(|p| p.as_str() == USER_CUSTOM_ICONS)
                    .unwrap_or(packs.len());
                packs.insert(at, pack_id.clone());
                state
            });
            if let Err(err) = FULL_STATE.load().write_settings() {
                log::error!("Failed to save settings: {err}");
            }
        }
        ResourceKind::Theme if theme_on => {}
        _ => RESOURCES.enable_resource(kind, id),
    }
}

fn has_extension(path: &Path, extensions: &[&str]) -> bool {
    path.extension()
        .map(|e| e.to_string_lossy().to_lowercase())
        .is_some_and(|e| extensions.contains(&e.as_str()))
}

/// Ascii id part for a folder name, `None` when nothing usable is left (e.g. cyrillic names).
fn slugify(name: &str) -> Option<String> {
    let mut slug = String::new();
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            slug.push(c.to_ascii_lowercase());
        } else if !slug.ends_with('-') && !slug.is_empty() {
            slug.push('-');
        }
    }
    let slug = slug.trim_end_matches('-').to_owned();
    (!slug.is_empty()).then_some(slug)
}
