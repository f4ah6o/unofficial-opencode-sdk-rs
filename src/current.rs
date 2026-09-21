//! Current (legacy/root) OpenCode API surface.
//!
//! The crate root exposes these APIs directly through [crate::Client]. Complex
//! upstream unions are intentionally represented as `serde_json::Value` while
//! the V1/current contract remains broad and additive.

use std::collections::BTreeMap;

use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::client::SessionApi;
use crate::models::{PromptRequest, Session};
use crate::sse::{EventStream, event_stream_from_response};
use crate::{Client, Error};

fn current_url(
    client: &Client,
    path: &str,
    pairs: impl IntoIterator<Item = (String, String)>,
) -> Result<url::Url, Error> {
    let mut url = client.url(path)?;
    {
        let mut query = url.query_pairs_mut();
        if let Some(directory) = client.configured_directory() {
            query.append_pair("directory", directory);
        }
        if let Some(workspace) = client.configured_workspace() {
            query.append_pair("workspace", workspace);
        }
        for (key, value) in pairs {
            query.append_pair(&key, &value);
        }
    }
    Ok(url)
}

async fn decode_value(client: &Client, response: reqwest::Response) -> Result<Value, Error> {
    client.decode(response).await
}

async fn decode_bool(client: &Client, response: reqwest::Response) -> Result<bool, Error> {
    client.decode(response).await
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SessionListOptions {
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub roots: Option<bool>,
    #[serde(default)]
    pub start: Option<u64>,
    #[serde(default)]
    pub search: Option<String>,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SessionMessagesOptions {
    #[serde(default)]
    pub limit: Option<u32>,
    #[serde(default)]
    pub before: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct PtyCreateRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub args: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct PtyUpdateRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolListOptions {
    pub provider: String,
    pub model: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum VcsDiffMode {
    Git,
    Branch,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VcsDiffOptions {
    pub mode: VcsDiffMode,
    #[serde(default)]
    pub context: Option<u32>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct FindFilesOptions {
    pub query: String,
    #[serde(default)]
    pub dirs: Option<bool>,
    #[serde(default, rename = "type")]
    pub entry_type: Option<String>,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Error,
    Warn,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LogRequest {
    pub service: String,
    pub level: LogLevel,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extra: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct McpAddRequest {
    pub name: String,
    pub config: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderOauthAuthorizeRequest {
    pub method: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inputs: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderOauthCallbackRequest {
    pub method: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PermissionReplyRequest {
    pub reply: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QuestionReplyRequest {
    pub answers: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GlobalHealth {
    pub healthy: bool,
    pub version: String,
}

pub struct GlobalApi<'a> {
    client: &'a Client,
}

impl GlobalApi<'_> {
    pub async fn health(&self) -> Result<GlobalHealth, Error> {
        let response = self
            .client
            .request_base(Method::GET, "global/health")?
            .send()
            .await?;
        self.client.decode(response).await
    }

    pub async fn config(&self) -> Result<Value, Error> {
        let response = self
            .client
            .request_base(Method::GET, "global/config")?
            .send()
            .await?;
        decode_value(self.client, response).await
    }

    pub async fn update_config(&self, body: &Value) -> Result<Value, Error> {
        let response = self
            .client
            .request_base(Method::PATCH, "global/config")?
            .json(body)
            .send()
            .await?;
        decode_value(self.client, response).await
    }

    pub async fn dispose(&self) -> Result<bool, Error> {
        let response = self
            .client
            .request_base(Method::POST, "global/dispose")?
            .send()
            .await?;
        decode_bool(self.client, response).await
    }

    pub async fn upgrade(&self, target: &str) -> Result<Value, Error> {
        let response = self
            .client
            .request_base(Method::POST, "global/upgrade")?
            .json(&serde_json::json!({ "target": target }))
            .send()
            .await?;
        decode_value(self.client, response).await
    }

    pub async fn events(&self) -> Result<EventStream, Error> {
        let response = self
            .client
            .request_base(Method::GET, "global/event")?
            .send()
            .await?;
        if !response.status().is_success() {
            self.client.ensure_success(response).await?;
            unreachable!();
        }
        Ok(event_stream_from_response(response))
    }
}

pub struct ProjectApi<'a> {
    client: &'a Client,
}

impl ProjectApi<'_> {
    pub async fn list(&self) -> Result<Vec<Value>, Error> {
        let response = self.client.request(Method::GET, "project")?.send().await?;
        self.client.decode(response).await
    }

    pub async fn current(&self) -> Result<Value, Error> {
        let response = self
            .client
            .request(Method::GET, "project/current")?
            .send()
            .await?;
        decode_value(self.client, response).await
    }

    pub async fn init_git(&self) -> Result<Value, Error> {
        let response = self
            .client
            .request(Method::POST, "project/git/init")?
            .send()
            .await?;
        decode_value(self.client, response).await
    }

    pub async fn update(&self, project_id: &str, body: &Value) -> Result<Value, Error> {
        let response = self
            .client
            .request(Method::PATCH, &format!("project/{project_id}"))?
            .json(body)
            .send()
            .await?;
        decode_value(self.client, response).await
    }

    pub async fn directories(&self, project_id: &str) -> Result<Value, Error> {
        let response = self
            .client
            .request(Method::GET, &format!("project/{project_id}/directories"))?
            .send()
            .await?;
        decode_value(self.client, response).await
    }
}

pub struct PtyApi<'a> {
    client: &'a Client,
}

impl PtyApi<'_> {
    pub async fn list(&self) -> Result<Vec<Value>, Error> {
        let response = self.client.request(Method::GET, "pty")?.send().await?;
        self.client.decode(response).await
    }

    pub async fn create(&self, body: &PtyCreateRequest) -> Result<Value, Error> {
        let response = self
            .client
            .request(Method::POST, "pty")?
            .json(body)
            .send()
            .await?;
        decode_value(self.client, response).await
    }

    pub async fn get(&self, pty_id: &str) -> Result<Value, Error> {
        let response = self
            .client
            .request(Method::GET, &format!("pty/{pty_id}"))?
            .send()
            .await?;
        decode_value(self.client, response).await
    }

    pub async fn update(&self, pty_id: &str, body: &PtyUpdateRequest) -> Result<Value, Error> {
        let response = self
            .client
            .request(Method::PUT, &format!("pty/{pty_id}"))?
            .json(body)
            .send()
            .await?;
        decode_value(self.client, response).await
    }

    pub async fn remove(&self, pty_id: &str) -> Result<bool, Error> {
        let response = self
            .client
            .request(Method::DELETE, &format!("pty/{pty_id}"))?
            .send()
            .await?;
        decode_bool(self.client, response).await
    }

    pub async fn shells(&self) -> Result<Vec<Value>, Error> {
        let response = self
            .client
            .request(Method::GET, "pty/shells")?
            .send()
            .await?;
        self.client.decode(response).await
    }

    pub async fn connect_token(&self, pty_id: &str) -> Result<Value, Error> {
        let response = self
            .client
            .request(Method::POST, &format!("pty/{pty_id}/connect-token"))?
            .send()
            .await?;
        decode_value(self.client, response).await
    }

    pub async fn connect(
        &self,
        pty_id: &str,
        cursor: Option<&str>,
        ticket: Option<&str>,
    ) -> Result<bool, Error> {
        let mut pairs = Vec::new();
        if let Some(cursor) = cursor {
            pairs.push(("cursor".to_owned(), cursor.to_owned()));
        }
        if let Some(ticket) = ticket {
            pairs.push(("ticket".to_owned(), ticket.to_owned()));
        }
        let url = current_url(self.client, &format!("pty/{pty_id}/connect"), pairs)?;
        let response = self
            .client
            .request_url(Method::GET, url)
            .send()
            .await?;
        decode_bool(self.client, response).await
    }
}

pub struct ConfigApi<'a> {
    client: &'a Client,
}

impl ConfigApi<'_> {
    pub async fn get(&self) -> Result<Value, Error> {
        let response = self.client.request(Method::GET, "config")?.send().await?;
        decode_value(self.client, response).await
    }

    pub async fn update(&self, body: &Value) -> Result<Value, Error> {
        let response = self
            .client
            .request(Method::PATCH, "config")?
            .json(body)
            .send()
            .await?;
        decode_value(self.client, response).await
    }

    pub async fn providers(&self) -> Result<Value, Error> {
        let response = self
            .client
            .request(Method::GET, "config/providers")?
            .send()
            .await?;
        decode_value(self.client, response).await
    }
}

pub struct ToolApi<'a> {
    client: &'a Client,
}

impl ToolApi<'_> {
    pub async fn ids(&self) -> Result<Value, Error> {
        let response = self
            .client
            .request(Method::GET, "experimental/tool/ids")?
            .send()
            .await?;
        decode_value(self.client, response).await
    }

    pub async fn list(&self, options: &ToolListOptions) -> Result<Value, Error> {
        let url = current_url(
            self.client,
            "experimental/tool",
            [
                ("provider".to_owned(), options.provider.clone()),
                ("model".to_owned(), options.model.clone()),
            ],
        )?;
        let response = self
            .client
            .request_url(Method::GET, url)
            .send()
            .await?;
        decode_value(self.client, response).await
    }
}

pub struct InstanceApi<'a> {
    client: &'a Client,
}

impl InstanceApi<'_> {
    pub async fn dispose(&self) -> Result<bool, Error> {
        let response = self
            .client
            .request(Method::POST, "instance/dispose")?
            .send()
            .await?;
        decode_bool(self.client, response).await
    }
}

pub struct PathApi<'a> {
    client: &'a Client,
}

impl PathApi<'_> {
    pub async fn get(&self) -> Result<Value, Error> {
        let response = self.client.request(Method::GET, "path")?.send().await?;
        decode_value(self.client, response).await
    }
}

pub struct VcsApi<'a> {
    client: &'a Client,
}

impl VcsApi<'_> {
    pub async fn get(&self) -> Result<Value, Error> {
        let response = self.client.request(Method::GET, "vcs")?.send().await?;
        decode_value(self.client, response).await
    }

    pub async fn status(&self) -> Result<Vec<Value>, Error> {
        let response = self
            .client
            .request(Method::GET, "vcs/status")?
            .send()
            .await?;
        self.client.decode(response).await
    }

    pub async fn diff(&self, options: &VcsDiffOptions) -> Result<Vec<Value>, Error> {
        let mut pairs = vec![(
            "mode".to_owned(),
            match options.mode {
                VcsDiffMode::Git => "git",
                VcsDiffMode::Branch => "branch",
            }
            .to_owned(),
        )];
        if let Some(context) = options.context {
            pairs.push(("context".to_owned(), context.to_string()));
        }
        let url = current_url(self.client, "vcs/diff", pairs)?;
        let response = self
            .client
            .request_url(Method::GET, url)
            .send()
            .await?;
        self.client.decode(response).await
    }

    pub async fn diff_raw(&self) -> Result<String, Error> {
        let response = self
            .client
            .request(Method::GET, "vcs/diff/raw")?
            .send()
            .await?;
        if !response.status().is_success() {
            self.client.ensure_success(response).await?;
            unreachable!();
        }
        Ok(response.text().await?)
    }

    pub async fn apply(&self, patch: &str) -> Result<bool, Error> {
        #[derive(Deserialize)]
        struct ApplyResponse {
            applied: bool,
        }
        let response = self
            .client
            .request(Method::POST, "vcs/apply")?
            .json(&serde_json::json!({ "patch": patch }))
            .send()
            .await?;
        let value: ApplyResponse = self.client.decode(response).await?;
        Ok(value.applied)
    }
}

pub struct CommandApi<'a> {
    client: &'a Client,
}

impl CommandApi<'_> {
    pub async fn list(&self) -> Result<Vec<Value>, Error> {
        let response = self
            .client
            .request(Method::GET, "command")?
            .send()
            .await?;
        self.client.decode(response).await
    }
}

pub struct ProviderApi<'a> {
    client: &'a Client,
}

impl ProviderApi<'_> {
    pub async fn list(&self) -> Result<Value, Error> {
        let response = self
            .client
            .request(Method::GET, "provider")?
            .send()
            .await?;
        decode_value(self.client, response).await
    }

    pub async fn auth(&self) -> Result<Value, Error> {
        let response = self
            .client
            .request(Method::GET, "provider/auth")?
            .send()
            .await?;
        decode_value(self.client, response).await
    }

    pub fn oauth(&self) -> ProviderOauthApi<'_> {
        ProviderOauthApi {
            client: self.client,
        }
    }
}

pub struct ProviderOauthApi<'a> {
    client: &'a Client,
}

impl ProviderOauthApi<'_> {
    pub async fn authorize(
        &self,
        provider_id: &str,
        body: &ProviderOauthAuthorizeRequest,
    ) -> Result<Value, Error> {
        let response = self
            .client
            .request(
                Method::POST,
                &format!("provider/{provider_id}/oauth/authorize"),
            )?
            .json(body)
            .send()
            .await?;
        decode_value(self.client, response).await
    }

    pub async fn callback(
        &self,
        provider_id: &str,
        body: &ProviderOauthCallbackRequest,
    ) -> Result<bool, Error> {
        let response = self
            .client
            .request(
                Method::POST,
                &format!("provider/{provider_id}/oauth/callback"),
            )?
            .json(body)
            .send()
            .await?;
        decode_bool(self.client, response).await
    }
}

pub struct FindApi<'a> {
    client: &'a Client,
}

impl FindApi<'_> {
    pub async fn text(&self, pattern: &str) -> Result<Vec<Value>, Error> {
        let url = current_url(
            self.client,
            "find",
            [("pattern".to_owned(), pattern.to_owned())],
        )?;
        let response = self
            .client
            .request_url(Method::GET, url)
            .send()
            .await?;
        self.client.decode(response).await
    }

    pub async fn files(&self, options: &FindFilesOptions) -> Result<Vec<String>, Error> {
        let mut pairs = vec![("query".to_owned(), options.query.clone())];
        if let Some(dirs) = options.dirs {
            pairs.push(("dirs".to_owned(), dirs.to_string()));
        }
        if let Some(entry_type) = &options.entry_type {
            pairs.push(("type".to_owned(), entry_type.clone()));
        }
        if let Some(limit) = options.limit {
            pairs.push(("limit".to_owned(), limit.to_string()));
        }
        let url = current_url(self.client, "find/file", pairs)?;
        let response = self
            .client
            .request_url(Method::GET, url)
            .send()
            .await?;
        self.client.decode(response).await
    }

    pub async fn symbols(&self, query: &str) -> Result<Vec<Value>, Error> {
        let url = current_url(
            self.client,
            "find/symbol",
            [("query".to_owned(), query.to_owned())],
        )?;
        let response = self
            .client
            .request_url(Method::GET, url)
            .send()
            .await?;
        self.client.decode(response).await
    }
}

pub struct FileApi<'a> {
    client: &'a Client,
}

impl FileApi<'_> {
    pub async fn list(&self, path: &str) -> Result<Vec<Value>, Error> {
        let url = current_url(
            self.client,
            "file",
            [("path".to_owned(), path.to_owned())],
        )?;
        let response = self
            .client
            .request_url(Method::GET, url)
            .send()
            .await?;
        self.client.decode(response).await
    }

    pub async fn read(&self, path: &str) -> Result<Value, Error> {
        let url = current_url(
            self.client,
            "file/content",
            [("path".to_owned(), path.to_owned())],
        )?;
        let response = self
            .client
            .request_url(Method::GET, url)
            .send()
            .await?;
        decode_value(self.client, response).await
    }

    pub async fn status(&self) -> Result<Vec<Value>, Error> {
        let response = self
            .client
            .request(Method::GET, "file/status")?
            .send()
            .await?;
        self.client.decode(response).await
    }
}

pub struct AppApi<'a> {
    client: &'a Client,
}

impl AppApi<'_> {
    pub async fn log(&self, body: &LogRequest) -> Result<bool, Error> {
        let response = self
            .client
            .request(Method::POST, "log")?
            .json(body)
            .send()
            .await?;
        decode_bool(self.client, response).await
    }

    pub async fn agents(&self) -> Result<Vec<Value>, Error> {
        let response = self.client.request(Method::GET, "agent")?.send().await?;
        self.client.decode(response).await
    }

    pub async fn skills(&self) -> Result<Vec<Value>, Error> {
        let response = self.client.request(Method::GET, "skill")?.send().await?;
        self.client.decode(response).await
    }
}

pub struct McpApi<'a> {
    client: &'a Client,
}

impl McpApi<'_> {
    pub async fn status(&self) -> Result<BTreeMap<String, Value>, Error> {
        let response = self.client.request(Method::GET, "mcp")?.send().await?;
        self.client.decode(response).await
    }

    pub async fn add(&self, body: &McpAddRequest) -> Result<BTreeMap<String, Value>, Error> {
        let response = self
            .client
            .request(Method::POST, "mcp")?
            .json(body)
            .send()
            .await?;
        self.client.decode(response).await
    }

    pub async fn connect(&self, name: &str) -> Result<bool, Error> {
        let response = self
            .client
            .request(Method::POST, &format!("mcp/{name}/connect"))?
            .send()
            .await?;
        decode_bool(self.client, response).await
    }

    pub async fn disconnect(&self, name: &str) -> Result<bool, Error> {
        let response = self
            .client
            .request(Method::POST, &format!("mcp/{name}/disconnect"))?
            .send()
            .await?;
        decode_bool(self.client, response).await
    }

    pub fn auth(&self) -> McpAuthApi<'_> {
        McpAuthApi {
            client: self.client,
        }
    }
}

pub struct McpAuthApi<'a> {
    client: &'a Client,
}

impl McpAuthApi<'_> {
    pub async fn start(&self, name: &str) -> Result<Value, Error> {
        let response = self
            .client
            .request(Method::POST, &format!("mcp/{name}/auth"))?
            .send()
            .await?;
        decode_value(self.client, response).await
    }

    pub async fn remove(&self, name: &str) -> Result<Value, Error> {
        let response = self
            .client
            .request(Method::DELETE, &format!("mcp/{name}/auth"))?
            .send()
            .await?;
        decode_value(self.client, response).await
    }

    pub async fn callback(&self, name: &str, code: &str) -> Result<Value, Error> {
        let response = self
            .client
            .request(Method::POST, &format!("mcp/{name}/auth/callback"))?
            .json(&serde_json::json!({ "code": code }))
            .send()
            .await?;
        decode_value(self.client, response).await
    }

    pub async fn authenticate(&self, name: &str) -> Result<Value, Error> {
        let response = self
            .client
            .request(Method::POST, &format!("mcp/{name}/auth/authenticate"))?
            .send()
            .await?;
        decode_value(self.client, response).await
    }
}

pub struct LspApi<'a> {
    client: &'a Client,
}

impl LspApi<'_> {
    pub async fn status(&self) -> Result<Vec<Value>, Error> {
        let response = self.client.request(Method::GET, "lsp")?.send().await?;
        self.client.decode(response).await
    }
}

pub struct FormatterApi<'a> {
    client: &'a Client,
}

impl FormatterApi<'_> {
    pub async fn status(&self) -> Result<Vec<Value>, Error> {
        let response = self
            .client
            .request(Method::GET, "formatter")?
            .send()
            .await?;
        self.client.decode(response).await
    }
}

pub struct AuthApi<'a> {
    client: &'a Client,
}

impl AuthApi<'_> {
    pub async fn set(&self, provider_id: &str, body: &Value) -> Result<bool, Error> {
        let response = self
            .client
            .request_base(Method::PUT, &format!("auth/{provider_id}"))?
            .json(body)
            .send()
            .await?;
        decode_bool(self.client, response).await
    }

    pub async fn remove(&self, provider_id: &str) -> Result<bool, Error> {
        let response = self
            .client
            .request_base(Method::DELETE, &format!("auth/{provider_id}"))?
            .send()
            .await?;
        decode_bool(self.client, response).await
    }
}

pub struct PermissionApi<'a> {
    client: &'a Client,
}

impl PermissionApi<'_> {
    pub async fn list(&self) -> Result<Vec<Value>, Error> {
        let response = self
            .client
            .request(Method::GET, "permission")?
            .send()
            .await?;
        self.client.decode(response).await
    }

    pub async fn reply(
        &self,
        request_id: &str,
        body: &PermissionReplyRequest,
    ) -> Result<bool, Error> {
        let response = self
            .client
            .request(Method::POST, &format!("permission/{request_id}/reply"))?
            .json(body)
            .send()
            .await?;
        decode_bool(self.client, response).await
    }
}

pub struct QuestionApi<'a> {
    client: &'a Client,
}

impl QuestionApi<'_> {
    pub async fn list(&self) -> Result<Vec<Value>, Error> {
        let response = self
            .client
            .request(Method::GET, "question")?
            .send()
            .await?;
        self.client.decode(response).await
    }

    pub async fn reply(
        &self,
        request_id: &str,
        body: &QuestionReplyRequest,
    ) -> Result<bool, Error> {
        let response = self
            .client
            .request(Method::POST, &format!("question/{request_id}/reply"))?
            .json(body)
            .send()
            .await?;
        decode_bool(self.client, response).await
    }

    pub async fn reject(&self, request_id: &str) -> Result<bool, Error> {
        let response = self
            .client
            .request(Method::POST, &format!("question/{request_id}/reject"))?
            .send()
            .await?;
        decode_bool(self.client, response).await
    }
}

pub struct TuiApi<'a> {
    client: &'a Client,
}

impl TuiApi<'_> {
    async fn post_bool(&self, path: &str, body: Option<&Value>) -> Result<bool, Error> {
        let mut request = self.client.request(Method::POST, path)?;
        if let Some(body) = body {
            request = request.json(body);
        }
        decode_bool(self.client, request.send().await?).await
    }

    pub async fn append_prompt(&self, text: &str) -> Result<bool, Error> {
        self.post_bool("tui/append-prompt", Some(&serde_json::json!({ "text": text })))
            .await
    }

    pub async fn open_help(&self) -> Result<bool, Error> {
        self.post_bool("tui/open-help", None).await
    }

    pub async fn open_sessions(&self) -> Result<bool, Error> {
        self.post_bool("tui/open-sessions", None).await
    }

    pub async fn open_themes(&self) -> Result<bool, Error> {
        self.post_bool("tui/open-themes", None).await
    }

    pub async fn open_models(&self) -> Result<bool, Error> {
        self.post_bool("tui/open-models", None).await
    }

    pub async fn submit_prompt(&self) -> Result<bool, Error> {
        self.post_bool("tui/submit-prompt", None).await
    }

    pub async fn clear_prompt(&self) -> Result<bool, Error> {
        self.post_bool("tui/clear-prompt", None).await
    }

    pub async fn execute_command(&self, command: &str) -> Result<bool, Error> {
        self.post_bool(
            "tui/execute-command",
            Some(&serde_json::json!({ "command": command })),
        )
        .await
    }

    pub async fn show_toast(&self, body: &Value) -> Result<bool, Error> {
        self.post_bool("tui/show-toast", Some(body)).await
    }

    pub async fn publish(&self, body: &Value) -> Result<bool, Error> {
        self.post_bool("tui/publish", Some(body)).await
    }

    pub async fn select_session(&self, session_id: &str) -> Result<bool, Error> {
        self.post_bool(
            "tui/select-session",
            Some(&serde_json::json!({ "sessionID": session_id })),
        )
        .await
    }

    pub fn control(&self) -> TuiControlApi<'_> {
        TuiControlApi {
            client: self.client,
        }
    }
}

pub struct TuiControlApi<'a> {
    client: &'a Client,
}

impl TuiControlApi<'_> {
    pub async fn next(&self) -> Result<Value, Error> {
        let response = self
            .client
            .request(Method::GET, "tui/control/next")?
            .send()
            .await?;
        decode_value(self.client, response).await
    }

    pub async fn response(&self, body: &Value) -> Result<bool, Error> {
        let response = self
            .client
            .request(Method::POST, "tui/control/response")?
            .json(body)
            .send()
            .await?;
        decode_bool(self.client, response).await
    }
}

impl SessionApi<'_> {
    pub async fn list_with(&self, options: &SessionListOptions) -> Result<Vec<Session>, Error> {
        let mut pairs = Vec::new();
        if let Some(scope) = &options.scope {
            pairs.push(("scope".to_owned(), scope.clone()));
        }
        if let Some(path) = &options.path {
            pairs.push(("path".to_owned(), path.clone()));
        }
        if let Some(roots) = options.roots {
            pairs.push(("roots".to_owned(), roots.to_string()));
        }
        if let Some(start) = options.start {
            pairs.push(("start".to_owned(), start.to_string()));
        }
        if let Some(search) = &options.search {
            pairs.push(("search".to_owned(), search.clone()));
        }
        if let Some(limit) = options.limit {
            pairs.push(("limit".to_owned(), limit.to_string()));
        }
        let url = current_url(self.client, "session", pairs)?;
        let response = self
            .client
            .request_url(Method::GET, url)
            .send()
            .await?;
        self.client.decode(response).await
    }

    pub async fn status(&self) -> Result<BTreeMap<String, Value>, Error> {
        let response = self
            .client
            .request(Method::GET, "session/status")?
            .send()
            .await?;
        self.client.decode(response).await
    }

    pub async fn delete(&self, session_id: &str) -> Result<bool, Error> {
        let response = self
            .client
            .request(Method::DELETE, &format!("session/{session_id}"))?
            .send()
            .await?;
        decode_bool(self.client, response).await
    }

    pub async fn update(&self, session_id: &str, body: &Value) -> Result<Session, Error> {
        let response = self
            .client
            .request(Method::PATCH, &format!("session/{session_id}"))?
            .json(body)
            .send()
            .await?;
        self.client.decode(response).await
    }

    pub async fn children(&self, session_id: &str) -> Result<Vec<Session>, Error> {
        let response = self
            .client
            .request(Method::GET, &format!("session/{session_id}/children"))?
            .send()
            .await?;
        self.client.decode(response).await
    }

    pub async fn todo(&self, session_id: &str) -> Result<Vec<Value>, Error> {
        let response = self
            .client
            .request(Method::GET, &format!("session/{session_id}/todo"))?
            .send()
            .await?;
        self.client.decode(response).await
    }

    pub async fn init(&self, session_id: &str, body: &Value) -> Result<bool, Error> {
        let response = self
            .client
            .request(Method::POST, &format!("session/{session_id}/init"))?
            .json(body)
            .send()
            .await?;
        decode_bool(self.client, response).await
    }

    pub async fn fork(&self, session_id: &str, message_id: Option<&str>) -> Result<Session, Error> {
        let body = match message_id {
            Some(message_id) => serde_json::json!({ "messageID": message_id }),
            None => serde_json::json!({}),
        };
        let response = self
            .client
            .request(Method::POST, &format!("session/{session_id}/fork"))?
            .json(&body)
            .send()
            .await?;
        self.client.decode(response).await
    }

    pub async fn share(&self, session_id: &str) -> Result<Session, Error> {
        let response = self
            .client
            .request(Method::POST, &format!("session/{session_id}/share"))?
            .send()
            .await?;
        self.client.decode(response).await
    }

    pub async fn unshare(&self, session_id: &str) -> Result<Session, Error> {
        let response = self
            .client
            .request(Method::DELETE, &format!("session/{session_id}/share"))?
            .send()
            .await?;
        self.client.decode(response).await
    }

    pub async fn diff(&self, session_id: &str, message_id: Option<&str>) -> Result<Vec<Value>, Error> {
        let mut pairs = Vec::new();
        if let Some(message_id) = message_id {
            pairs.push(("messageID".to_owned(), message_id.to_owned()));
        }
        let url = current_url(self.client, &format!("session/{session_id}/diff"), pairs)?;
        let response = self
            .client
            .request_url(Method::GET, url)
            .send()
            .await?;
        self.client.decode(response).await
    }

    pub async fn summarize(&self, session_id: &str, body: &Value) -> Result<bool, Error> {
        let response = self
            .client
            .request(Method::POST, &format!("session/{session_id}/summarize"))?
            .json(body)
            .send()
            .await?;
        decode_bool(self.client, response).await
    }

    pub async fn messages(
        &self,
        session_id: &str,
        options: &SessionMessagesOptions,
    ) -> Result<Vec<Value>, Error> {
        let mut pairs = Vec::new();
        if let Some(limit) = options.limit {
            pairs.push(("limit".to_owned(), limit.to_string()));
        }
        if let Some(before) = &options.before {
            pairs.push(("before".to_owned(), before.clone()));
        }
        let url = current_url(
            self.client,
            &format!("session/{session_id}/message"),
            pairs,
        )?;
        let response = self
            .client
            .request_url(Method::GET, url)
            .send()
            .await?;
        self.client.decode(response).await
    }

    pub async fn message(&self, session_id: &str, message_id: &str) -> Result<Value, Error> {
        let response = self
            .client
            .request(
                Method::GET,
                &format!("session/{session_id}/message/{message_id}"),
            )?
            .send()
            .await?;
        decode_value(self.client, response).await
    }

    pub async fn prompt_async(&self, session_id: &str, body: &PromptRequest) -> Result<(), Error> {
        let response = self
            .client
            .request(Method::POST, &format!("session/{session_id}/prompt_async"))?
            .json(body)
            .send()
            .await?;
        self.client.ensure_success(response).await
    }

    pub async fn command(&self, session_id: &str, body: &Value) -> Result<Value, Error> {
        let response = self
            .client
            .request(Method::POST, &format!("session/{session_id}/command"))?
            .json(body)
            .send()
            .await?;
        decode_value(self.client, response).await
    }

    pub async fn shell(&self, session_id: &str, body: &Value) -> Result<Value, Error> {
        let response = self
            .client
            .request(Method::POST, &format!("session/{session_id}/shell"))?
            .json(body)
            .send()
            .await?;
        decode_value(self.client, response).await
    }

    pub async fn revert(&self, session_id: &str, body: &Value) -> Result<Session, Error> {
        let response = self
            .client
            .request(Method::POST, &format!("session/{session_id}/revert"))?
            .json(body)
            .send()
            .await?;
        self.client.decode(response).await
    }

    pub async fn unrevert(&self, session_id: &str) -> Result<Session, Error> {
        let response = self
            .client
            .request(Method::POST, &format!("session/{session_id}/unrevert"))?
            .send()
            .await?;
        self.client.decode(response).await
    }

    pub async fn delete_message(&self, session_id: &str, message_id: &str) -> Result<bool, Error> {
        let response = self
            .client
            .request(
                Method::DELETE,
                &format!("session/{session_id}/message/{message_id}"),
            )?
            .send()
            .await?;
        decode_bool(self.client, response).await
    }

    pub async fn update_part(
        &self,
        session_id: &str,
        message_id: &str,
        part_id: &str,
        body: &Value,
    ) -> Result<Value, Error> {
        let response = self
            .client
            .request(
                Method::PATCH,
                &format!("session/{session_id}/message/{message_id}/part/{part_id}"),
            )?
            .json(body)
            .send()
            .await?;
        decode_value(self.client, response).await
    }

    pub async fn delete_part(
        &self,
        session_id: &str,
        message_id: &str,
        part_id: &str,
    ) -> Result<bool, Error> {
        let response = self
            .client
            .request(
                Method::DELETE,
                &format!("session/{session_id}/message/{message_id}/part/{part_id}"),
            )?
            .send()
            .await?;
        decode_bool(self.client, response).await
    }

    pub async fn respond_permission(
        &self,
        session_id: &str,
        permission_id: &str,
        response_value: &str,
    ) -> Result<bool, Error> {
        let response = self
            .client
            .request(
                Method::POST,
                &format!("session/{session_id}/permissions/{permission_id}"),
            )?
            .json(&serde_json::json!({ "response": response_value }))
            .send()
            .await?;
        decode_bool(self.client, response).await
    }
}

impl Client {
    pub fn global(&self) -> GlobalApi<'_> {
        GlobalApi { client: self }
    }

    pub fn project(&self) -> ProjectApi<'_> {
        ProjectApi { client: self }
    }

    pub fn pty(&self) -> PtyApi<'_> {
        PtyApi { client: self }
    }

    pub fn config(&self) -> ConfigApi<'_> {
        ConfigApi { client: self }
    }

    pub fn tool(&self) -> ToolApi<'_> {
        ToolApi { client: self }
    }

    pub fn instance(&self) -> InstanceApi<'_> {
        InstanceApi { client: self }
    }

    pub fn path(&self) -> PathApi<'_> {
        PathApi { client: self }
    }

    pub fn vcs(&self) -> VcsApi<'_> {
        VcsApi { client: self }
    }

    pub fn command(&self) -> CommandApi<'_> {
        CommandApi { client: self }
    }

    pub fn provider(&self) -> ProviderApi<'_> {
        ProviderApi { client: self }
    }

    pub fn find(&self) -> FindApi<'_> {
        FindApi { client: self }
    }

    pub fn file(&self) -> FileApi<'_> {
        FileApi { client: self }
    }

    pub fn app(&self) -> AppApi<'_> {
        AppApi { client: self }
    }

    pub fn mcp(&self) -> McpApi<'_> {
        McpApi { client: self }
    }

    pub fn lsp(&self) -> LspApi<'_> {
        LspApi { client: self }
    }

    pub fn formatter(&self) -> FormatterApi<'_> {
        FormatterApi { client: self }
    }

    pub fn tui(&self) -> TuiApi<'_> {
        TuiApi { client: self }
    }

    pub fn auth(&self) -> AuthApi<'_> {
        AuthApi { client: self }
    }

    pub fn permission(&self) -> PermissionApi<'_> {
        PermissionApi { client: self }
    }

    pub fn question(&self) -> QuestionApi<'_> {
        QuestionApi { client: self }
    }

    pub fn event(&self) -> crate::client::EventsApi<'_> {
        self.events()
    }
}
