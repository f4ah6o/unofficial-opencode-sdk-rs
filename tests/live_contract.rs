use unofficial_opencode_sdk::v2::{
    CreateSessionRequest as V2CreateSessionRequest, ListSessionsOptions,
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
}
