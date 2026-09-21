use unofficial_opencode_sdk::{CreateSessionRequest, ModelRef, PromptPart, PromptRequest};

#[test]
fn request_shapes_use_upstream_camel_case() {
    let create = CreateSessionRequest {
        parent_id: Some("ses_parent".into()),
        title: Some("hello".into()),
        ..Default::default()
    };
    let value = serde_json::to_value(create).unwrap();
    assert_eq!(value["parentID"], "ses_parent");
    assert!(value.get("parent_id").is_none());

    let prompt = PromptRequest {
        message_id: Some("msg_1".into()),
        model: Some(ModelRef {
            provider_id: "provider".into(),
            model_id: "model".into(),
        }),
        no_reply: Some(true),
        parts: vec![PromptPart::text("hello")],
        ..Default::default()
    };
    let value = serde_json::to_value(prompt).unwrap();
    assert_eq!(value["messageID"], "msg_1");
    assert_eq!(value["model"]["providerID"], "provider");
    assert_eq!(value["model"]["modelID"], "model");
    assert_eq!(value["noReply"], true);
    assert_eq!(value["parts"][0]["type"], "text");
    assert_eq!(value["parts"][0]["text"], "hello");
}
