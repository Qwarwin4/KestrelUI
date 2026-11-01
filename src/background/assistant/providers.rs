//! The chat APIs the assistant talks to: OpenAI-compatible servers (OpenAI, OpenRouter,
//! DeepSeek, Groq, LM Studio...) and Anthropic's Messages API.

use std::time::Duration;

use seelen_core::{state::AssistantProvider, system_state::AssistantMessage};
use serde_json::{Value, json};

use crate::{error::Result, utils::http};

const ANTHROPIC: &str = "https://api.anthropic.com/v1";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const TIMEOUT: Duration = Duration::from_secs(180);

pub struct Endpoint {
    pub provider: AssistantProvider,
    pub base: String,
    pub key: Option<String>,
    pub model: String,
}

impl Endpoint {
    pub fn base_for(provider: AssistantProvider, custom: &str) -> String {
        match provider {
            AssistantProvider::OpenAI => "https://api.openai.com/v1".into(),
            AssistantProvider::OpenRouter => "https://openrouter.ai/api/v1".into(),
            AssistantProvider::Anthropic => ANTHROPIC.into(),
            AssistantProvider::Custom => custom.trim().trim_end_matches('/').into(),
        }
    }
}

pub struct Tool {
    pub name: &'static str,
    pub description: &'static str,
    pub parameters: Value,
}

pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub args: Value,
}

pub enum Turn {
    Text(String),
    Tools(Vec<ToolCall>),
}

/// History kept in the provider's own format, so tool calls and their results go back
/// exactly the way the provider sent them.
pub struct Conversation {
    endpoint: Endpoint,
    system: String,
    messages: Vec<Value>,
}

impl Conversation {
    pub fn new(endpoint: Endpoint, system: String, history: &[AssistantMessage]) -> Self {
        let messages = history
            .iter()
            .map(|m| {
                let role = match m.role {
                    seelen_core::system_state::AssistantRole::User => "user",
                    seelen_core::system_state::AssistantRole::Assistant => "assistant",
                };
                json!({ "role": role, "content": m.content })
            })
            .collect();
        Self {
            endpoint,
            system,
            messages,
        }
    }

    pub async fn next(&mut self, tools: &[Tool]) -> Result<Turn> {
        match self.endpoint.provider {
            AssistantProvider::Anthropic => self.next_anthropic(tools).await,
            _ => self.next_openai(tools).await,
        }
    }

    pub fn push_results(&mut self, results: Vec<(ToolCall, String)>) {
        match self.endpoint.provider {
            AssistantProvider::Anthropic => {
                let content: Vec<Value> = results
                    .into_iter()
                    .map(|(call, out)| {
                        json!({ "type": "tool_result", "tool_use_id": call.id, "content": out })
                    })
                    .collect();
                self.messages
                    .push(json!({ "role": "user", "content": content }));
            }
            _ => {
                for (call, out) in results {
                    self.messages
                        .push(json!({ "role": "tool", "tool_call_id": call.id, "content": out }));
                }
            }
        }
    }

    fn openai_tools(tools: &[Tool]) -> Vec<Value> {
        tools
            .iter()
            .map(|t| {
                json!({
                    "type": "function",
                    "function": { "name": t.name, "description": t.description, "parameters": t.parameters },
                })
            })
            .collect()
    }

    fn with_system(&self) -> Vec<Value> {
        let mut all = vec![json!({ "role": "system", "content": self.system })];
        all.extend(self.messages.iter().cloned());
        all
    }

    async fn next_openai(&mut self, tools: &[Tool]) -> Result<Turn> {
        let ep = &self.endpoint;
        let body = json!({
            "model": ep.model,
            "messages": self.with_system(),
            "tools": Self::openai_tools(tools),
        });
        let mut req = http::client()
            .post(format!("{}/chat/completions", ep.base))
            .timeout(TIMEOUT)
            .json(&body);
        if let Some(key) = &ep.key {
            req = req.bearer_auth(key);
        }
        if ep.provider == AssistantProvider::OpenRouter {
            req = req
                .header("HTTP-Referer", "https://github.com/Qwarwin4/KestrelUI")
                .header("X-Title", "KestrelUI");
        }

        let res = send(req).await?;
        let msg = &res["choices"][0]["message"];
        let calls: Vec<ToolCall> = msg["tool_calls"]
            .as_array()
            .map(|calls| {
                calls
                    .iter()
                    .map(|c| ToolCall {
                        id: c["id"].as_str().unwrap_or_default().to_owned(),
                        name: c["function"]["name"]
                            .as_str()
                            .unwrap_or_default()
                            .to_owned(),
                        args: parse_args(&c["function"]["arguments"]),
                    })
                    .collect()
            })
            .unwrap_or_default();

        // rebuilt instead of cloned: some providers add fields they refuse to get back
        let mut echo = json!({ "role": "assistant", "content": msg["content"] });
        if !calls.is_empty() {
            echo["tool_calls"] = msg["tool_calls"].clone();
        }
        self.messages.push(echo);

        if calls.is_empty() {
            return Ok(Turn::Text(text_of(&msg["content"])));
        }
        Ok(Turn::Tools(calls))
    }

    async fn next_anthropic(&mut self, tools: &[Tool]) -> Result<Turn> {
        let ep = &self.endpoint;
        let tools: Vec<Value> = tools
            .iter()
            .map(|t| json!({ "name": t.name, "description": t.description, "input_schema": t.parameters }))
            .collect();
        let body = json!({
            "model": ep.model,
            "max_tokens": 2048,
            "system": self.system,
            "messages": self.messages,
            "tools": tools,
        });
        let req = http::client()
            .post(format!("{ANTHROPIC}/messages"))
            .timeout(TIMEOUT)
            .header("x-api-key", ep.key.clone().unwrap_or_default())
            .header("anthropic-version", ANTHROPIC_VERSION)
            .json(&body);

        let res = send(req).await?;
        let blocks = res["content"].as_array().cloned().unwrap_or_default();
        self.messages
            .push(json!({ "role": "assistant", "content": blocks }));

        let calls: Vec<ToolCall> = blocks
            .iter()
            .filter(|b| b["type"] == "tool_use")
            .map(|b| ToolCall {
                id: b["id"].as_str().unwrap_or_default().to_owned(),
                name: b["name"].as_str().unwrap_or_default().to_owned(),
                args: b["input"].clone(),
            })
            .collect();
        if !calls.is_empty() {
            return Ok(Turn::Tools(calls));
        }
        let text = blocks
            .iter()
            .filter_map(|b| b["text"].as_str())
            .collect::<Vec<_>>()
            .join("\n");
        Ok(Turn::Text(clean(&text)))
    }
}

pub async fn list_models(ep: &Endpoint) -> Result<Vec<String>> {
    let (req, list, field) = match ep.provider {
        AssistantProvider::Anthropic => (
            http::client()
                .get(format!("{ANTHROPIC}/models?limit=100"))
                .header("x-api-key", ep.key.clone().unwrap_or_default())
                .header("anthropic-version", ANTHROPIC_VERSION),
            "data",
            "id",
        ),
        _ => {
            let mut req = http::get(&format!("{}/models", ep.base));
            if let Some(key) = &ep.key {
                req = req.bearer_auth(key);
            }
            (req, "data", "id")
        }
    };
    let res = send(req).await?;
    let mut models: Vec<String> = res[list]
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|m| m[field].as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    models.sort();
    Ok(models)
}

/// Turns error bodies into a readable message, every provider nests it differently
async fn send(req: reqwest::RequestBuilder) -> Result<Value> {
    let res = req.send().await?;
    let status = res.status();
    let body = res.text().await?;
    if status.is_success() {
        return Ok(serde_json::from_str(&body)?);
    }
    let parsed: Value = serde_json::from_str(&body).unwrap_or(Value::Null);
    let msg = parsed["error"]["message"]
        .as_str()
        .or_else(|| parsed["error"].as_str())
        .or_else(|| parsed["message"].as_str())
        .map(str::to_owned)
        .unwrap_or_else(|| body.chars().take(300).collect());
    Err(format!("{status}: {msg}").into())
}

/// OpenAI-compatible servers send arguments as a JSON string, some as an object, small
/// models sometimes as broken JSON
fn parse_args(raw: &Value) -> Value {
    match raw {
        Value::String(s) => serde_json::from_str(s).unwrap_or_else(|_| json!({})),
        Value::Object(_) => raw.clone(),
        _ => json!({}),
    }
}

fn text_of(content: &Value) -> String {
    clean(content.as_str().unwrap_or_default())
}

/// drops `<think>` blocks reasoning models leave in the answer
fn clean(text: &str) -> String {
    let mut out = text.to_owned();
    while let (Some(start), Some(end)) = (out.find("<think>"), out.find("</think>")) {
        if end < start {
            break;
        }
        out.replace_range(start..end + "</think>".len(), "");
    }
    out.trim().to_owned()
}
