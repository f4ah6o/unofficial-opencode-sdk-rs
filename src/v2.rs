//! Preview client surface for OpenCode's `/api/*` V2 contract.
//!
//! OpenCode currently ships this contract through the official JavaScript
//! package's `/v2` export while the package itself remains on the 1.x release
//! line. The V2 schema is still evolving, so this module is intentionally
//! isolated from the legacy/current root client.

use std::collections::{BTreeMap, VecDeque};
use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
use futures_core::Stream;
use futures_util::StreamExt;
use futures_util::stream::{self, BoxStream};
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{Error, SseDecoder};

mod discovery;
mod resources;
mod session;
pub use discovery::*;
pub use resources::*;
pub use session::*;

/// OpenCode `/api/*` dialect served by the connected server.
///
/// OpenCode 1.x serves a preview of the V2 contract with `api/health` and
/// `api/question/*`; OpenCode 2.x speaks the same family of routes natively
/// but moved liveness to `api/info`, questions to `api/form`, and a few
/// session operations to experimental paths. The client probes once via
/// [`Client::serve_dialect`] and caches the result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ServeDialect {
    /// OpenCode 1.x `/api/*` preview surface.
    Preview1x,
    /// OpenCode 2.x native `/api/*` surface.
    Native2x,
}

/// V2 client for OpenCode's `/api/*` HTTP surface.
#[derive(Clone)]
pub struct Client {
    inner: crate::Client,
    directory: Option<String>,
    workspace_id: Option<String>,
    dialect: std::sync::OnceLock<ServeDialect>,
}

#[derive(Default)]
pub struct ClientBuilder {
    inner: crate::ClientBuilder,
    directory: Option<String>,
    workspace_id: Option<String>,
}

impl Client {
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    pub(crate) fn from_current(client: crate::Client) -> Self {
        let directory = client.configured_directory().map(str::to_owned);
        let workspace_id = client.configured_workspace().map(str::to_owned);
        Self {
            inner: client,
            directory,
            workspace_id,
            dialect: std::sync::OnceLock::new(),
        }
    }

    pub fn session(&self) -> SessionApi<'_> {
        SessionApi { client: self }
    }

    pub fn events(&self) -> EventsApi<'_> {
        EventsApi { client: self }
    }

    /// Detect which `/api/*` dialect the connected OpenCode server speaks.
    ///
    /// Probes `GET api/info`: OpenCode 2.x answers with a JSON `ServerInfo`
    /// body while OpenCode 1.x serves the HTML app shell there, so a JSON
    /// response containing `version` identifies the native 2.x contract.
    /// The result is cached on this client.
    pub async fn serve_dialect(&self) -> Result<ServeDialect, Error> {
        if let Some(dialect) = self.dialect.get() {
            return Ok(*dialect);
        }
        let response = self
            .inner
            .request_base(Method::GET, "api/info")?
            .send()
            .await?;
        let dialect = match response.status().is_success() {
            true => match response.json::<Value>().await {
                Ok(info) if info.get("version").is_some() => ServeDialect::Native2x,
                _ => ServeDialect::Preview1x,
            },
            false => ServeDialect::Preview1x,
        };
        let _ = self.dialect.set(dialect);
        Ok(dialect)
    }
}

impl From<crate::Client> for Client {
    fn from(client: crate::Client) -> Self {
        Self::from_current(client)
    }
}

impl ClientBuilder {
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.inner = self.inner.base_url(base_url);
        self
    }

    pub fn http_client(mut self, client: reqwest::Client) -> Self {
        self.inner = self.inner.http_client(client);
        self
    }

    /// Set the default V2 location directory.
    pub fn directory(mut self, directory: impl Into<String>) -> Self {
        self.directory = Some(directory.into());
        self
    }

    /// Set the default V2 workspace ID.
    pub fn workspace_id(mut self, workspace_id: impl Into<String>) -> Self {
        self.workspace_id = Some(workspace_id.into());
        self
    }

    pub fn basic_auth(mut self, username: impl Into<String>, password: impl Into<String>) -> Self {
        self.inner = self.inner.basic_auth(username, password);
        self
    }

    pub fn password(mut self, password: impl Into<String>) -> Self {
        self.inner = self.inner.password(password);
        self
    }

    pub fn build(self) -> Result<Client, Error> {
        Ok(Client {
            inner: self.inner.build()?,
            directory: self.directory,
            workspace_id: self.workspace_id,
            dialect: std::sync::OnceLock::new(),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocationRef {
    pub directory: String,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        rename = "workspaceID"
    )]
    pub workspace_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ModelRef {
    pub id: String,
    #[serde(rename = "providerID")]
    pub provider_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub variant: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: String,
    #[serde(default, rename = "parentID")]
    pub parent_id: Option<String>,
    #[serde(rename = "projectID")]
    pub project_id: String,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub model: Option<ModelRef>,
    pub cost: f64,
    pub tokens: TokenUsage,
    pub time: SessionTime,
    /// OpenCode 2.x omits `title` until the session is summarized.
    #[serde(default)]
    pub title: Option<String>,
    pub location: LocationRef,
    #[serde(default)]
    pub subpath: Option<String>,
    #[serde(default)]
    pub revert: Option<Value>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TokenUsage {
    pub input: u64,
    pub output: u64,
    pub reasoning: u64,
    pub cache: CacheUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CacheUsage {
    pub read: u64,
    pub write: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionTime {
    pub created: u64,
    pub updated: u64,
    #[serde(default)]
    pub archived: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct Cursor {
    #[serde(default)]
    pub previous: Option<String>,
    #[serde(default)]
    pub next: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionPage {
    pub data: Vec<Session>,
    pub cursor: Cursor,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Order {
    Asc,
    Desc,
}

#[derive(Debug, Clone, Default, Serialize)]
pub struct ListSessionsOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order: Option<Order>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub directory: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subpath: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CreateSessionRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<ModelRef>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<LocationRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PromptSource {
    pub start: u64,
    pub end: u64,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PromptFileAttachment {
    pub uri: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<PromptSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PromptAgentAttachment {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<PromptSource>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PromptInput {
    pub text: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub files: Option<Vec<PromptFileAttachment>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agents: Option<Vec<PromptAgentAttachment>>,
}

impl PromptInput {
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            files: None,
            agents: None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Delivery {
    Steer,
    Queue,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PromptRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prompt: Option<PromptInput>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub delivery: Option<Delivery>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resume: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionInputAdmitted {
    /// OpenCode 2.x admissions have no durable inbox sequence; `None` there.
    #[serde(default)]
    pub admitted_seq: Option<u64>,
    pub id: String,
    #[serde(rename = "sessionID")]
    pub session_id: String,
    /// 1.x `prompt` object, or the 2.x user `payload`.
    pub prompt: Value,
    pub delivery: Delivery,
    pub time_created: u64,
    #[serde(default)]
    pub promoted_seq: Option<u64>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl SessionInputAdmitted {
    /// Normalize the two admission wire shapes: the 1.x preview returns
    /// `{admittedSeq, prompt, timeCreated}` while 2.x returns the durable
    /// user message `{id, sessionID, type: "user", payload, delivery,
    /// time: {created}}`. Exposed for callers decoding raw envelopes.
    pub fn from_wire(mut data: Value) -> Result<Self, serde_json::Error> {
        let is_2x = data.get("type").and_then(Value::as_str) == Some("user")
            && data.get("payload").is_some();
        if is_2x {
            if let Some(payload) = data.get("payload").cloned() {
                data["prompt"] = payload;
            }
            if let Some(created) = data
                .get("time")
                .and_then(|time| time.get("created"))
                .and_then(Value::as_u64)
            {
                data["timeCreated"] = created.into();
            }
        }
        serde_json::from_value(data)
    }
}

#[derive(Debug, Deserialize)]
struct DataEnvelope<T> {
    data: T,
}

pub struct SessionApi<'a> {
    client: &'a Client,
}

impl SessionApi<'_> {
    pub async fn list(&self, options: &ListSessionsOptions) -> Result<SessionPage, Error> {
        let mut url = self.client.inner.url("api/session")?;
        {
            let mut query = url.query_pairs_mut();
            if let Some(workspace) = options
                .workspace
                .as_deref()
                .or(self.client.workspace_id.as_deref())
            {
                query.append_pair("workspace", workspace);
            }
            if let Some(limit) = options.limit {
                query.append_pair("limit", &limit.to_string());
            }
            if let Some(order) = options.order {
                query.append_pair(
                    "order",
                    match order {
                        Order::Asc => "asc",
                        Order::Desc => "desc",
                    },
                );
            }
            if let Some(search) = options.search.as_deref() {
                query.append_pair("search", search);
            }
            if let Some(directory) = options
                .directory
                .as_deref()
                .or(self.client.directory.as_deref())
            {
                query.append_pair("directory", directory);
            }
            if let Some(project) = options.project.as_deref() {
                query.append_pair("project", project);
            }
            if let Some(subpath) = options.subpath.as_deref() {
                query.append_pair("subpath", subpath);
            }
            if let Some(cursor) = options.cursor.as_deref() {
                query.append_pair("cursor", cursor);
            }
        }
        let response = self
            .client
            .inner
            .request_url(Method::GET, url)
            .send()
            .await?;
        self.client.inner.decode(response).await
    }

    pub async fn create(&self, body: &CreateSessionRequest) -> Result<Session, Error> {
        let mut body = body.clone();
        if body.location.is_none() {
            if let Some(directory) = &self.client.directory {
                body.location = Some(LocationRef {
                    directory: directory.clone(),
                    workspace_id: self.client.workspace_id.clone(),
                });
            }
        }
        let response = self
            .client
            .inner
            .request_base(Method::POST, "api/session")?
            .json(&body)
            .send()
            .await?;
        let envelope: DataEnvelope<Session> = self.client.inner.decode(response).await?;
        Ok(envelope.data)
    }

    pub async fn get(&self, session_id: &str) -> Result<Session, Error> {
        let response = self
            .client
            .inner
            .request_base(Method::GET, &format!("api/session/{session_id}"))?
            .send()
            .await?;
        let envelope: DataEnvelope<Session> = self.client.inner.decode(response).await?;
        Ok(envelope.data)
    }

    pub async fn prompt(
        &self,
        session_id: &str,
        body: &PromptRequest,
    ) -> Result<SessionInputAdmitted, Error> {
        // OpenCode 1.x reads `prompt.text`; OpenCode 2.x reads a top-level
        // `text`. Send both — both contracts ignore the other side's keys.
        let mut payload = serde_json::to_value(body).unwrap_or_default();
        if let (Some(object), Some(prompt)) = (payload.as_object_mut(), body.prompt.as_ref()) {
            object.insert("text".to_owned(), prompt.text.clone().into());
        }
        let response = self
            .client
            .inner
            .request_base(Method::POST, &format!("api/session/{session_id}/prompt"))?
            .json(&payload)
            .send()
            .await?;
        let envelope: DataEnvelope<Value> = self.client.inner.decode(response).await?;
        SessionInputAdmitted::from_wire(envelope.data).map_err(Error::from)
    }

    pub async fn wait(&self, session_id: &str) -> Result<(), Error> {
        let path = match self.client.serve_dialect().await? {
            ServeDialect::Preview1x => format!("api/session/{session_id}/wait"),
            ServeDialect::Native2x => format!("api/experimental/session/{session_id}/wait"),
        };
        let response = self
            .client
            .inner
            .request_base(Method::POST, &path)?
            .send()
            .await?;
        self.client.inner.ensure_success(response).await
    }

    pub async fn interrupt(&self, session_id: &str) -> Result<(), Error> {
        let response = self
            .client
            .inner
            .request_base(Method::POST, &format!("api/session/{session_id}/interrupt"))?
            .send()
            .await?;
        self.client.inner.ensure_success(response).await
    }

    /// Subscribe to the V2 durable event stream for one session.
    ///
    /// Event data is intentionally kept as raw text because the V2 durable
    /// event union is still evolving upstream. OpenCode 2.x has no
    /// session-scoped event route, so this subscribes to the global
    /// `/api/event` stream there and callers must filter events by
    /// `sessionID` themselves; `after` only applies to the 1.x route.
    pub async fn events(
        &self,
        session_id: &str,
        after: Option<&str>,
    ) -> Result<EventStream, Error> {
        let dialect = self.client.serve_dialect().await?;
        let path = match dialect {
            ServeDialect::Preview1x => format!("api/session/{session_id}/event"),
            ServeDialect::Native2x => "api/event".to_owned(),
        };
        let mut url = self.client.inner.url(&path)?;
        if dialect == ServeDialect::Preview1x {
            if let Some(after) = after {
                url.query_pairs_mut().append_pair("after", after);
            }
        }
        let response = self
            .client
            .inner
            .request_url(Method::GET, url)
            .send()
            .await?;
        if !response.status().is_success() {
            self.client.inner.ensure_success(response).await?;
            unreachable!();
        }
        Ok(event_stream_from_response(response))
    }
}

pub struct EventsApi<'a> {
    client: &'a Client,
}

impl EventsApi<'_> {
    /// Subscribe to the native V2 server event stream at `/api/event`.
    pub async fn subscribe(&self) -> Result<EventStream, Error> {
        let response = self
            .client
            .inner
            .request_base(Method::GET, "api/event")?
            .send()
            .await?;
        if !response.status().is_success() {
            self.client.inner.ensure_success(response).await?;
            unreachable!();
        }
        Ok(event_stream_from_response(response))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub id: Option<String>,
    pub event: Option<String>,
    pub data: String,
}

pub struct EventStream {
    inner: BoxStream<'static, Result<Event, Error>>,
}

impl Stream for EventStream {
    type Item = Result<Event, Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.inner).poll_next(cx)
    }
}

type ByteStream = BoxStream<'static, Result<Bytes, reqwest::Error>>;

struct EventStreamState {
    bytes: ByteStream,
    decoder: SseDecoder,
    queued: VecDeque<Event>,
    finished: bool,
}

fn event_stream_from_response(response: reqwest::Response) -> EventStream {
    let state = EventStreamState {
        bytes: response.bytes_stream().boxed(),
        decoder: SseDecoder::new(),
        queued: VecDeque::new(),
        finished: false,
    };
    let inner = stream::unfold(state, |mut state| async move {
        loop {
            if let Some(event) = state.queued.pop_front() {
                return Some((Ok(event), state));
            }
            if state.finished {
                return None;
            }
            match state.bytes.next().await {
                Some(Ok(chunk)) => match state.decoder.push(&chunk) {
                    Ok(frames) => state.queued.extend(frames.into_iter().map(|frame| Event {
                        id: frame.id,
                        event: frame.event,
                        data: frame.data,
                    })),
                    Err(error) => return Some((Err(error), state)),
                },
                Some(Err(error)) => return Some((Err(Error::Http(error)), state)),
                None => {
                    state.finished = true;
                    match state.decoder.finish() {
                        Ok(frames) => state.queued.extend(frames.into_iter().map(|frame| Event {
                            id: frame.id,
                            event: frame.event,
                            data: frame.data,
                        })),
                        Err(error) => return Some((Err(error), state)),
                    }
                }
            }
        }
    })
    .boxed();
    EventStream { inner }
}
