//! GitHub sign-in (device flow) and publishing to the showcase as a pull request.
//! Only needed to publish: browsing the showcase never asks for an account.

use std::{
    io::Cursor,
    sync::LazyLock,
    time::{Duration, Instant},
};

use base64::Engine;
use seelen_core::system_state::{GithubDeviceCode, GithubUser, ShowcaseSubmission};
use serde::Deserialize;
use serde_json::{Value, json};
use windows::{
    Security::Credentials::{PasswordCredential, PasswordVault},
    core::HSTRING,
};

use crate::{
    error::Result,
    state::application::FULL_STATE,
    utils::{http, lock_free::TracedMutex},
};

const CLIENT_ID: Option<&str> = option_env!("KESTREL_GITHUB_CLIENT_ID");
const API: &str = "https://api.github.com";
const VAULT: &str = "KestrelUI GitHub";
const SHAREABLE: &[&str] = &[
    "byWidget",
    "byTheme",
    "activeThemes",
    "activeIconPacks",
    "dateFormat",
    "startOfWeek",
    "performanceMode",
    "pollingInterval",
];

static PENDING: LazyLock<TracedMutex<Option<Pending>>> = LazyLock::new(|| TracedMutex::new(None));
static PREVIEW: LazyLock<TracedMutex<Option<Vec<u8>>>> = LazyLock::new(|| TracedMutex::new(None));

struct Pending {
    device_code: String,
    interval: u64,
    deadline: Instant,
}

#[derive(Deserialize)]
struct DeviceResp {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
}

#[derive(Deserialize)]
struct TokenResp {
    access_token: Option<String>,
    error: Option<String>,
    interval: Option<u64>,
}

pub async fn start_login() -> Result<GithubDeviceCode> {
    let client_id = CLIENT_ID.ok_or("github sign-in is not configured in this build")?;
    let url = url::Url::parse_with_params(
        "https://github.com/login/device/code",
        &[("client_id", client_id), ("scope", "public_repo")],
    )
    .map_err(|e| e.to_string())?;
    let resp: DeviceResp = http::client()
        .post(url)
        .header("Accept", "application/json")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    *PENDING.lock() = Some(Pending {
        device_code: resp.device_code,
        interval: resp.interval.max(5),
        deadline: Instant::now() + Duration::from_secs(resp.expires_in),
    });
    Ok(GithubDeviceCode {
        user_code: resp.user_code,
        verification_uri: resp.verification_uri,
        expires_in: resp.expires_in,
    })
}

pub async fn wait_login() -> Result<GithubUser> {
    let client_id = CLIENT_ID.ok_or("github sign-in is not configured in this build")?;
    let (code, mut interval, deadline) = match PENDING.lock().as_ref() {
        Some(p) => (p.device_code.clone(), p.interval, p.deadline),
        None => return Err("no github sign-in in progress".into()),
    };

    let token = loop {
        if Instant::now() > deadline {
            return Err("github sign-in code expired".into());
        }
        tokio::time::sleep(Duration::from_secs(interval)).await;

        let url = url::Url::parse_with_params(
            "https://github.com/login/oauth/access_token",
            &[
                ("client_id", client_id),
                ("device_code", code.as_str()),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ],
        )
        .map_err(|e| e.to_string())?;
        let resp: TokenResp = http::client()
            .post(url)
            .header("Accept", "application/json")
            .send()
            .await?
            .json()
            .await?;

        if let Some(token) = resp.access_token {
            break token;
        }
        match resp.error.as_deref() {
            Some("authorization_pending") => {}
            Some("slow_down") => interval = resp.interval.unwrap_or(interval + 5),
            Some(err) => {
                PENDING.lock().take();
                return Err(format!("github sign-in failed: {err}").into());
            }
            None => return Err("github sign-in: empty response".into()),
        }
    };

    PENDING.lock().take();
    store_token(&token)?;
    current_user(&token).await
}

pub async fn user() -> Result<Option<GithubUser>> {
    let Some(token) = token() else {
        return Ok(None);
    };
    match current_user(&token).await {
        Ok(user) => Ok(Some(user)),
        Err(err) => {
            log::warn!("github token rejected, signing out: {err}");
            logout();
            Ok(None)
        }
    }
}

pub fn logout() {
    let Ok(vault) = PasswordVault::new() else {
        return;
    };
    if let Ok(cred) = vault.Retrieve(&HSTRING::from(VAULT), &HSTRING::from("token")) {
        vault.Remove(&cred).ok();
    }
}

/// Screenshot of the primary monitor, kept until the next publish. Returns a data url
/// so the settings window can show what is going to be uploaded.
pub fn capture_preview() -> Result<String> {
    let buf = win_screenshot::capture::capture_display()?;
    let img = image::RgbaImage::from_raw(buf.width, buf.height, buf.pixels)
        .ok_or("bad screenshot buffer")?;
    let img = image::DynamicImage::ImageRgba8(img)
        .resize(1280, 720, image::imageops::FilterType::Triangle)
        .to_rgb8();

    let mut jpg = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(Cursor::new(&mut jpg), 82)
        .encode_image(&img)?;

    let url = format!(
        "data:image/jpeg;base64,{}",
        base64::engine::general_purpose::STANDARD.encode(&jpg)
    );
    *PREVIEW.lock() = Some(jpg);
    Ok(url)
}

/// Puts the current setup into the user's own `kestrelui-<name>` repository and tags it
/// so the showcase search finds it. Publishing the same name again updates that repo.
/// Returns the repository url.
pub async fn publish(sub: ShowcaseSubmission) -> Result<String> {
    let token = token().ok_or("sign in with github first")?;
    let name = sub.name.trim();
    if name.is_empty() || name.chars().count() > 48 {
        return Err("name must be 1-48 characters".into());
    }
    let preview = PREVIEW
        .lock()
        .clone()
        .ok_or("take a preview screenshot first")?;
    let description = sub.description.trim();
    let tags: Vec<String> = sub
        .tags
        .iter()
        .map(|t| slug(t))
        .filter(|t| !t.is_empty())
        .take(8)
        .collect();

    let me = current_user(&token).await?;
    let mut base = slug(name);
    if base.is_empty() {
        base = "setup".to_owned();
    }
    let repo = format!("{}/kestrelui-{base}", me.login);

    let res = api(&token, reqwest::Method::POST, "/user/repos")
        .json(&json!({
            "name": repo.split_once('/').map(|(_, n)| n).unwrap_or_default(),
            "description": description,
            "auto_init": true,
        }))
        .send()
        .await?;
    // 422 = already published under this name, update it in place
    if !res.status().is_success() && res.status().as_u16() != 422 {
        return Err(format!("github: can't create {repo}: {}", res.status()).into());
    }

    let manifest = json!({
        "name": name,
        "kind": "preset",
        "description": description,
        "tags": tags,
        "preview": "preview.jpg",
        "settings": "settings.json",
    });
    let settings = shareable(serde_json::to_value(&FULL_STATE.load().settings)?);
    let files = [
        (super::MANIFEST, serde_json::to_vec_pretty(&manifest)?),
        ("settings.json", serde_json::to_vec_pretty(&settings)?),
        ("preview.jpg", preview),
    ];
    for (file, bytes) in files {
        put_file(&token, &repo, file, &bytes).await?;
    }

    let mut topics = vec![super::TOPIC.to_owned(), "kestrelui-preset".to_owned()];
    topics.extend(tags);
    topics.dedup();
    api(
        &token,
        reqwest::Method::PUT,
        &format!("/repos/{repo}/topics"),
    )
    .json(&json!({ "names": topics.into_iter().take(20).collect::<Vec<_>>() }))
    .send()
    .await?
    .error_for_status()?;

    PREVIEW.lock().take();
    Ok(format!("https://github.com/{repo}"))
}

/// Keeps only the settings that make sense to share. Used both when publishing and when
/// installing, since nobody reviews what other people put in their repos.
pub fn shareable(all: Value) -> Value {
    let mut patch = serde_json::Map::new();
    for key in SHAREABLE {
        if let Some(v) = all.get(*key) {
            patch.insert((*key).to_owned(), v.clone());
        }
    }
    if let Some(Value::Object(widgets)) = patch.get_mut("byWidget") {
        for cfg in widgets.values_mut() {
            if let Value::Object(cfg) = cfg {
                cfg.remove("shortcuts");
            }
        }
    }
    Value::Object(patch)
}

fn api(token: &str, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
    http::client()
        .request(method, format!("{API}{path}"))
        .bearer_auth(token)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .timeout(Duration::from_secs(60))
}

async fn current_user(token: &str) -> Result<GithubUser> {
    let me: Value = api(token, reqwest::Method::GET, "/user")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(GithubUser {
        login: me["login"].as_str().ok_or("github: no login")?.to_owned(),
        avatar_url: me["avatar_url"].as_str().unwrap_or_default().to_owned(),
    })
}

async fn put_file(token: &str, repo: &str, path: &str, bytes: &[u8]) -> Result<()> {
    let url = format!("/repos/{repo}/contents/{path}");
    // republishing overwrites, which needs the blob sha of the old file
    let old: Option<Value> = api(token, reqwest::Method::GET, &url)
        .send()
        .await?
        .json()
        .await
        .ok();
    let mut body = json!({
        "message": format!("update {path}"),
        "content": base64::engine::general_purpose::STANDARD.encode(bytes),
    });
    if let Some(sha) = old.as_ref().and_then(|v| v["sha"].as_str()) {
        body["sha"] = json!(sha);
    }
    api(token, reqwest::Method::PUT, &url)
        .json(&body)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

fn slug(s: &str) -> String {
    let mut out = String::new();
    for c in s.to_lowercase().chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    out.trim_end_matches('-').chars().take(40).collect()
}

fn token() -> Option<String> {
    let vault = PasswordVault::new().ok()?;
    let cred = vault
        .Retrieve(&HSTRING::from(VAULT), &HSTRING::from("token"))
        .ok()?;
    cred.RetrievePassword().ok()?;
    Some(cred.Password().ok()?.to_string())
}

fn store_token(token: &str) -> Result<()> {
    logout();
    let vault = PasswordVault::new()?;
    let cred = PasswordCredential::CreatePasswordCredential(
        &HSTRING::from(VAULT),
        &HSTRING::from("token"),
        &HSTRING::from(token),
    )?;
    vault.Add(&cred)?;
    Ok(())
}
