mod icons_downloader;

use std::{
    ffi::OsStr,
    io::Write,
    path::{Path, PathBuf},
    time::Duration,
};

use seelen_core::{
    resource::{Resource, ResourceId, ResourceKind, SluResource, SluResourceFile},
    state::{CssStyles, Dialog, DialogContent, IconPack, Wallpaper},
};
use tauri::Listener;
use uuid::Uuid;

use crate::{
    app::get_app_handle,
    cli::uri::icons_downloader::download_remote_icons,
    error::Result,
    resources::RESOURCES,
    state::application::FULL_STATE,
    utils::{constants::SEELEN_COMMON, date_based_hex_id, http},
    widgets::trigger_dialog_backend,
};

pub const URI: &str = "winuirev.uri:";

pub async fn process_uri(uri: &str) -> Result<()> {
    log::trace!("Loading URI: {uri}");

    if !uri.starts_with(URI) {
        let path = PathBuf::from(uri);
        if !path.is_file() || path.extension() != Some(OsStr::new("slu")) || !path.exists() {
            return Err("Invalid file to load".into());
        }

        let file = SluResourceFile::load(&path).await?;
        store_file_on_respective_user_folder(&file).await?;

        let dialog_id = Uuid::new_v4();
        trigger_dialog_backend(Dialog {
            identifier: dialog_id,
            ..Default::default()
        })?;
        update_dialog_to_added_resource(dialog_id, &file.resource)?;
        return Ok(());
    }

    let path = uri.trim_start_matches(URI).trim_start_matches("/");

    // showcase/<id> — installs an entry from the WinUIRev showcase registry
    if let Some(id) = path.strip_prefix("showcase/") {
        crate::showcase::install_with_dialog(id.trim_end_matches('/')).await?;
        return Ok(());
    }

    Err("Invalid URI format".into())
}

fn path_by_resource_kind(kind: &ResourceKind) -> &Path {
    match kind {
        ResourceKind::Theme => SEELEN_COMMON.user_themes_path(),
        ResourceKind::IconPack => SEELEN_COMMON.user_icons_path(),
        ResourceKind::Widget => SEELEN_COMMON.user_widgets_path(),
        ResourceKind::Plugin => SEELEN_COMMON.user_plugins_path(),
        ResourceKind::Wallpaper => SEELEN_COMMON.user_wallpapers_path(),
        ResourceKind::SoundPack => SEELEN_COMMON.user_sounds_path(),
    }
}

async fn store_file_on_respective_user_folder(file: &SluResourceFile) -> Result<PathBuf> {
    let mut path_to_store = path_by_resource_kind(&file.resource.kind).to_path_buf();
    if file.resource.kind == ResourceKind::IconPack || file.resource.kind == ResourceKind::Wallpaper
    {
        path_to_store.push(file.resource.id.to_string());
        tokio::fs::create_dir_all(&path_to_store).await?;
        path_to_store.push("metadata.slu");
    } else {
        path_to_store.push(format!("{}.slu", file.resource.id));
    }
    file.store(&path_to_store).await?;
    Ok(path_to_store)
}

pub fn downloading_dialog(dialog_id: Uuid) -> Dialog {
    Dialog {
        identifier: dialog_id,
        title: vec![DialogContent::Group {
            items: vec![
                DialogContent::Icon {
                    name: "TbCloudDownload".to_string(),
                    styles: Some(
                        CssStyles::new()
                            .add("color", "var(--color-blue-800)")
                            .add("height", "1.2rem"),
                    ),
                },
                DialogContent::Text {
                    value: t!("resource.downloading").to_string(),
                    styles: None,
                },
            ],
            styles: Some(CssStyles::new().add("alignItems", "center")),
        }],
        content: vec![
            DialogContent::Text {
                value: t!("resource.downloading_body").to_string(),
                styles: None,
            },
            DialogContent::Loader {
                styles: Some(CssStyles::new().add("marginTop", "12px")),
            },
        ],
        ..Default::default()
    }
}

/// Stores a decoded resource in the user folder and pulls its remote assets.
pub async fn install_resource_file(file: &SluResourceFile) -> Result<()> {
    let saved_path = store_file_on_respective_user_folder(file).await?;

    if file.resource.kind == ResourceKind::IconPack {
        let mut pack = IconPack::load(saved_path.parent().unwrap()).await?;
        download_remote_icons(&mut pack).await?;
    }

    if file.resource.kind == ResourceKind::Wallpaper {
        let mut wallpaper = Wallpaper::load(saved_path.parent().unwrap()).await?;
        download_remote_wallpapers(&mut wallpaper).await?;
    }

    Ok(())
}

pub fn update_dialog_to_added_resource(dialog_id: Uuid, resource: &Resource) -> Result<()> {
    let config = resource_to_dialog(dialog_id, resource)?;
    trigger_dialog_backend(config)?;

    let used_id = ResourceId::Remote(resource.id);
    let kind = resource.kind;

    let event = format!("resource::{}::enable", resource.id);
    get_app_handle().once(event, move |_| {
        RESOURCES.enable_resource(kind, used_id.clone());
    });

    Ok(())
}

fn resource_to_dialog(id: Uuid, resource: &Resource) -> Result<Dialog> {
    let image_styles = CssStyles::new()
        .add("width", "90px")
        .add("minWidth", "90px")
        .add("height", "90px")
        .add("borderRadius", "14px")
        .add("backgroundColor", "var(--color-gray-200)")
        .add("display", "flex")
        .add("alignItems", "center")
        .add("justifyContent", "center");

    let image = if let Some(url) = &resource.metadata.portrait {
        DialogContent::Image {
            href: url.clone(),
            styles: Some(image_styles),
        }
    } else {
        DialogContent::Icon {
            name: "GrStatusUnknown".to_string(),
            styles: Some(image_styles),
        }
    };

    let state = FULL_STATE.load();
    let locale = state.locale();

    Ok(Dialog {
        identifier: id,
        width: 500.0,
        height: 280.0,
        title: vec![DialogContent::Group {
            items: vec![
                DialogContent::Icon {
                    name: "GrCircleInformation".to_string(),
                    styles: Some(
                        CssStyles::new()
                            .add("color", "var(--color-blue-900)")
                            .add("height", "1.2rem"),
                    ),
                },
                DialogContent::Text {
                    value: t!("resource.added").to_string(),
                    styles: None,
                },
            ],
            styles: Some(CssStyles::new().add("alignItems", "center")),
        }],
        content: vec![DialogContent::Group {
            items: vec![
                image,
                DialogContent::Group {
                    items: vec![
                        DialogContent::Text {
                            value: resource.metadata.display_name.get(locale).to_owned(),
                            styles: Some(
                                CssStyles::new()
                                    .add("fontWeight", "bold")
                                    .add("fontSize", "2rem")
                                    .add("lineHeight", "1.2em"),
                            ),
                        },
                        DialogContent::Text {
                            value: resource.metadata.description.get(locale).to_owned(),
                            styles: None,
                        },
                    ],
                    styles: Some(CssStyles::new().add("flexDirection", "column")),
                },
            ],
            styles: None,
        }],
        footer: vec![DialogContent::Button {
            skin: Some("solid".to_string()),
            inner: vec![DialogContent::Text {
                value: t!("resource.enable").to_string(),
                styles: None,
            }],
            on_click: format!("resource::{}::enable", resource.id),
            styles: None,
        }],
    })
}

pub fn error_dialog(id: Uuid, err: &str) -> Dialog {
    Dialog {
        identifier: id,
        title: vec![DialogContent::Group {
            items: vec![
                DialogContent::Icon {
                    name: "BiSolidError".to_string(),
                    styles: Some(
                        CssStyles::new()
                            .add("color", "var(--color-red-600)")
                            .add("height", "1.2rem"),
                    ),
                },
                DialogContent::Text {
                    value: t!("resource.download_failed_title").to_string(),
                    styles: None,
                },
            ],
            styles: Some(CssStyles::new().add("alignItems", "center")),
        }],
        content: vec![
            DialogContent::Text {
                value: t!("resource.download_failed_body").to_string(),
                styles: None,
            },
            DialogContent::Text {
                value: "=-=-=-=-=-=-=-=-=-=-=-=-=-=-=-=".to_string(),
                styles: Some(CssStyles::new().add("color", "var(--color-gray-400)")),
            },
            DialogContent::Text {
                value: format!("Error: {err}"),
                styles: None,
            },
        ],
        ..Default::default()
    }
}

async fn download_remote_wallpapers(wallpaper: &mut Wallpaper) -> Result<()> {
    if wallpaper.url.is_none() && wallpaper.thumbnail_url.is_none() {
        return Ok(());
    }

    let folder_to_store = wallpaper.metadata.directory()?;

    // Download the main wallpaper file
    if let Some(url) = &wallpaper.url {
        let filename = download_remote_asset(url, &folder_to_store).await?;
        wallpaper.filename = Some(filename);
    }

    // Download the thumbnail, it's not essential so failures are only logged
    if let Some(thumbnail_url) = &wallpaper.thumbnail_url {
        match download_remote_asset(thumbnail_url, &folder_to_store).await {
            Ok(thumbnail_filename) => wallpaper.thumbnail_filename = Some(thumbnail_filename),
            Err(err) => log::error!("Failed to download wallpaper thumbnail: {err}"),
        }
    }

    wallpaper.save().await?;
    Ok(())
}

const DOWNLOAD_MAX_ATTEMPTS: u32 = 3;
const DOWNLOAD_RETRY_DELAY: Duration = Duration::from_secs(2);

async fn download_remote_asset(url: &url::Url, folder_to_store: &Path) -> Result<String> {
    if !folder_to_store.is_dir() {
        std::fs::create_dir_all(folder_to_store)?;
    }

    let Some(extension) = url.path().split('.').next_back() else {
        return Err("Could not determine file extension from URL".into());
    };

    for attempt in 1..DOWNLOAD_MAX_ATTEMPTS {
        match try_download_asset(url, folder_to_store, extension).await {
            Ok(filename) => return Ok(filename),
            Err(err) => {
                log::warn!(
                    "Failed to download asset (attempt {attempt}/{DOWNLOAD_MAX_ATTEMPTS}): {err}"
                );
                tokio::time::sleep(DOWNLOAD_RETRY_DELAY).await;
            }
        }
    }

    // Last attempt: let the error propagate directly instead of tracking it manually.
    try_download_asset(url, folder_to_store, extension).await
}

async fn try_download_asset(
    url: &url::Url,
    folder_to_store: &Path,
    extension: &str,
) -> Result<String> {
    // Use a long timeout for large assets (videos can be hundreds of MB; the
    // global 15 s client timeout is too short on slow / CDN-throttled connections).
    let res = http::get(url.as_str())
        .timeout(Duration::from_secs(3600))
        .send()
        .await?;
    if !res.status().is_success() {
        let status = res.status();
        let body = res.text().await.unwrap_or_default();
        return Err(format!("Failed to download asset: {status} - {body}").into());
    }

    let filename = format!("{}.{}", date_based_hex_id(), extension);
    let file_path = folder_to_store.join(&filename);

    // Stream chunks to disk to avoid buffering the whole file in memory.
    let mut file = std::fs::File::create(&file_path)?;
    let mut stream = res;
    while let Some(chunk) = stream.chunk().await? {
        file.write_all(&chunk)?;
    }
    file.flush()?;

    Ok(filename)
}
