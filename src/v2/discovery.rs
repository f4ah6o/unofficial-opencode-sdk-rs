use std::collections::BTreeMap;

use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{Client, Located, LocationInfo, LocationQuery, ModelRef, ServeDialect};
use crate::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Health {
    pub healthy: bool,
    /// OpenCode 2.x `api/info` fields (`version`, `pid`, `urls`, `paths`).
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AgentInfo {
    pub id: String,
    #[serde(default)]
    pub model: Option<ModelRef>,
    pub request: Value,
    #[serde(default)]
    pub system: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    pub mode: String,
    pub hidden: bool,
    #[serde(default)]
    pub color: Option<Value>,
    #[serde(default)]
    pub steps: Option<u64>,
    pub permissions: Value,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CommandInfo {
    pub name: String,
    /// OpenCode 2.x only ships `description`; `template` is 1.x-preview data.
    #[serde(default)]
    pub template: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub model: Option<ModelRef>,
    #[serde(default)]
    pub subtask: Option<bool>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SkillInfo {
    pub name: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub slash: Option<bool>,
    /// 1.x-preview field; 2.x exposes the file as `path` instead.
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub content: String,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReferenceInfo {
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub hidden: Option<bool>,
    pub source: Value,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IntegrationInfo {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub methods: Vec<Value>,
    #[serde(default)]
    pub connections: Vec<Value>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationAttempt {
    #[serde(rename = "attemptID")]
    pub attempt_id: String,
    pub url: String,
    pub instructions: String,
    pub mode: String,
    pub time: Value,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IntegrationAttemptStatus {
    pub status: String,
    #[serde(default)]
    pub message: Option<String>,
    pub time: Value,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntegrationKeyRequest {
    pub key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IntegrationOauthRequest {
    #[serde(rename = "methodID")]
    pub method_id: String,
    #[serde(default)]
    pub inputs: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntegrationAttemptCompleteRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CredentialUpdateRequest {
    pub label: String,
}

pub struct HealthApi<'a> {
    client: &'a Client,
}

impl HealthApi<'_> {
    /// Liveness check: `api/health` on OpenCode 1.x, `api/info` on 2.x.
    pub async fn get(&self) -> Result<Health, Error> {
        match self.client.serve_dialect().await? {
            ServeDialect::Preview1x => {
                let response = self
                    .client
                    .inner
                    .request_base(Method::GET, "api/health")?
                    .send()
                    .await?;
                self.client.inner.decode(response).await
            }
            ServeDialect::Native2x => {
                let response = self
                    .client
                    .inner
                    .request_base(Method::GET, "api/info")?
                    .send()
                    .await?;
                let info: Value = self.client.inner.decode(response).await?;
                let mut extra = match info {
                    Value::Object(map) => map,
                    other => {
                        let mut map = serde_json::Map::new();
                        map.insert("info".to_owned(), other);
                        map
                    }
                };
                extra.remove("healthy");
                Ok(Health {
                    healthy: true,
                    extra: extra.into_iter().collect(),
                })
            }
        }
    }
}

pub struct LocationApi<'a> {
    client: &'a Client,
}

impl LocationApi<'_> {
    pub async fn get(&self, location: Option<&LocationQuery>) -> Result<LocationInfo, Error> {
        let mut url = self.client.inner.url("api/location")?;
        super::resources::apply_location(&mut url, self.client, location);
        let response = self
            .client
            .inner
            .request_url(Method::GET, url)
            .send()
            .await?;
        self.client.inner.decode(response).await
    }
}

pub struct AgentApi<'a> {
    client: &'a Client,
}

impl AgentApi<'_> {
    pub async fn list(
        &self,
        location: Option<&LocationQuery>,
    ) -> Result<Located<Vec<AgentInfo>>, Error> {
        let mut url = self.client.inner.url("api/agent")?;
        super::resources::apply_location(&mut url, self.client, location);
        let response = self
            .client
            .inner
            .request_url(Method::GET, url)
            .send()
            .await?;
        self.client.inner.decode(response).await
    }
}

pub struct CommandApi<'a> {
    client: &'a Client,
}

impl CommandApi<'_> {
    pub async fn list(
        &self,
        location: Option<&LocationQuery>,
    ) -> Result<Located<Vec<CommandInfo>>, Error> {
        let mut url = self.client.inner.url("api/command")?;
        super::resources::apply_location(&mut url, self.client, location);
        let response = self
            .client
            .inner
            .request_url(Method::GET, url)
            .send()
            .await?;
        self.client.inner.decode(response).await
    }
}

pub struct SkillApi<'a> {
    client: &'a Client,
}

impl SkillApi<'_> {
    pub async fn list(
        &self,
        location: Option<&LocationQuery>,
    ) -> Result<Located<Vec<SkillInfo>>, Error> {
        let mut url = self.client.inner.url("api/skill")?;
        super::resources::apply_location(&mut url, self.client, location);
        let response = self
            .client
            .inner
            .request_url(Method::GET, url)
            .send()
            .await?;
        self.client.inner.decode(response).await
    }
}

pub struct ReferenceApi<'a> {
    client: &'a Client,
}

impl ReferenceApi<'_> {
    pub async fn list(
        &self,
        location: Option<&LocationQuery>,
    ) -> Result<Located<Vec<ReferenceInfo>>, Error> {
        let mut url = self.client.inner.url("api/reference")?;
        super::resources::apply_location(&mut url, self.client, location);
        let response = self
            .client
            .inner
            .request_url(Method::GET, url)
            .send()
            .await?;
        self.client.inner.decode(response).await
    }
}

pub struct IntegrationApi<'a> {
    client: &'a Client,
}

impl IntegrationApi<'_> {
    pub async fn list(
        &self,
        location: Option<&LocationQuery>,
    ) -> Result<Located<Vec<IntegrationInfo>>, Error> {
        let mut url = self.client.inner.url("api/integration")?;
        super::resources::apply_location(&mut url, self.client, location);
        let response = self
            .client
            .inner
            .request_url(Method::GET, url)
            .send()
            .await?;
        self.client.inner.decode(response).await
    }

    pub async fn get(
        &self,
        integration_id: &str,
        location: Option<&LocationQuery>,
    ) -> Result<Located<IntegrationInfo>, Error> {
        let mut url = self
            .client
            .inner
            .url(&format!("api/integration/{integration_id}"))?;
        super::resources::apply_location(&mut url, self.client, location);
        let response = self
            .client
            .inner
            .request_url(Method::GET, url)
            .send()
            .await?;
        self.client.inner.decode(response).await
    }

    pub fn connect(&self) -> IntegrationConnectApi<'_> {
        IntegrationConnectApi {
            client: self.client,
        }
    }

    pub fn attempt(&self) -> IntegrationAttemptApi<'_> {
        IntegrationAttemptApi {
            client: self.client,
        }
    }
}

pub struct IntegrationConnectApi<'a> {
    client: &'a Client,
}

impl IntegrationConnectApi<'_> {
    pub async fn key(
        &self,
        integration_id: &str,
        body: &IntegrationKeyRequest,
        location: Option<&LocationQuery>,
    ) -> Result<(), Error> {
        let mut url = self
            .client
            .inner
            .url(&format!("api/integration/{integration_id}/connect/key"))?;
        super::resources::apply_location(&mut url, self.client, location);
        let response = self
            .client
            .inner
            .request_url(Method::POST, url)
            .json(body)
            .send()
            .await?;
        self.client.inner.ensure_success(response).await
    }

    pub async fn oauth(
        &self,
        integration_id: &str,
        body: &IntegrationOauthRequest,
        location: Option<&LocationQuery>,
    ) -> Result<Located<IntegrationAttempt>, Error> {
        let mut url = self
            .client
            .inner
            .url(&format!("api/integration/{integration_id}/connect/oauth"))?;
        super::resources::apply_location(&mut url, self.client, location);
        let response = self
            .client
            .inner
            .request_url(Method::POST, url)
            .json(body)
            .send()
            .await?;
        self.client.inner.decode(response).await
    }
}

pub struct IntegrationAttemptApi<'a> {
    client: &'a Client,
}

impl IntegrationAttemptApi<'_> {
    /// Poll an OAuth/command connect attempt.
    ///
    /// OpenCode 2.x moved attempt polling under
    /// `api/integration/{id}/connect/{command|oauth}/{attemptID}`; this
    /// method only covers the 1.x preview route and returns
    /// [`Error::Unsupported`] on OpenCode 2.x.
    pub async fn status(
        &self,
        attempt_id: &str,
        location: Option<&LocationQuery>,
    ) -> Result<Located<IntegrationAttemptStatus>, Error> {
        if self.client.serve_dialect().await? == ServeDialect::Native2x {
            return Err(Error::Unsupported(
                "api/integration/attempt is 1.x only; use api/integration/{id}/connect/{command|oauth}/{attemptID} on 2.x",
            ));
        }
        let mut url = self
            .client
            .inner
            .url(&format!("api/integration/attempt/{attempt_id}"))?;
        super::resources::apply_location(&mut url, self.client, location);
        let response = self
            .client
            .inner
            .request_url(Method::GET, url)
            .send()
            .await?;
        self.client.inner.decode(response).await
    }

    pub async fn cancel(
        &self,
        attempt_id: &str,
        location: Option<&LocationQuery>,
    ) -> Result<(), Error> {
        if self.client.serve_dialect().await? == ServeDialect::Native2x {
            return Err(Error::Unsupported(
                "api/integration/attempt is 1.x only; use api/integration/{id}/connect/{command|oauth}/{attemptID} on 2.x",
            ));
        }
        let mut url = self
            .client
            .inner
            .url(&format!("api/integration/attempt/{attempt_id}"))?;
        super::resources::apply_location(&mut url, self.client, location);
        let response = self
            .client
            .inner
            .request_url(Method::DELETE, url)
            .send()
            .await?;
        self.client.inner.ensure_success(response).await
    }

    pub async fn complete(
        &self,
        attempt_id: &str,
        body: &IntegrationAttemptCompleteRequest,
        location: Option<&LocationQuery>,
    ) -> Result<(), Error> {
        if self.client.serve_dialect().await? == ServeDialect::Native2x {
            return Err(Error::Unsupported(
                "api/integration/attempt is 1.x only; use api/integration/{id}/connect/{command|oauth}/{attemptID} on 2.x",
            ));
        }
        let mut url = self
            .client
            .inner
            .url(&format!("api/integration/attempt/{attempt_id}/complete"))?;
        super::resources::apply_location(&mut url, self.client, location);
        let response = self
            .client
            .inner
            .request_url(Method::POST, url)
            .json(body)
            .send()
            .await?;
        self.client.inner.ensure_success(response).await
    }
}

pub struct CredentialApi<'a> {
    client: &'a Client,
}

impl CredentialApi<'_> {
    /// Activate a stored credential (OpenCode 2.x only).
    pub async fn activate(
        &self,
        credential_id: &str,
        location: Option<&LocationQuery>,
    ) -> Result<(), Error> {
        if self.client.serve_dialect().await? != ServeDialect::Native2x {
            return Err(Error::Unsupported(
                "api/credential/{id}/activate requires OpenCode 2.x",
            ));
        }
        let mut url = self
            .client
            .inner
            .url(&format!("api/credential/{credential_id}/activate"))?;
        super::resources::apply_location(&mut url, self.client, location);
        let response = self
            .client
            .inner
            .request_url(Method::POST, url)
            .send()
            .await?;
        self.client.inner.ensure_success(response).await
    }

    pub async fn update(
        &self,
        credential_id: &str,
        body: &CredentialUpdateRequest,
        location: Option<&LocationQuery>,
    ) -> Result<(), Error> {
        let mut url = self
            .client
            .inner
            .url(&format!("api/credential/{credential_id}"))?;
        super::resources::apply_location(&mut url, self.client, location);
        let response = self
            .client
            .inner
            .request_url(Method::PATCH, url)
            .json(body)
            .send()
            .await?;
        self.client.inner.ensure_success(response).await
    }

    pub async fn remove(
        &self,
        credential_id: &str,
        location: Option<&LocationQuery>,
    ) -> Result<(), Error> {
        let mut url = self
            .client
            .inner
            .url(&format!("api/credential/{credential_id}"))?;
        super::resources::apply_location(&mut url, self.client, location);
        let response = self
            .client
            .inner
            .request_url(Method::DELETE, url)
            .send()
            .await?;
        self.client.inner.ensure_success(response).await
    }
}

impl Client {
    pub fn health(&self) -> HealthApi<'_> {
        HealthApi { client: self }
    }

    pub fn location(&self) -> LocationApi<'_> {
        LocationApi { client: self }
    }

    pub fn agent(&self) -> AgentApi<'_> {
        AgentApi { client: self }
    }

    pub fn command(&self) -> CommandApi<'_> {
        CommandApi { client: self }
    }

    pub fn skill(&self) -> SkillApi<'_> {
        SkillApi { client: self }
    }

    pub fn reference(&self) -> ReferenceApi<'_> {
        ReferenceApi { client: self }
    }

    pub fn integration(&self) -> IntegrationApi<'_> {
        IntegrationApi { client: self }
    }

    pub fn credential(&self) -> CredentialApi<'_> {
        CredentialApi { client: self }
    }
}
