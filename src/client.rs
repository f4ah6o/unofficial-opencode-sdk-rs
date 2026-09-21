use std::sync::Arc;

use reqwest::{Method, RequestBuilder, Response};
use serde::Serialize;
use serde::de::DeserializeOwned;
use url::Url;

use crate::error::{ApiError, Error};
use crate::models::{CreateSessionRequest, MessageWithParts, PromptRequest, Session};
use crate::sse::{EventStream, event_stream_from_response};

#[derive(Clone)]
pub struct Client {
    inner: Arc<Inner>,
}

struct Inner {
    base_url: Url,
    http: reqwest::Client,
    directory: Option<String>,
    username: Option<String>,
    password: Option<String>,
}

#[derive(Default)]
pub struct ClientBuilder {
    base_url: Option<String>,
    http: Option<reqwest::Client>,
    directory: Option<String>,
    username: Option<String>,
    password: Option<String>,
}

impl Client {
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    pub fn session(&self) -> SessionApi<'_> {
        SessionApi { client: self }
    }

    pub fn events(&self) -> EventsApi<'_> {
        EventsApi { client: self }
    }

    fn url(&self, path: &str) -> Result<Url, Error> {
        Ok(self.inner.base_url.join(path.trim_start_matches('/'))?)
    }

    fn request(&self, method: Method, path: &str) -> Result<RequestBuilder, Error> {
        let mut url = self.url(path)?;
        if let Some(directory) = &self.inner.directory {
            url.query_pairs_mut().append_pair("directory", directory);
        }
        let mut request = self.inner.http.request(method, url);
        if let Some(password) = &self.inner.password {
            let username = self.inner.username.as_deref().unwrap_or("opencode");
            request = request.basic_auth(username, Some(password));
        }
        Ok(request)
    }

    async fn decode<T: DeserializeOwned>(&self, response: Response) -> Result<T, Error> {
        if response.status().is_success() {
            return Ok(response.json().await?);
        }
        Err(api_error(response).await.into())
    }
}

impl ClientBuilder {
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = Some(base_url.into());
        self
    }

    pub fn http_client(mut self, client: reqwest::Client) -> Self {
        self.http = Some(client);
        self
    }

    /// Set the OpenCode workspace/directory routing context.
    pub fn directory(mut self, directory: impl Into<String>) -> Self {
        self.directory = Some(directory.into());
        self
    }

    /// Configure the Basic Auth contract used by OpenCode server when
    /// `OPENCODE_SERVER_PASSWORD` is set.
    pub fn basic_auth(mut self, username: impl Into<String>, password: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self.password = Some(password.into());
        self
    }

    /// Configure only the server password and use OpenCode's default username
    /// (`opencode`).
    pub fn password(mut self, password: impl Into<String>) -> Self {
        self.password = Some(password.into());
        self
    }

    pub fn build(self) -> Result<Client, Error> {
        let raw = self
            .base_url
            .unwrap_or_else(|| "http://127.0.0.1:4096".to_owned());
        let mut base_url = Url::parse(&raw)?;
        if !base_url.path().ends_with('/') {
            base_url.set_path(&format!("{}/", base_url.path()));
        }
        Ok(Client {
            inner: Arc::new(Inner {
                base_url,
                http: self.http.unwrap_or_default(),
                directory: self.directory,
                username: self.username,
                password: self.password,
            }),
        })
    }
}

pub struct SessionApi<'a> {
    client: &'a Client,
}

impl SessionApi<'_> {
    pub async fn list(&self) -> Result<Vec<Session>, Error> {
        let response = self.client.request(Method::GET, "session")?.send().await?;
        self.client.decode(response).await
    }

    pub async fn get(&self, session_id: &str) -> Result<Session, Error> {
        let response = self
            .client
            .request(Method::GET, &format!("session/{session_id}"))?
            .send()
            .await?;
        self.client.decode(response).await
    }

    pub async fn create(&self, body: &CreateSessionRequest) -> Result<Session, Error> {
        let response = self
            .client
            .request(Method::POST, "session")?
            .json(body)
            .send()
            .await?;
        self.client.decode(response).await
    }

    pub async fn prompt(
        &self,
        session_id: &str,
        body: &PromptRequest,
    ) -> Result<MessageWithParts, Error> {
        let response = self
            .client
            .request(Method::POST, &format!("session/{session_id}/message"))?
            .json(body)
            .send()
            .await?;
        self.client.decode(response).await
    }

    pub async fn abort(&self, session_id: &str) -> Result<bool, Error> {
        let response = self
            .client
            .request(Method::POST, &format!("session/{session_id}/abort"))?
            .send()
            .await?;
        self.client.decode(response).await
    }

    pub async fn raw_json<T: Serialize + ?Sized, R: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<&T>,
    ) -> Result<R, Error> {
        let mut request = self.client.request(method, path)?;
        if let Some(body) = body {
            request = request.json(body);
        }
        let response = request.send().await?;
        self.client.decode(response).await
    }
}

pub struct EventsApi<'a> {
    client: &'a Client,
}

impl EventsApi<'_> {
    pub async fn subscribe(&self) -> Result<EventStream, Error> {
        let response = self.client.request(Method::GET, "event")?.send().await?;
        if !response.status().is_success() {
            return Err(api_error(response).await.into());
        }
        Ok(event_stream_from_response(response))
    }
}

async fn api_error(response: Response) -> ApiError {
    let status = response.status().as_u16();
    let body = response.text().await.unwrap_or_default();
    let json = serde_json::from_str(&body).ok();
    ApiError { status, body, json }
}
