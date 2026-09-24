use std::collections::BTreeMap;

use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{ModelRef, Order, PermissionRequest, QuestionRequest, ServeDialect, SessionApi};
use crate::Error;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SessionActiveState {
    Running,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionActive {
    #[serde(rename = "type")]
    pub state: SessionActiveState,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionHistory {
    #[serde(default)]
    pub data: Vec<Value>,
    pub has_more: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionMessagesPage {
    #[serde(default)]
    pub data: Vec<Value>,
    pub cursor: super::Cursor,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SessionMessagesOptions {
    pub limit: Option<u32>,
    pub order: Option<Order>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum FileDiffStatus {
    Added,
    Modified,
    Deleted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileDiff {
    pub path: String,
    pub status: FileDiffStatus,
    pub additions: u64,
    pub deletions: u64,
    pub patch: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RevertState {
    #[serde(rename = "messageID")]
    pub message_id: String,
    #[serde(default, rename = "partID")]
    pub part_id: Option<String>,
    #[serde(default)]
    pub snapshot: Option<String>,
    #[serde(default)]
    pub diff: Option<String>,
    #[serde(default)]
    pub files: Vec<FileDiff>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RevertStageRequest {
    #[serde(rename = "messageID")]
    pub message_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub files: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PermissionReply {
    Once,
    Always,
    Reject,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PermissionEffect {
    Allow,
    Deny,
    Ask,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PermissionCreated {
    pub id: String,
    pub effect: PermissionEffect,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PermissionSource {
    #[serde(rename = "type")]
    pub source_type: String,
    #[serde(rename = "messageID")]
    pub message_id: String,
    #[serde(rename = "callID")]
    pub call_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "camelCase")]
pub struct CreatePermissionRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    pub action: String,
    #[serde(default)]
    pub resources: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub save: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<PermissionSource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReplyPermissionRequest {
    pub reply: PermissionReply,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QuestionReplyRequest {
    pub answers: Vec<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct Envelope<T> {
    data: T,
}

pub struct SessionRevertApi<'a> {
    client: &'a super::Client,
    session_id: &'a str,
}

impl SessionRevertApi<'_> {
    pub async fn stage(&self, body: &RevertStageRequest) -> Result<RevertState, Error> {
        let response = self
            .client
            .inner
            .request_base(
                Method::POST,
                &format!("api/session/{}/revert/stage", self.session_id),
            )?
            .json(body)
            .send()
            .await?;
        let envelope: Envelope<RevertState> = self.client.inner.decode(response).await?;
        Ok(envelope.data)
    }

    /// Clear staged revert state: `POST .../revert/clear` on 1.x,
    /// `DELETE .../revert` on 2.x.
    pub async fn clear(&self) -> Result<(), Error> {
        let response = match self.client.serve_dialect().await? {
            ServeDialect::Preview1x => {
                self.client
                    .inner
                    .request_base(
                        Method::POST,
                        &format!("api/session/{}/revert/clear", self.session_id),
                    )?
                    .send()
                    .await?
            }
            ServeDialect::Native2x => {
                self.client
                    .inner
                    .request_base(
                        Method::DELETE,
                        &format!("api/session/{}/revert", self.session_id),
                    )?
                    .send()
                    .await?
            }
        };
        self.client.inner.ensure_success(response).await
    }

    pub async fn commit(&self) -> Result<(), Error> {
        let response = self
            .client
            .inner
            .request_base(
                Method::POST,
                &format!("api/session/{}/revert/commit", self.session_id),
            )?
            .send()
            .await?;
        self.client.inner.ensure_success(response).await
    }
}

pub struct SessionPermissionApi<'a> {
    client: &'a super::Client,
    session_id: &'a str,
}

impl SessionPermissionApi<'_> {
    pub async fn list(&self) -> Result<Vec<PermissionRequest>, Error> {
        let response = self
            .client
            .inner
            .request_base(
                Method::GET,
                &format!("api/session/{}/permission", self.session_id),
            )?
            .send()
            .await?;
        let envelope: Envelope<Vec<PermissionRequest>> = self.client.inner.decode(response).await?;
        Ok(envelope.data)
    }

    pub async fn create(&self, body: &CreatePermissionRequest) -> Result<PermissionCreated, Error> {
        let response = self
            .client
            .inner
            .request_base(
                Method::POST,
                &format!("api/session/{}/permission", self.session_id),
            )?
            .json(body)
            .send()
            .await?;
        let envelope: Envelope<PermissionCreated> = self.client.inner.decode(response).await?;
        Ok(envelope.data)
    }

    pub async fn get(&self, request_id: &str) -> Result<PermissionRequest, Error> {
        let response = self
            .client
            .inner
            .request_base(
                Method::GET,
                &format!("api/session/{}/permission/{request_id}", self.session_id),
            )?
            .send()
            .await?;
        let envelope: Envelope<PermissionRequest> = self.client.inner.decode(response).await?;
        Ok(envelope.data)
    }

    pub async fn reply(
        &self,
        request_id: &str,
        body: &ReplyPermissionRequest,
    ) -> Result<(), Error> {
        let response = self
            .client
            .inner
            .request_base(
                Method::POST,
                &format!(
                    "api/session/{}/permission/{request_id}/reply",
                    self.session_id
                ),
            )?
            .json(body)
            .send()
            .await?;
        self.client.inner.ensure_success(response).await
    }
}

pub struct SessionQuestionApi<'a> {
    client: &'a super::Client,
    session_id: &'a str,
}

impl SessionQuestionApi<'_> {
    /// Pending questions (1.x `api/session/{id}/question`) or forms
    /// (2.x `api/session/{id}/form`); both decode into [`QuestionRequest`].
    pub async fn list(&self) -> Result<Vec<QuestionRequest>, Error> {
        let path = match self.client.serve_dialect().await? {
            ServeDialect::Preview1x => format!("api/session/{}/question", self.session_id),
            ServeDialect::Native2x => format!("api/session/{}/form", self.session_id),
        };
        let response = self
            .client
            .inner
            .request_base(Method::GET, &path)?
            .send()
            .await?;
        let envelope: Envelope<Vec<QuestionRequest>> = self.client.inner.decode(response).await?;
        Ok(envelope.data)
    }

    /// Answer with positional `answers` (OpenCode 1.x question contract).
    ///
    /// OpenCode 2.x forms are keyed by field key instead of position — use
    /// [`SessionQuestionApi::reply_answer`] there. Returns
    /// [`Error::Unsupported`] on OpenCode 2.x.
    pub async fn reply(&self, request_id: &str, body: &QuestionReplyRequest) -> Result<(), Error> {
        if self.client.serve_dialect().await? == ServeDialect::Native2x {
            return Err(Error::Unsupported(
                "positional question replies are 1.x only; use reply_answer() for 2.x forms",
            ));
        }
        let response = self
            .client
            .inner
            .request_base(
                Method::POST,
                &format!(
                    "api/session/{}/question/{request_id}/reply",
                    self.session_id
                ),
            )?
            .json(body)
            .send()
            .await?;
        self.client.inner.ensure_success(response).await
    }

    /// Submit a keyed form answer (OpenCode 2.x `api/session/{id}/form/{formID}/reply`).
    ///
    /// `answer` keys must match the form's field keys. Returns
    /// [`Error::Unsupported`] on OpenCode 1.x.
    pub async fn reply_answer(
        &self,
        form_id: &str,
        answer: &BTreeMap<String, Value>,
    ) -> Result<(), Error> {
        if self.client.serve_dialect().await? != ServeDialect::Native2x {
            return Err(Error::Unsupported(
                "keyed form replies require OpenCode 2.x; use reply() for 1.x questions",
            ));
        }
        let response = self
            .client
            .inner
            .request_base(
                Method::POST,
                &format!("api/session/{}/form/{form_id}/reply", self.session_id),
            )?
            .json(&serde_json::json!({ "answer": answer }))
            .send()
            .await?;
        self.client.inner.ensure_success(response).await
    }

    /// Reject/cancel a pending question (1.x) or form (2.x `DELETE .../form/{id}`).
    pub async fn reject(&self, request_id: &str) -> Result<(), Error> {
        let response = match self.client.serve_dialect().await? {
            ServeDialect::Preview1x => {
                self.client
                    .inner
                    .request_base(
                        Method::POST,
                        &format!(
                            "api/session/{}/question/{request_id}/reject",
                            self.session_id
                        ),
                    )?
                    .send()
                    .await?
            }
            ServeDialect::Native2x => {
                self.client
                    .inner
                    .request_base(
                        Method::DELETE,
                        &format!("api/session/{}/form/{request_id}", self.session_id),
                    )?
                    .send()
                    .await?
            }
        };
        self.client.inner.ensure_success(response).await
    }
}

impl SessionApi<'_> {
    pub async fn active(&self) -> Result<BTreeMap<String, SessionActive>, Error> {
        let response = self
            .client
            .inner
            .request_base(Method::GET, "api/session/active")?
            .send()
            .await?;
        let envelope: Envelope<BTreeMap<String, SessionActive>> =
            self.client.inner.decode(response).await?;
        Ok(envelope.data)
    }

    pub async fn switch_agent(&self, session_id: &str, agent: &str) -> Result<(), Error> {
        #[derive(Serialize)]
        struct Body<'a> {
            agent: &'a str,
        }

        let response = self
            .client
            .inner
            .request_base(Method::POST, &format!("api/session/{session_id}/agent"))?
            .json(&Body { agent })
            .send()
            .await?;
        self.client.inner.ensure_success(response).await
    }

    pub async fn switch_model(&self, session_id: &str, model: &ModelRef) -> Result<(), Error> {
        #[derive(Serialize)]
        struct Body<'a> {
            model: &'a ModelRef,
        }

        let response = self
            .client
            .inner
            .request_base(Method::POST, &format!("api/session/{session_id}/model"))?
            .json(&Body { model })
            .send()
            .await?;
        self.client.inner.ensure_success(response).await
    }

    pub async fn compact(&self, session_id: &str) -> Result<(), Error> {
        let response = self
            .client
            .inner
            .request_base(Method::POST, &format!("api/session/{session_id}/compact"))?
            .send()
            .await?;
        self.client.inner.ensure_success(response).await
    }

    pub async fn context(&self, session_id: &str) -> Result<Vec<Value>, Error> {
        let response = self
            .client
            .inner
            .request_base(Method::GET, &format!("api/session/{session_id}/context"))?
            .send()
            .await?;
        let envelope: Envelope<Vec<Value>> = self.client.inner.decode(response).await?;
        Ok(envelope.data)
    }

    /// Session history. OpenCode 2.x has no history route — returns
    /// [`Error::Unsupported`] there; use `messages()` for 2.x message history.
    pub async fn history(
        &self,
        session_id: &str,
        limit: Option<u64>,
        after: Option<u64>,
    ) -> Result<SessionHistory, Error> {
        if self.client.serve_dialect().await? == ServeDialect::Native2x {
            return Err(Error::Unsupported(
                "api/session/{id}/history is 1.x only; use messages() on 2.x",
            ));
        }
        let mut url = self
            .client
            .inner
            .url(&format!("api/session/{session_id}/history"))?;
        {
            let mut query = url.query_pairs_mut();
            if let Some(limit) = limit {
                query.append_pair("limit", &limit.to_string());
            }
            if let Some(after) = after {
                query.append_pair("after", &after.to_string());
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

    pub async fn message(&self, session_id: &str, message_id: &str) -> Result<Value, Error> {
        let response = self
            .client
            .inner
            .request_base(
                Method::GET,
                &format!("api/session/{session_id}/message/{message_id}"),
            )?
            .send()
            .await?;
        let envelope: Envelope<Value> = self.client.inner.decode(response).await?;
        Ok(envelope.data)
    }

    pub async fn messages(
        &self,
        session_id: &str,
        options: &SessionMessagesOptions,
    ) -> Result<SessionMessagesPage, Error> {
        let mut url = self
            .client
            .inner
            .url(&format!("api/session/{session_id}/message"))?;
        {
            let mut query = url.query_pairs_mut();
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

    pub fn revert<'a>(&'a self, session_id: &'a str) -> SessionRevertApi<'a> {
        SessionRevertApi {
            client: self.client,
            session_id,
        }
    }

    pub fn permission<'a>(&'a self, session_id: &'a str) -> SessionPermissionApi<'a> {
        SessionPermissionApi {
            client: self.client,
            session_id,
        }
    }

    pub fn question<'a>(&'a self, session_id: &'a str) -> SessionQuestionApi<'a> {
        SessionQuestionApi {
            client: self.client,
            session_id,
        }
    }
}
