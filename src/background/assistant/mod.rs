//! AI assistant of the settings app. Talks to a model over its API (OpenAI, Anthropic,
//! OpenRouter, any OpenAI-compatible server) and can read and change settings through tools.
//! Every answer that changed something is preceded by a settings snapshot, so the showcase
//! undo restores it.

mod providers;

use seelen_core::{
    state::AssistantProvider,
    system_state::{AssistantAction, AssistantMessage, AssistantReply},
};
use serde_json::{Map, Value, json};
use windows::{
    Security::Credentials::{PasswordCredential, PasswordVault},
    core::HSTRING,
};

use crate::{
    error::Result,
    resources::RESOURCES,
    showcase::{apply_settings, ensure_settings, snapshot_settings, undo},
    state::application::FULL_STATE,
};

use providers::{Conversation, Endpoint, Tool, ToolCall, Turn};

const VAULT: &str = "KestrelUI Assistant";
const MAX_STEPS: usize = 8;
/// what the assistant may read and change, the rest (shortcuts, monitors, per-app rules,
/// updater, dev tools) stays out of its reach
const EDITABLE: &[&str] = &[
    "byWidget",
    "byTheme",
    "byWallpaper",
    "activeThemes",
    "activeIconPacks",
    "dateFormat",
    "startOfWeek",
    "performanceMode",
    "language",
];
const MAX_TOOL_OUTPUT: usize = 12_000;

const SYSTEM_PROMPT: &str = r#"You are the assistant built into KestrelUI, a desktop environment for Windows (a fork of Seelen UI). You help the user customize it and answer questions about it.

What KestrelUI has, by widget id:
- @seelen/fancy-toolbar: the top bar with modules (clock, tray, media, network...)
- @seelen/weg: the dock that replaces the taskbar
- @seelen/window-manager: tiling window manager
- @seelen/wallpaper-manager: animated wallpapers; can also show the Windows wallpaper
- @seelen/apps-menu: the start menu
- themes (@kestrel/neon-moss is the default one: dark glass with a neon accent), icon packs, wallpapers, and a Showcase of setups other users published on GitHub.

Settings app sections: Showcase, General (language, autostart, neon color), Resources (themes, widgets, icon packs, wallpapers), Shortcuts, Monitors, Settings by application, Extras.

How to change things:
- Use the tools. Before changing a setting, read it with get_settings(path) to see the exact field names and value format. Never invent field names.
- Change only what was asked, with the smallest patch. Patches are deep-merged; arrays are replaced whole.
- Theme look goes through byTheme.<theme id>.<css variable>. list_resources("themes") shows each theme's variables. Neon Moss: --neon-color (hex like #5ef0a0), --neon-icon-follow ("1" icons use the neon color, "0" use --neon-icon-color), --neon-icon-color (hex), --neon-glass (0 to 1, panel opacity), --neon-glow.
- A theme or icon pack is turned on by adding its id to activeThemes / activeIconPacks.
- After a change, say briefly what you changed; the user can revert it with the Undo button under your message.
- You can't change keyboard shortcuts, monitor layouts or per-app rules, and you can't install Showcase entries or turn autostart on. Tell the user where in the settings app to do that.

Reply in the language of the user's last message. Keep answers short and concrete."#;

fn vault_user(provider: AssistantProvider) -> &'static str {
    match provider {
        AssistantProvider::OpenAI => "openai",
        AssistantProvider::Anthropic => "anthropic",
        AssistantProvider::OpenRouter => "openrouter",
        AssistantProvider::Custom => "custom",
    }
}

fn key(provider: AssistantProvider) -> Option<String> {
    let vault = PasswordVault::new().ok()?;
    let cred = vault
        .Retrieve(&HSTRING::from(VAULT), &HSTRING::from(vault_user(provider)))
        .ok()?;
    cred.RetrievePassword().ok()?;
    Some(cred.Password().ok()?.to_string())
}

fn set_key(provider: AssistantProvider, value: Option<String>) -> Result<()> {
    let vault = PasswordVault::new()?;
    let user = HSTRING::from(vault_user(provider));
    if let Ok(cred) = vault.Retrieve(&HSTRING::from(VAULT), &user) {
        vault.Remove(&cred)?;
    }
    let Some(value) = value.map(|v| v.trim().to_owned()).filter(|v| !v.is_empty()) else {
        return Ok(());
    };
    let cred = PasswordCredential::CreatePasswordCredential(
        &HSTRING::from(VAULT),
        &user,
        &HSTRING::from(value),
    )?;
    vault.Add(&cred)?;
    Ok(())
}

fn endpoint() -> Result<Endpoint> {
    let cfg = FULL_STATE.load().settings.assistant.clone();
    let model = cfg.model.trim().to_owned();
    if model.is_empty() {
        return Err("No model selected".into());
    }
    let base = Endpoint::base_for(cfg.provider, &cfg.base_url);
    if base.is_empty() {
        return Err("No server address".into());
    }
    let key = key(cfg.provider);
    let needs_key = matches!(
        cfg.provider,
        AssistantProvider::OpenAI | AssistantProvider::Anthropic | AssistantProvider::OpenRouter
    );
    if needs_key && key.is_none() {
        return Err("No API key".into());
    }
    Ok(Endpoint {
        provider: cfg.provider,
        base,
        key,
        model,
    })
}

// ---------------------------------------------------------------------------------------
// tools

fn tool_definitions() -> Vec<Tool> {
    vec![
        Tool {
            name: "get_settings",
            description: "Read the user's current KestrelUI settings. Without a path returns an overview: plain values as they are, nested objects as their list of keys. With a dot path like \"byWidget.@seelen/fancy-toolbar\" or \"byTheme.@kestrel/neon-moss\" returns that part in full.",
            parameters: json!({
                "type": "object",
                "properties": { "path": { "type": "string", "description": "dot separated path, empty for the overview" } },
            }),
        },
        Tool {
            name: "update_settings",
            description: "Deep-merge a partial settings object into the current settings and apply it right away. Top-level keys allowed: byWidget, byTheme, byWallpaper, activeThemes, activeIconPacks, dateFormat, startOfWeek, performanceMode, language. Example: {\"byWidget\": {\"@seelen/fancy-toolbar\": {\"height\": 32}}}",
            parameters: json!({
                "type": "object",
                "properties": { "patch": { "type": "object", "description": "partial settings to merge" } },
                "required": ["patch"],
            }),
        },
        Tool {
            name: "list_resources",
            description: "List installed themes (with their configurable css variables), icon packs, wallpapers or widgets. Pass id to get one widget's settings declaration.",
            parameters: json!({
                "type": "object",
                "properties": {
                    "kind": { "type": "string", "enum": ["themes", "iconPacks", "wallpapers", "widgets"] },
                    "id": { "type": "string", "description": "optional, a widget id for its settings declaration" },
                },
                "required": ["kind"],
            }),
        },
    ]
}

#[derive(Default)]
struct Session {
    changed: bool,
    actions: Vec<AssistantAction>,
}

impl Session {
    fn run(&mut self, call: &ToolCall) -> String {
        let result = match call.name.as_str() {
            "get_settings" => get_settings(call.args["path"].as_str().unwrap_or_default()),
            "update_settings" => self.update_settings(&call.args),
            "list_resources" => list_resources(
                call.args["kind"].as_str().unwrap_or_default(),
                call.args["id"].as_str(),
            ),
            other => Err(format!("unknown tool {other}").into()),
        };
        let out = match result {
            Ok(out) => out,
            Err(err) => format!("error: {err}"),
        };
        if out.len() > MAX_TOOL_OUTPUT {
            let mut cut = MAX_TOOL_OUTPUT;
            while !out.is_char_boundary(cut) {
                cut -= 1;
            }
            return format!("{}\n... (cut, ask for a narrower path)", &out[..cut]);
        }
        out
    }

    fn update_settings(&mut self, args: &Value) -> Result<String> {
        // small models like to send the object as a JSON string
        let patch = match &args["patch"] {
            Value::String(s) => serde_json::from_str(s)?,
            other => other.clone(),
        };
        let Value::Object(mut patch) = patch else {
            return Err("patch must be an object".into());
        };
        if let Some(bad) = patch.keys().find(|k| !EDITABLE.contains(&k.as_str())) {
            return Err(format!("{bad} can't be changed, allowed: {}", EDITABLE.join(", ")).into());
        }
        if let Some(Value::Object(widgets)) = patch.get_mut("byWidget") {
            for cfg in widgets.values_mut() {
                if let Value::Object(cfg) = cfg {
                    cfg.remove("$shortcuts");
                    cfg.remove("shortcuts");
                }
            }
        }

        let mut leaves = Vec::new();
        collect_leaves(&Value::Object(patch.clone()), &mut Vec::new(), &mut leaves);
        let summary = leaves
            .iter()
            .map(|(path, _)| path.join("."))
            .collect::<Vec<_>>()
            .join(", ");

        if !self.changed {
            snapshot_settings()?;
        }
        if let Err(err) = apply_settings(Value::Object(patch)) {
            if !self.changed {
                // drop the snapshot taken for nothing
                let _ = undo();
            }
            self.actions.push(AssistantAction {
                tool: "update_settings".into(),
                summary,
                ok: false,
            });
            return Err(err);
        }
        self.changed = true;

        // settings ignore unknown fields and fix invalid values, so check what stuck
        let now = serde_json::to_value(&FULL_STATE.load().settings)?;
        let ignored: Vec<String> = leaves
            .iter()
            .filter(|(path, value)| !lookup(&now, path).is_some_and(|v| same(v, value)))
            .map(|(path, _)| path.join("."))
            .collect();

        self.actions.push(AssistantAction {
            tool: "update_settings".into(),
            summary: summary.clone(),
            ok: ignored.is_empty(),
        });
        if ignored.is_empty() {
            Ok(format!("applied: {summary}"))
        } else {
            Ok(format!(
                "applied, but these were ignored (unknown field or invalid value, check with get_settings): {}",
                ignored.join(", ")
            ))
        }
    }

    fn reply(self, content: String) -> AssistantReply {
        AssistantReply {
            content,
            actions: self.actions,
            changed_settings: self.changed,
        }
    }
}

fn editable_settings() -> Result<Value> {
    let all = serde_json::to_value(&FULL_STATE.load().settings)?;
    let mut out = Map::new();
    for key in EDITABLE {
        if let Some(v) = all.get(*key) {
            out.insert((*key).to_owned(), v.clone());
        }
    }
    Ok(Value::Object(out))
}

/// plain values and flat objects as they are, anything deeper as its list of keys
fn overview(value: &Value) -> Value {
    let Value::Object(map) = value else {
        return value.clone();
    };
    let shown: Map<String, Value> = map
        .iter()
        .map(|(k, v)| {
            let v = match v {
                Value::Object(inner) if inner.values().any(|x| x.is_object() || x.is_array()) => {
                    json!({ "keys": inner.keys().collect::<Vec<_>>() })
                }
                _ => v.clone(),
            };
            (k.clone(), v)
        })
        .collect();
    Value::Object(shown)
}

fn get_settings(path: &str) -> Result<String> {
    let settings = editable_settings()?;
    let path = path.trim().trim_matches('.');
    if path.is_empty() {
        return Ok(serde_json::to_string(&overview(&settings))?);
    }
    let segments: Vec<String> = path.split('.').map(str::to_owned).collect();
    let Some(found) = lookup(&settings, &segments) else {
        return Err(format!("nothing at {path}, top-level keys: {}", EDITABLE.join(", ")).into());
    };
    let full = serde_json::to_string(found)?;
    if full.len() > MAX_TOOL_OUTPUT {
        return Ok(serde_json::to_string(&overview(found))?);
    }
    Ok(full)
}

fn lookup<'a>(value: &'a Value, path: &[String]) -> Option<&'a Value> {
    path.iter().try_fold(value, |v, key| v.get(key))
}

/// 1 and 1.0 are the same value for a setting
fn same(a: &Value, b: &Value) -> bool {
    match (a.as_f64(), b.as_f64()) {
        (Some(x), Some(y)) => (x - y).abs() < 1e-9,
        _ => a == b,
    }
}

fn collect_leaves(value: &Value, path: &mut Vec<String>, out: &mut Vec<(Vec<String>, Value)>) {
    match value {
        Value::Object(map) if !map.is_empty() => {
            for (k, v) in map {
                path.push(k.clone());
                collect_leaves(v, path, out);
                path.pop();
            }
        }
        _ => out.push((path.clone(), value.clone())),
    }
}

fn list_resources(kind: &str, id: Option<&str>) -> Result<String> {
    let state = FULL_STATE.load();
    let lang = state.settings.language.clone();
    let active_themes: Vec<String> = state
        .settings
        .active_themes
        .iter()
        .map(|t| t.to_string())
        .collect();
    let active_packs: Vec<String> = state
        .settings
        .active_icon_packs
        .iter()
        .map(|p| p.to_string())
        .collect();

    let list: Vec<Value> = match kind {
        "themes" => RESOURCES
            .themes()
            .iter()
            .map(|t| {
                json!({
                    "id": t.id,
                    "name": t.metadata.display_name.get(&lang),
                    "description": t.metadata.description.get(&lang),
                    "active": active_themes.contains(&t.id.to_string()),
                    "variables": t.settings,
                })
            })
            .collect(),
        "iconPacks" => RESOURCES
            .icon_packs()
            .iter()
            .map(|p| {
                json!({
                    "id": p.id,
                    "name": p.metadata.display_name.get(&lang),
                    "active": active_packs.contains(&p.id.to_string()),
                })
            })
            .collect(),
        "wallpapers" => RESOURCES
            .wallpapers()
            .iter()
            .map(|w| {
                json!({
                    "id": w.id,
                    "name": w.metadata.display_name.get(&lang),
                    "type": w.r#type,
                })
            })
            .collect(),
        "widgets" => RESOURCES
            .widgets()
            .iter()
            .filter(|w| id.is_none_or(|id| w.id.to_string() == id))
            .map(|w| {
                let mut entry = json!({
                    "id": w.id,
                    "name": w.metadata.display_name.get(&lang),
                    "description": w.metadata.description.get(&lang),
                });
                if id.is_some() {
                    entry["settings"] = serde_json::to_value(&w.settings).unwrap_or_default();
                }
                entry
            })
            .collect(),
        other => return Err(format!("unknown kind {other}").into()),
    };
    Ok(serde_json::to_string(&list)?)
}

// ---------------------------------------------------------------------------------------
// handlers

impl crate::tauri_handlers::Handlers {
    pub async fn assistant_chat(
        webview: tauri::WebviewWindow,
        messages: Vec<AssistantMessage>,
    ) -> Result<AssistantReply> {
        ensure_settings(&webview)?;
        let tools = tool_definitions();
        let mut conv = Conversation::new(endpoint()?, SYSTEM_PROMPT.to_owned(), &messages);
        let mut session = Session::default();

        for _ in 0..MAX_STEPS {
            let turn = match conv.next(&tools).await {
                Ok(turn) => turn,
                // keep the undo button reachable when something was changed before the error
                Err(err) if session.changed => return Ok(session.reply(format!("Error: {err}"))),
                Err(err) => return Err(err),
            };
            match turn {
                Turn::Text(content) => return Ok(session.reply(content)),
                Turn::Tools(calls) => {
                    let results = calls
                        .into_iter()
                        .map(|call| {
                            let out = session.run(&call);
                            (call, out)
                        })
                        .collect();
                    conv.push_results(results);
                }
            }
        }
        Ok(session.reply("Stopped: too many steps for one answer.".into()))
    }

    pub fn assistant_set_key(
        webview: tauri::WebviewWindow,
        provider: AssistantProvider,
        key: Option<String>,
    ) -> Result<()> {
        ensure_settings(&webview)?;
        set_key(provider, key)
    }

    pub fn assistant_has_key(
        webview: tauri::WebviewWindow,
        provider: AssistantProvider,
    ) -> Result<bool> {
        ensure_settings(&webview)?;
        Ok(key(provider).is_some())
    }

    pub async fn assistant_list_models(webview: tauri::WebviewWindow) -> Result<Vec<String>> {
        ensure_settings(&webview)?;
        let cfg = FULL_STATE.load().settings.assistant.clone();
        let ep = Endpoint {
            provider: cfg.provider,
            base: Endpoint::base_for(cfg.provider, &cfg.base_url),
            key: key(cfg.provider),
            model: cfg.model,
        };
        providers::list_models(&ep).await
    }
}
