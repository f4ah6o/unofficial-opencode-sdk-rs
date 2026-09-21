use unofficial_opencode_sdk::v2::{
    CreateSessionRequest, Delivery, FileSystemEntry, FileSystemEntryType, Located, LocationInfo,
    LocationRef, ModelInfo, ModelRef, ProjectLocationInfo, PromptInput, PromptRequest, Session,
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
