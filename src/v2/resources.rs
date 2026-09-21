use std::collections::BTreeMap;

use bytes::Bytes;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::Client;
use crate::Error;

/// Per-request V2 location override.
///
/// Query endpoints encode this using OpenCode's deep-object parameters:
/// `location[directory]` and `location[workspace]`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocationQuery {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub directory: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectLocationInfo {
    pub id: String,
    pub directory: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LocationInfo {
    pub directory: String,
    #[serde(default, rename = "workspaceID")]
    pub workspace_id: Option<String>,
    pub project: ProjectLocationInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Located<T> {
    pub location: LocationInfo,
    pub data: T,
}

/// Stable top-level fields from OpenCode's preview `ModelV2Info`.
///
/// The deeply nested provider-specific model contract is retained losslessly as
/// JSON while V2 remains preview.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub id: String,
    #[serde(rename = "providerID")]
    pub provider_id: String,
    #[serde(default)]
    pub family: Option<String>,
    pub name: String,
    pub api: Value,
    pub capabilities: Value,
    pub request: Value,
    #[serde(default)]
    pub variants: Vec<Value>,
    pub time: Value,
    #[serde(default)]
    pub cost: Vec<Value>,
    pub status: String,
    pub enabled: bool,
    pub limit: Value,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInfo {
    pub id: String,
    #[serde(default, rename = "integrationID")]
    pub integration_id: Option<String>,
    pub name: String,
    #[serde(default)]
    pub disabled: Option<bool>,
    pub api: Value,
    pub request: Value,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FileSystemEntryType {
    File,
    Directory,
}

impl FileSystemEntryType {
    fn as_query(self) -> &'static str {
        match self {
            Self::File => "file",
            Self::Directory => "directory",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileSystemEntry {
    pub path: String,
    #[serde(rename = "type")]
    pub entry_type: FileSystemEntryType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PermissionRequest {
    pub id: String,
    #[serde(rename = "sessionID")]
    pub session_id: String,
    pub action: String,
    pub resources: Vec<String>,
    #[serde(default)]
    pub save: Vec<String>,
    #[serde(default)]
    pub metadata: Option<Value>,
    #[serde(default)]
    pub source: Option<Value>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SavedPermission {
    pub id: String,
    #[serde(rename = "projectID")]
    pub project_id: String,
    pub action: String,
    pub resource: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct QuestionRequest {
    pub id: String,
    #[serde(rename = "sessionID")]
    pub session_id: String,
    #[serde(default)]
    pub questions: Vec<Value>,
    #[serde(default)]
    pub tool: Option<Value>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FindFilesOptions {
    pub query: String,
    pub entry_type: Option<FileSystemEntryType>,
    /// Upstream currently exposes this query parameter as a string.
    pub limit: Option<String>,
    pub location: Option<LocationQuery>,
}

#[derive(Debug, Deserialize)]
struct DataEnvelope<T> {
    data: T,
}

fn apply_location(url: &mut url::Url, client: &Client, location: Option<&LocationQuery>) {
    let directory = location
        .and_then(|value| value.directory.as_deref())
        .or(client.directory.as_deref());
    let workspace = location
        .and_then(|value| value.workspace.as_deref())
        .or(client.workspace_id.as_deref());

    let mut query = url.query_pairs_mut();
    if let Some(directory) = directory {
        query.append_pair("location[directory]", directory);
    }
    if let Some(workspace) = workspace {
        query.append_pair("location[workspace]", workspace);
    }
}

pub struct ModelApi<'a> {
    client: &'a Client,
}

impl ModelApi<'_> {
    pub async fn list(
        &self,
        location: Option<&LocationQuery>,
    ) -> Result<Located<Vec<ModelInfo>>, Error> {
        let mut url = self.client.inner.url("api/model")?;
        apply_location(&mut url, self.client, location);
        let response = self
            .client
            .inner
            .request_url(Method::GET, url)
            .send()
            .await?;
        self.client.inner.decode(response).await
    }
}

pub struct ProviderApi<'a> {
    client: &'a Client,
}

impl ProviderApi<'_> {
    pub async fn list(
        &self,
        location: Option<&LocationQuery>,
    ) -> Result<Located<Vec<ProviderInfo>>, Error> {
        let mut url = self.client.inner.url("api/provider")?;
        apply_location(&mut url, self.client, location);
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
        provider_id: &str,
        location: Option<&LocationQuery>,
    ) -> Result<Located<ProviderInfo>, Error> {
        let mut url = self
            .client
            .inner
            .url(&format!("api/provider/{provider_id}"))?;
        apply_location(&mut url, self.client, location);
        let response = self
            .client
            .inner
            .request_url(Method::GET, url)
            .send()
            .await?;
        self.client.inner.decode(response).await
    }
}

pub struct FsApi<'a> {
    client: &'a Client,
}

impl FsApi<'_> {
    /// Read a file relative to the requested V2 location.
    pub async fn read(&self, path: &str, location: Option<&LocationQuery>) -> Result<Bytes, Error> {
        let mut url = self
            .client
            .inner
            .url(&format!("api/fs/read/{}", path.trim_start_matches('/')))?;
        apply_location(&mut url, self.client, location);
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
        Ok(response.bytes().await?)
    }

    pub async fn list(
        &self,
        path: Option<&str>,
        location: Option<&LocationQuery>,
    ) -> Result<Located<Vec<FileSystemEntry>>, Error> {
        let mut url = self.client.inner.url("api/fs/list")?;
        apply_location(&mut url, self.client, location);
        if let Some(path) = path {
            url.query_pairs_mut().append_pair("path", path);
        }
        let response = self
            .client
            .inner
            .request_url(Method::GET, url)
            .send()
            .await?;
        self.client.inner.decode(response).await
    }

    pub async fn find(
        &self,
        options: &FindFilesOptions,
    ) -> Result<Located<Vec<FileSystemEntry>>, Error> {
        let mut url = self.client.inner.url("api/fs/find")?;
        apply_location(&mut url, self.client, options.location.as_ref());
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("query", &options.query);
            if let Some(entry_type) = options.entry_type {
                query.append_pair("type", entry_type.as_query());
            }
            if let Some(limit) = options.limit.as_deref() {
                query.append_pair("limit", limit);
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
}

pub struct PermissionApi<'a> {
    client: &'a Client,
}

impl PermissionApi<'_> {
    pub fn request(&self) -> PermissionRequestApi<'_> {
        PermissionRequestApi {
            client: self.client,
        }
    }

    pub fn saved(&self) -> SavedPermissionApi<'_> {
        SavedPermissionApi {
            client: self.client,
        }
    }
}

pub struct PermissionRequestApi<'a> {
    client: &'a Client,
}

impl PermissionRequestApi<'_> {
    pub async fn list(
        &self,
        location: Option<&LocationQuery>,
    ) -> Result<Located<Vec<PermissionRequest>>, Error> {
        let mut url = self.client.inner.url("api/permission/request")?;
        apply_location(&mut url, self.client, location);
        let response = self
            .client
            .inner
            .request_url(Method::GET, url)
            .send()
            .await?;
        self.client.inner.decode(response).await
    }
}

pub struct SavedPermissionApi<'a> {
    client: &'a Client,
}

impl SavedPermissionApi<'_> {
    pub async fn list(&self, project_id: Option<&str>) -> Result<Vec<SavedPermission>, Error> {
        let mut url = self.client.inner.url("api/permission/saved")?;
        if let Some(project_id) = project_id {
            url.query_pairs_mut().append_pair("projectID", project_id);
        }
        let response = self
            .client
            .inner
            .request_url(Method::GET, url)
            .send()
            .await?;
        let envelope: DataEnvelope<Vec<SavedPermission>> =
            self.client.inner.decode(response).await?;
        Ok(envelope.data)
    }

    pub async fn remove(&self, id: &str) -> Result<(), Error> {
        let response = self
            .client
            .inner
            .request_base(Method::DELETE, &format!("api/permission/saved/{id}"))?
            .send()
            .await?;
        self.client.inner.ensure_success(response).await
    }
}

pub struct QuestionApi<'a> {
    client: &'a Client,
}

impl QuestionApi<'_> {
    pub fn request(&self) -> QuestionRequestApi<'_> {
        QuestionRequestApi {
            client: self.client,
        }
    }
}

pub struct QuestionRequestApi<'a> {
    client: &'a Client,
}

impl QuestionRequestApi<'_> {
    pub async fn list(
        &self,
        location: Option<&LocationQuery>,
    ) -> Result<Located<Vec<QuestionRequest>>, Error> {
        let mut url = self.client.inner.url("api/question/request")?;
        apply_location(&mut url, self.client, location);
        let response = self
            .client
            .inner
            .request_url(Method::GET, url)
            .send()
            .await?;
        self.client.inner.decode(response).await
    }
}

impl Client {
    pub fn model(&self) -> ModelApi<'_> {
        ModelApi { client: self }
    }

    pub fn provider(&self) -> ProviderApi<'_> {
        ProviderApi { client: self }
    }

    pub fn fs(&self) -> FsApi<'_> {
        FsApi { client: self }
    }

    pub fn permission(&self) -> PermissionApi<'_> {
        PermissionApi { client: self }
    }

    pub fn question(&self) -> QuestionApi<'_> {
        QuestionApi { client: self }
    }
}
