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

/// V2 client for OpenCode's `/api/*` HTTP surface.
#[derive(Clone)]
pub struct Client {
    inner: crate::Client,
    directory: Option<String>,
    workspace_id: Option<String>,
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
        Self {
            inner: client,
            directory,
            workspace_id: None,
        }
    }

    pub fn session(&self) -> SessionApi<'_> {
        SessionApi { client: self }
    }

    pub fn events(&self) -> EventsApi<'_> {
        EventsApi { client: self }
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
    pub title: String,
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
    pub admitted_seq: u64,
    pub id: String,
    #[serde(rename = "sessionID")]
    pub session_id: String,
    pub prompt: Value,
    pub delivery: Delivery,
    pub time_created: u64,
    #[serde(default)]
    pub promoted_seq: Option<u64>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
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
        let response = self
            .client
            .inner
            .request_base(Method::POST, &format!("api/session/{session_id}/prompt"))?
            .json(body)
            .send()
            .await?;
        let envelope: DataEnvelope<SessionInputAdmitted> =
            self.client.inner.decode(response).await?;
        Ok(envelope.data)
    }

    pub async fn wait(&self, session_id: &str) -> Result<(), Error> {
        let response = self
            .client
            .inner
            .request_base(Method::POST, &format!("api/session/{session_id}/wait"))?
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
    /// event union is still evolving upstream.
    pub async fn events(
        &self,
        session_id: &str,
        after: Option<&str>,
    ) -> Result<EventStream, Error> {
        let mut url = self
            .client
            .inner
            .url(&format!("api/session/{session_id}/event"))?;
        if let Some(after) = after {
            url.query_pairs_mut().append_pair("after", after);
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
