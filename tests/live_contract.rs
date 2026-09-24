use unofficial_opencode_sdk::current::{
    FindFilesOptions as CurrentFindFilesOptions, SessionListOptions as CurrentSessionListOptions,
    SessionMessagesOptions as CurrentSessionMessagesOptions, VcsDiffMode, VcsDiffOptions,
};
use unofficial_opencode_sdk::v2::{
    CreateSessionRequest as V2CreateSessionRequest, FileSystemEntryType, FindFilesOptions,
    ListSessionsOptions,
};
use unofficial_opencode_sdk::{Client, CreateSessionRequest};

#[tokio::test]
#[ignore = "requires a real OpenCode server; exercised by contract workflow"]
async fn live_session_and_sse_contract() {
    let base_url = std::env::var("OPENCODE_TEST_BASE_URL")
        .expect("OPENCODE_TEST_BASE_URL must point to a running OpenCode server");
    let mut builder = Client::builder().base_url(base_url);
    // OpenCode 2.x serve always requires Basic auth (a generated password
    // when unset); 1.x serves without it by default.
    if let Ok(password) = std::env::var("OPENCODE_TEST_PASSWORD") {
        let username =
            std::env::var("OPENCODE_TEST_USERNAME").unwrap_or_else(|_| "opencode".to_owned());
        builder = builder.basic_auth(username, password);
    }
    let client = builder.build().unwrap();

    let v2 = client.v2();
    let dialect = v2.serve_dialect().await.expect("detect serve dialect");

    // The legacy/current client surface is 1.x-only: OpenCode 2.x serves
    // `/api/*` natively and no `/session`/`/global/*` routes at all.
    if matches!(
        dialect,
        unofficial_opencode_sdk::v2::ServeDialect::Preview1x
    ) {
        let created = client
            .session()
            .create(&CreateSessionRequest {
                title: Some("unofficial-opencode-sdk contract smoke".into()),
                ..Default::default()
            })
            .await
            .expect("create session");

        let fetched = client
            .session()
            .get(&created.id)
            .await
            .expect("get session");
        assert_eq!(created.id, fetched.id);

        let global_health = client.global().health().await.expect("get global health");
        assert!(global_health.healthy);
        client.global().config().await.expect("get global config");

        client
            .project()
            .list()
            .await
            .expect("list current projects");
        client
            .project()
            .current()
            .await
            .expect("get current project");
        client.pty().list().await.expect("list current ptys");
        client.pty().shells().await.expect("list current shells");
        client.config().get().await.expect("get current config");
        client
            .config()
            .providers()
            .await
            .expect("get current config providers");
        client.tool().ids().await.expect("list current tool ids");
        client.path().get().await.expect("get current path");
        client.vcs().get().await.expect("get current vcs");
        client.vcs().status().await.expect("get current vcs status");
        client
            .vcs()
            .diff(&VcsDiffOptions {
                mode: VcsDiffMode::Git,
                context: Some(1),
            })
            .await
            .expect("get current vcs diff");

        client
            .session()
            .status()
            .await
            .expect("get current session status");
        client
            .session()
            .update(
                &created.id,
                &serde_json::json!({"title": "current contract smoke updated"}),
            )
            .await
            .expect("update current session");
        client
            .session()
            .children(&created.id)
            .await
            .expect("list current session children");
        client
            .session()
            .todo(&created.id)
            .await
            .expect("get current session todo");
        client
            .session()
            .messages(&created.id, &CurrentSessionMessagesOptions::default())
            .await
            .expect("get current session messages");
        client
            .session()
            .diff(&created.id, None)
            .await
            .expect("get current session diff");
        client
            .session()
            .list_with(&CurrentSessionListOptions {
                limit: Some(20),
                ..Default::default()
            })
            .await
            .expect("list current sessions with options");

        client
            .command()
            .list()
            .await
            .expect("list current commands");
        client
            .provider()
            .list()
            .await
            .expect("list current providers");
        client
            .provider()
            .auth()
            .await
            .expect("list current provider auth methods");
        client
            .find()
            .files(&CurrentFindFilesOptions {
                query: "package".into(),
                dirs: Some(false),
                entry_type: Some("file".into()),
                limit: Some(20),
            })
            .await
            .expect("find current files");
        client.file().list(".").await.expect("list current files");
        client
            .file()
            .status()
            .await
            .expect("get current file status");
        client.app().agents().await.expect("list current agents");
        client.app().skills().await.expect("list current skills");
        client.mcp().status().await.expect("get current mcp status");
        client.lsp().status().await.expect("get current lsp status");
        client
            .formatter()
            .status()
            .await
            .expect("get current formatter status");
        client
            .permission()
            .list()
            .await
            .expect("list current permissions");
        client
            .question()
            .list()
            .await
            .expect("list current questions");

        let listed = client.session().list().await.expect("list sessions");
        assert!(listed.iter().any(|session| session.id == created.id));

        let stream = client
            .events()
            .subscribe()
            .await
            .expect("connect SSE endpoint");
        drop(stream);

        assert!(
            client
                .session()
                .abort(&created.id)
                .await
                .expect("abort session")
        );

        let forked = client
            .session()
            .fork(&created.id, None)
            .await
            .expect("fork current session");
        assert!(
            client
                .session()
                .delete(&forked.id)
                .await
                .expect("delete forked current session")
        );

        assert!(
            client
                .session()
                .delete(&created.id)
                .await
                .expect("delete current smoke session")
        );
    }
    let v2_created = v2
        .session()
        .create(&V2CreateSessionRequest::default())
        .await
        .expect("create v2 session");

    let v2_fetched = v2
        .session()
        .get(&v2_created.id)
        .await
        .expect("get v2 session");
    assert_eq!(v2_created.id, v2_fetched.id);

    let v2_page = v2
        .session()
        .list(&ListSessionsOptions::default())
        .await
        .expect("list v2 sessions");
    assert!(
        v2_page
            .data
            .iter()
            .any(|session| session.id == v2_created.id)
    );

    // Durable across serve restarts on 2.x, so the id must be unique per run.
    let prompt_id = format!(
        "msg_live_contract_{:x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis()
    );
    let admitted = v2
        .session()
        .prompt(
            &v2_created.id,
            &unofficial_opencode_sdk::v2::PromptRequest {
                id: Some(prompt_id.clone()),
                prompt: Some(unofficial_opencode_sdk::v2::PromptInput::text(
                    "reply with exactly: OK",
                )),
                delivery: Some(unofficial_opencode_sdk::v2::Delivery::Queue),
                resume: Some(true),
            },
        )
        .await
        .expect("admit v2 prompt");
    assert_eq!(admitted.session_id, v2_created.id);

    // 2.x `session.wait` answers 503 "not available yet" on some servers;
    // the admission above is the contract — poll messages until the user
    // message (and any assistant/error record) is durable.
    let mut saw_user_message = false;
    for _ in 0..20 {
        let page = v2
            .session()
            .messages(&v2_created.id, &Default::default())
            .await
            .expect("get v2 session messages");
        saw_user_message = page.data.iter().any(|message| {
            message.get("id").and_then(|id| id.as_str()) == Some(prompt_id.as_str())
        });
        if saw_user_message {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    }
    assert!(saw_user_message);

    v2.session()
        .interrupt(&v2_created.id)
        .await
        .expect("interrupt v2 session");

    v2.session()
        .active()
        .await
        .expect("list active v2 sessions");

    v2.session()
        .context(&v2_created.id)
        .await
        .expect("get v2 session context");

    // `api/session/{id}/history` exists only on the 1.x preview surface.
    match v2.serve_dialect().await.expect("detect serve dialect") {
        unofficial_opencode_sdk::v2::ServeDialect::Preview1x => {
            v2.session()
                .history(&v2_created.id, Some(20), None)
                .await
                .expect("get v2 session history");
        }
        unofficial_opencode_sdk::v2::ServeDialect::Native2x => {
            assert!(matches!(
                v2.session().history(&v2_created.id, Some(20), None).await,
                Err(unofficial_opencode_sdk::Error::Unsupported(_))
            ));
        }
    }

    v2.session()
        .messages(&v2_created.id, &Default::default())
        .await
        .expect("get v2 session messages");

    v2.session()
        .permission(&v2_created.id)
        .list()
        .await
        .expect("list v2 session permissions");

    v2.session()
        .question(&v2_created.id)
        .list()
        .await
        .expect("list v2 session questions");

    let models = v2.model().list(None).await.expect("list v2 models");
    assert!(!models.location.directory.is_empty());

    let providers = v2.provider().list(None).await.expect("list v2 providers");
    assert!(!providers.location.directory.is_empty());
    if let Some(provider) = providers.data.first() {
        let fetched_provider = v2
            .provider()
            .get(&provider.id, None)
            .await
            .expect("get v2 provider");
        assert_eq!(provider.id, fetched_provider.data.id);
    }

    let fs_entries = v2.fs().list(None, None).await.expect("list v2 filesystem");
    assert!(!fs_entries.location.directory.is_empty());
    if let Some(file) = fs_entries
        .data
        .iter()
        .find(|entry| entry.entry_type == FileSystemEntryType::File)
    {
        v2.fs()
            .read(&file.path, None)
            .await
            .expect("read v2 filesystem file");
    }

    v2.fs()
        .find(&FindFilesOptions {
            query: "package".into(),
            ..Default::default()
        })
        .await
        .expect("find v2 filesystem entries");

    v2.permission()
        .request()
        .list(None)
        .await
        .expect("list v2 permission requests");

    v2.permission()
        .saved()
        .list(None)
        .await
        .expect("list v2 saved permissions");

    v2.question()
        .request()
        .list(None)
        .await
        .expect("list v2 question requests");

    let health = v2.health().get().await.expect("get v2 health");
    assert!(health.healthy);

    let location = v2.location().get(None).await.expect("get v2 location");
    assert!(!location.directory.is_empty());

    v2.agent().list(None).await.expect("list v2 agents");
    v2.command().list(None).await.expect("list v2 commands");
    v2.skill().list(None).await.expect("list v2 skills");
    v2.reference().list(None).await.expect("list v2 references");

    let integrations = v2
        .integration()
        .list(None)
        .await
        .expect("list v2 integrations");
    if let Some(integration) = integrations.data.first() {
        let fetched = v2
            .integration()
            .get(&integration.id, None)
            .await
            .expect("get v2 integration");
        assert_eq!(fetched.data.id, integration.id);
    }
}
