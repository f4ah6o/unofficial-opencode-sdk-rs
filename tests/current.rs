use unofficial_opencode_sdk::current::{
    FindFilesOptions, LogLevel, LogRequest, McpAddRequest, PermissionReplyRequest,
    ProviderOauthAuthorizeRequest, ProviderOauthCallbackRequest, PtyCreateRequest,
    QuestionReplyRequest, SessionListOptions, SessionMessagesOptions, ToolListOptions, VcsDiffMode,
    VcsDiffOptions,
};

#[test]
fn current_request_shapes_match_openapi_contract() {
    let pty = PtyCreateRequest {
        command: Some("bash".into()),
        args: Some(vec!["-lc".into(), "echo ok".into()]),
        cwd: Some("/tmp".into()),
        title: Some("test".into()),
        env: Some([("FOO".into(), "bar".into())].into_iter().collect()),
    };
    let value = serde_json::to_value(pty).unwrap();
    assert_eq!(value["command"], "bash");
    assert_eq!(value["args"][0], "-lc");
    assert_eq!(value["env"]["FOO"], "bar");

    let log = LogRequest {
        service: "sdk-test".into(),
        level: LogLevel::Info,
        message: "hello".into(),
        extra: Some(serde_json::json!({"answer": 42})),
    };
    let value = serde_json::to_value(log).unwrap();
    assert_eq!(value["level"], "info");
    assert_eq!(value["extra"]["answer"], 42);

    let oauth = ProviderOauthAuthorizeRequest {
        method: 1,
        inputs: Some([("region".into(), "us".into())].into_iter().collect()),
    };
    let value = serde_json::to_value(oauth).unwrap();
    assert_eq!(value["method"], 1);
    assert_eq!(value["inputs"]["region"], "us");

    let callback = ProviderOauthCallbackRequest {
        method: 1,
        code: Some("code".into()),
    };
    let value = serde_json::to_value(callback).unwrap();
    assert_eq!(value["method"], 1);
    assert_eq!(value["code"], "code");

    let permission = PermissionReplyRequest {
        reply: "always".into(),
        message: Some("approved".into()),
    };
    let value = serde_json::to_value(permission).unwrap();
    assert_eq!(value["reply"], "always");

    let question = QuestionReplyRequest {
        answers: vec![vec!["A".into(), "B".into()]],
    };
    let value = serde_json::to_value(question).unwrap();
    assert_eq!(value["answers"][0][1], "B");
}

#[test]
fn current_query_option_types_cover_official_surface() {
    let session = SessionListOptions {
        scope: Some("project".into()),
        path: Some("/tmp/project".into()),
        roots: Some(true),
        start: Some(10),
        search: Some("hello".into()),
        limit: Some(20),
    };
    assert_eq!(session.scope.as_deref(), Some("project"));

    let messages = SessionMessagesOptions {
        limit: Some(50),
        before: Some("msg_1".into()),
    };
    assert_eq!(messages.limit, Some(50));

    let tools = ToolListOptions {
        provider: "openai".into(),
        model: "gpt".into(),
    };
    assert_eq!(tools.provider, "openai");

    let find = FindFilesOptions {
        query: "Cargo".into(),
        dirs: Some(false),
        entry_type: Some("file".into()),
        limit: Some(100),
    };
    assert_eq!(find.entry_type.as_deref(), Some("file"));

    let diff = VcsDiffOptions {
        mode: VcsDiffMode::Git,
        context: Some(3),
    };
    assert_eq!(diff.context, Some(3));

    let mcp = McpAddRequest {
        name: "example".into(),
        config: serde_json::json!({"type": "remote", "url": "https://example.invalid"}),
    };
    assert_eq!(mcp.name, "example");
}
