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
}
