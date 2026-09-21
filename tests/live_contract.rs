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
    let client = Client::builder().base_url(base_url).build().unwrap();

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

    let v2 = client.v2();
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

    v2.session()
        .history(&v2_created.id, Some(20), None)
        .await
        .expect("get v2 session history");

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
