use unofficial_opencode_sdk::v2::{
    AgentInfo, CreatePermissionRequest, CreateSessionRequest, Delivery, FileSystemEntry,
    FileSystemEntryType, IntegrationKeyRequest, IntegrationOauthRequest, Located, LocationInfo,
    LocationRef, ModelInfo, ModelRef, PermissionReply, PermissionSource, ProjectLocationInfo,
    PromptInput, PromptRequest, QuestionReplyRequest, ReplyPermissionRequest, RevertStageRequest,
    Session, SessionHistory,
};

#[test]
fn v2_create_and_prompt_shapes_match_upstream() {
    let create = CreateSessionRequest {
        id: Some("ses_1".into()),
        agent: Some("build".into()),
        model: Some(ModelRef {
            id: "model".into(),
            provider_id: "provider".into(),
            variant: Some("fast".into()),
        }),
        location: Some(LocationRef {
            directory: "/tmp/project".into(),
            workspace_id: Some("ws_1".into()),
        }),
    };

    let value = serde_json::to_value(create).unwrap();
    assert_eq!(value["model"]["id"], "model");
    assert_eq!(value["model"]["providerID"], "provider");
    assert_eq!(value["location"]["directory"], "/tmp/project");
    assert_eq!(value["location"]["workspaceID"], "ws_1");

    let prompt = PromptRequest {
        id: Some("msg_1".into()),
        prompt: Some(PromptInput::text("hello")),
        delivery: Some(Delivery::Steer),
        resume: Some(true),
    };
    let value = serde_json::to_value(prompt).unwrap();
    assert_eq!(value["id"], "msg_1");
    assert_eq!(value["prompt"]["text"], "hello");
    assert_eq!(value["delivery"], "steer");
    assert_eq!(value["resume"], true);
}

#[test]
fn v2_session_response_uses_id_acronym_fields() {
    let value = serde_json::json!({
        "id": "ses_1",
        "projectID": "project_1",
        "agent": "build",
        "model": {
            "id": "model",
            "providerID": "provider"
        },
        "cost": 0,
        "tokens": {
            "input": 1,
            "output": 2,
            "reasoning": 3,
            "cache": {
                "read": 4,
                "write": 5
            }
        },
        "time": {
            "created": 1,
            "updated": 2
        },
        "title": "hello",
        "location": {
            "directory": "/tmp/project",
            "workspaceID": "ws_1"
        }
    });

    let session: Session = serde_json::from_value(value).unwrap();
    assert_eq!(session.project_id, "project_1");
    assert_eq!(session.model.unwrap().provider_id, "provider");
    assert_eq!(session.location.workspace_id.as_deref(), Some("ws_1"));
}

#[test]
fn v2_resource_envelopes_preserve_preview_fields() {
    let value = serde_json::json!({
        "location": {
            "directory": "/tmp/project",
            "workspaceID": "wrk_1",
            "project": {
                "id": "project_1",
                "directory": "/tmp/project"
            }
        },
        "data": [{
            "id": "model_1",
            "providerID": "provider_1",
            "family": "family",
            "name": "Model",
            "api": {},
            "capabilities": {},
            "request": {},
            "variants": [],
            "time": {"released": 1},
            "cost": [],
            "status": "active",
            "enabled": true,
            "limit": {"context": 1000, "output": 100},
            "futureField": true
        }]
    });

    let models: Located<Vec<ModelInfo>> = serde_json::from_value(value).unwrap();
    assert_eq!(models.location.workspace_id.as_deref(), Some("wrk_1"));
    assert_eq!(models.data[0].provider_id, "provider_1");
    assert_eq!(
        models.data[0].extra.get("futureField"),
        Some(&serde_json::Value::Bool(true))
    );
}

#[test]
fn v2_filesystem_entry_type_matches_wire_contract() {
    let entry: FileSystemEntry = serde_json::from_value(serde_json::json!({
        "path": "src/lib.rs",
        "type": "file"
    }))
    .unwrap();
    assert_eq!(entry.entry_type, FileSystemEntryType::File);

    let location = LocationInfo {
        directory: "/tmp/project".into(),
        workspace_id: None,
        project: ProjectLocationInfo {
            id: "project_1".into(),
            directory: "/tmp/project".into(),
        },
    };
    assert_eq!(location.project.id, "project_1");
}

#[test]
fn v2_session_action_bodies_match_upstream() {
    let revert = RevertStageRequest {
        message_id: "msg_1".into(),
        files: Some(true),
    };
    let value = serde_json::to_value(revert).unwrap();
    assert_eq!(value["messageID"], "msg_1");
    assert_eq!(value["files"], true);

    let permission = CreatePermissionRequest {
        id: Some("per_1".into()),
        action: "read".into(),
        resources: vec!["src/lib.rs".into()],
        save: vec!["src/*".into()],
        metadata: Some(serde_json::json!({"reason": "test"})),
        source: Some(PermissionSource {
            source_type: "tool".into(),
            message_id: "msg_1".into(),
            call_id: "call_1".into(),
        }),
        agent: Some("build".into()),
    };
    let value = serde_json::to_value(permission).unwrap();
    assert_eq!(value["id"], "per_1");
    assert_eq!(value["action"], "read");
    assert_eq!(value["source"]["type"], "tool");
    assert_eq!(value["source"]["messageID"], "msg_1");
    assert_eq!(value["source"]["callID"], "call_1");

    let reply = ReplyPermissionRequest {
        reply: PermissionReply::Always,
        message: Some("approved".into()),
    };
    let value = serde_json::to_value(reply).unwrap();
    assert_eq!(value["reply"], "always");
    assert_eq!(value["message"], "approved");

    let question = QuestionReplyRequest {
        answers: vec![vec!["A".into()], vec!["B".into(), "C".into()]],
    };
    let value = serde_json::to_value(question).unwrap();
    assert_eq!(value["answers"][0][0], "A");
    assert_eq!(value["answers"][1][1], "C");
}

#[test]
fn v2_session_history_uses_camel_case_has_more() {
    let history: SessionHistory = serde_json::from_value(serde_json::json!({
        "data": [{"type": "session.created"}],
        "hasMore": true
    }))
    .unwrap();
    assert!(history.has_more);
    assert_eq!(history.data.len(), 1);
}

#[test]
fn v2_discovery_records_preserve_preview_fields() {
    let value = serde_json::json!({
        "id": "build",
        "request": {"headers": {}},
        "mode": "primary",
        "hidden": false,
        "permissions": {},
        "futureAgentField": 42
    });
    let agent: AgentInfo = serde_json::from_value(value).unwrap();
    assert_eq!(agent.id, "build");
    assert_eq!(
        agent.extra.get("futureAgentField"),
        Some(&serde_json::json!(42))
    );
}

#[test]
fn v2_integration_connect_bodies_match_upstream() {
    let key = IntegrationKeyRequest {
        key: "secret".into(),
        label: Some("work".into()),
    };
    let value = serde_json::to_value(key).unwrap();
    assert_eq!(value["key"], "secret");
    assert_eq!(value["label"], "work");

    let oauth = IntegrationOauthRequest {
        method_id: "oauth".into(),
        inputs: [("region".into(), "us".into())].into_iter().collect(),
        label: Some("work".into()),
    };
    let value = serde_json::to_value(oauth).unwrap();
    assert_eq!(value["methodID"], "oauth");
    assert_eq!(value["inputs"]["region"], "us");
    assert_eq!(value["label"], "work");
}
