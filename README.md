# unofficial-opencode-sdk

Unofficial Rust SDK for the OpenCode server HTTP and SSE APIs.

> **Unofficial:** this project is independent from OpenCode and is not affiliated
> with or endorsed by the OpenCode project.

The current compatibility snapshot targets OpenCode **1.18.31** and
`@opencode-ai/sdk` **1.18.31** at commit
`ba341c6cac5ed1ef867ef127245a42b5e33d4946` from the upstream `dev` branch.

OpenCode currently ships its preview V2 API from the official JavaScript
package's `@opencode-ai/sdk/v2` export while the npm package itself remains on
the 1.x version line. This crate mirrors that separation: legacy/current APIs
remain at the crate root and preview V2 APIs live under
`unofficial_opencode_sdk::v2`.

OpenCode **2.x** serves the `/api/*` contract natively (no `/global/*`
routes): the `v2` client detects the server dialect automatically and works
against both the 1.x preview surface and the native 2.x surface — see
[V2 dialect detection](#v2-dialect-detection).

## Scope

The SDK is deliberately client-only. It connects to an already running
OpenCode server and provides:

- a broad root V1/current surface matching all non-V2 current operations except
  explicit `experimental.*`, `worktree.*`, and `sync.*` families;
- root groups for global/project/pty/config/tool/instance/path/vcs/session/
  command/provider/find/file/app/mcp/lsp/formatter/tui/auth/permission/question/event;
- legacy/current SSE through `client.events().subscribe()` and
  `client.global().events()`;
- isolated `v2::Client` for OpenCode's preview `/api/*` contract;
- V2 session `list/create/get/active/switch-agent/switch-model/prompt/compact/wait/context/history/message/messages/events/interrupt`;
- V2 per-session revert, permission, and question operations;
- V2 health/location/agent/command/skill/reference discovery;
- V2 model/provider discovery;
- V2 integration discovery/connect/attempt APIs and credential update/remove;
- V2 filesystem read/list/find;
- V2 pending/saved permission discovery and saved-permission removal;
- V2 pending question discovery;
- V2 native server event subscription at `/api/event`;
- Basic Auth and directory/workspace context;
- a pinned, reproducible OpenAPI snapshot and operation manifest.

It contains no Temote-specific sandboxing, approvals, worktrees, broker,
checkpoint, task-state, or lifecycle code.

## Install

For the crates.io release:

```toml
[dependencies]
unofficial-opencode-sdk = "0.1"
```

The Rust crate import is `unofficial_opencode_sdk`.

Until the first crates.io release is published, the same package can be used
from Git:

```toml
[dependencies]
unofficial-opencode-sdk = { git = "https://github.com/f4ah6o/unofficial-opencode-sdk-rs" }
```

## Legacy/current client

```rust
use unofficial_opencode_sdk::Client;

let client = Client::builder()
    .base_url("http://127.0.0.1:4096")
    .build()?;
```

If OpenCode is started with `OPENCODE_SERVER_PASSWORD`, configure the matching
Basic Auth credentials. OpenCode defaults the username to `opencode`.

```rust
let client = Client::builder()
    .base_url("http://127.0.0.1:4096")
    .password(std::env::var("OPENCODE_SERVER_PASSWORD")?)
    .build()?;
```

Credentials are never persisted by the SDK.

### Sessions and prompts

```rust
use unofficial_opencode_sdk::{CreateSessionRequest, PromptPart, PromptRequest};

let session = client
    .session()
    .create(&CreateSessionRequest::default())
    .await?;

let response = client
    .session()
    .prompt(
        &session.id,
        &PromptRequest {
            parts: vec![PromptPart::text("Explain this repository")],
            ..Default::default()
        },
    )
    .await?;

client.session().abort(&session.id).await?;
```

Set root `ClientBuilder::directory(...)` and/or `workspace(...)` when the
legacy/current request must be routed to a specific OpenCode context.

The current surface is available directly from the root client:

```rust
let projects = client.project().list().await?;
let config = client.config().get().await?;
let providers = client.provider().list().await?;
let files = client.file().list(".").await?;
let permissions = client.permission().list().await?;
let questions = client.question().list().await?;
```

Complex upstream unions remain `serde_json::Value` where forcing a Rust enum
would make the preview/additive contract less forward-compatible. Endpoint,
method, path, query, and request-body wiring remain pinned by the generated
OpenAPI operation manifest.

### Events / SSE

```rust
use futures_util::StreamExt;
use unofficial_opencode_sdk::EventData;

let mut events = client.events().subscribe().await?;
while let Some(event) = events.next().await {
    match event?.data {
        EventData::Known(event) => println!("known: {:?}", event.kind),
        EventData::Unknown(value) => println!("future event: {value}"),
        EventData::Malformed { raw, error } => eprintln!("{error}: {raw}"),
    }
}
```

The handwritten SSE parser handles fragmented HTTP chunks, multiple events per
chunk, multiline `data:`, LF, CRLF, comments/unknown SSE fields, malformed
JSON, unknown future OpenCode event types, and disconnect/EOF flushing.

## V2 preview client

V2 is intentionally namespaced so evolving `/api/*` contracts cannot silently
change the legacy/current root API.

You can build a V2 client directly:

```rust
use unofficial_opencode_sdk::v2;

let client = v2::Client::builder()
    .base_url("http://127.0.0.1:4096")
    .directory("/path/to/project")
    .build()?;
```

Or reuse transport/authentication from an existing root client:

```rust
let v2 = client.v2();
```

A configured root-client directory is inherited when using `client.v2()`.
Direct V2 construction additionally supports `workspace_id(...)`.

### V2 dialect detection

The client detects once per client which `/api/*` dialect the server speaks
(`GET api/info` answers JSON `ServerInfo` only on 2.x) and adapts:

| Area | OpenCode 1.x preview | OpenCode 2.x native |
| --- | --- | --- |
| Liveness | `api/health` | `api/info` |
| Pending questions | `api/question/*` | `api/form` (+ `api/session/{id}/form*`) |
| Question reply | `answers` by position | `reply_answer` keyed by form field |
| Session events | `api/session/{id}/event` | global `api/event` (filter by `sessionID`) |
| Session wait | `api/session/{id}/wait` | `api/experimental/session/{id}/wait` |
| Revert clear | `POST .../revert/clear` | `DELETE .../revert` |
| Credential activate | n/a | `api/credential/{id}/activate` |

All other V2 routes are identical on both dialects. Payloads admit both wire
shapes: session `title` is optional (2.x), provider/model `api`/`request` are
optional (absent on 2.x), prompt admission normalizes the 1.x admission record
and the 2.x durable user message into `SessionInputAdmitted` (use
`SessionInputAdmitted::from_wire` for raw envelopes), and the prompt request
sends both the 1.x `prompt.text` and the 2.x top-level `text`.

Surfaces with no 2.x equivalent return `Error::Unsupported`: session
`history()` (use `messages()`), positional question `reply()` (use
`reply_answer()`), and `api/integration/attempt/*` polling (2.x moved it
under `api/integration/{id}/connect/{command|oauth}`).

Detected dialect is exposed via `v2.serve_dialect()`.

### V2 sessions

```rust
use unofficial_opencode_sdk::v2::{
    CreateSessionRequest, Delivery, ListSessionsOptions, PromptInput, PromptRequest,
};

let page = v2
    .session()
    .list(&ListSessionsOptions::default())
    .await?;

let session = v2
    .session()
    .create(&CreateSessionRequest::default())
    .await?;

let admitted = v2
    .session()
    .prompt(
        &session.id,
        &PromptRequest {
            prompt: Some(PromptInput::text("Explain this repository")),
            delivery: Some(Delivery::Steer),
            resume: Some(true),
            ..Default::default()
        },
    )
    .await?;

v2.session().wait(&session.id).await?;

let context = v2.session().context(&session.id).await?;
let history = v2.session().history(&session.id, Some(50), None).await?;
let messages = v2
    .session()
    .messages(&session.id, &Default::default())
    .await?;

let pending_permissions = v2.session().permission(&session.id).list().await?;
let pending_questions = v2.session().question(&session.id).list().await?;
```

V2 prompt admission is modeled separately from legacy/current message responses:
the endpoint durably admits input and returns its admission record.

Session message and durable-history unions are kept as raw JSON in this preview
surface because upstream V2 currently exposes several message/event variants.
The surrounding pagination and lifecycle structures remain typed.

Per-session helpers follow the official SDK grouping:

```rust
let permissions = v2.session().permission(&session.id);
let questions = v2.session().question(&session.id);
let revert = v2.session().revert(&session.id);
```

Permission/question reply and revert operations are available, but CI does not
create artificial pending approvals/questions or mutate repository files merely
to exercise destructive workflow paths.

### V2 discovery, integrations, filesystem, permissions, and questions

```rust
let health = v2.health().get().await?;
let location = v2.location().get(None).await?;
let agents = v2.agent().list(None).await?;
let commands = v2.command().list(None).await?;
let skills = v2.skill().list(None).await?;
let references = v2.reference().list(None).await?;
let integrations = v2.integration().list(None).await?;
```

Integration key/OAuth connect, OAuth attempt status/cancel/complete, and
credential update/remove are also exposed. CI does not use real integration
credentials or mutate stored credentials.


```rust
use unofficial_opencode_sdk::v2::FindFilesOptions;

let models = v2.model().list(None).await?;
let providers = v2.provider().list(None).await?;

let root = v2.fs().list(None, None).await?;
let matches = v2
    .fs()
    .find(&FindFilesOptions {
        query: "Cargo".into(),
        ..Default::default()
    })
    .await?;

let pending_permissions = v2.permission().request().list(None).await?;
let saved_permissions = v2.permission().saved().list(None).await?;
let pending_questions = v2.question().request().list(None).await?;
```

These location-aware V2 endpoints use OpenCode's deep-object query shape
(`location[directory]` / `location[workspace]`). The client's configured
directory/workspace is used by default; `LocationQuery` can override it for
one request.

Model/provider records type stable identity and status fields while retaining
preview provider-specific nested structures as JSON. This keeps the public Rust
surface useful without pretending the evolving V2 contract is stable.

### V2 events

```rust
use futures_util::StreamExt;

let mut events = v2.session().events(&session.id, None).await?;
while let Some(event) = events.next().await {
    let event = event?;
    println!("{:?}: {}", event.event, event.data);
}
```

V2 SSE data is retained as raw text instead of being forced into the
legacy/current event enum. Upstream's V2 durable-event schema is still evolving,
so this is deliberately forward-compatible.

## Contract and generation boundary

OpenCode does **not** check a permanent SDK OpenAPI artifact into
`packages/sdk/js`. At the pinned snapshot:

1. `Server.openapi()` builds the contract from the Effect `PublicApi`.
2. `matchLegacyOpenApi` normalizes that schema for the public/current API,
   including explicit SSE payload schemas.
3. `bun dev generate` emits the OpenAPI document used by the SDK build.
4. the official JS SDK uses `@hey-api/openapi-ts@0.90.10`, applies small
   generated-code compatibility patches, then deletes its temporary
   `openapi.json`.
5. the same generated contract contains preview V2 operation IDs such as
   `v2.session.create` and `v2.event.subscribe`.

This repository checks the generated contract into `spec/openapi.json` and
records the exact upstream version, official SDK version, commit, and V2 status
in `spec/upstream.json`.

Whole-document Rust model generation is **not** claimed in this first slice.
The current OpenAPI Generator Rust feature matrix lacks `anyOf`/union/null
support used by this OpenAPI 3.1 document, while Progenitor 0.15.0 documents
OpenAPI 3.0.x as its input target. Instead:

- `scripts/generate_contract.py` validates all stable V1/current operations
  plus the explicitly supported V2 preview operations from the authoritative snapshot;
- session/prompt compatibility models and ergonomic facades are handwritten,
  small, and preserve unknown fields where practical;
- SSE is handwritten because upstream itself must supplement the OpenAPI
  response schema for SSE.

This boundary favors protocol correctness over producing a large but misleading
generated Rust tree.

## Updating OpenCode

`spec/upstream.json` is the machine-readable compatibility pin. To update:

1. change `upstream_version` and `upstream_commit` after reviewing the desired
   OpenCode release/commit;
2. update `official_js_sdk.version` to match the pinned upstream SDK;
3. run `scripts/update-openapi`, or dispatch the
   `refresh-and-smoke-contract` GitHub Actions workflow;
4. review `spec/openapi.json` and `src/generated/operations.rs`;
5. run the Rust CI suite and the real-server contract smoke test.

Cargo package metadata also records the pinned OpenCode and official SDK
versions. The Rust crate's own SemVer remains independent so compatibility
updates and Rust-only fixes can be released separately.

## Compatibility policy

For `0.1.x`, compatibility is tied to the exact OpenCode commit in
`spec/upstream.json`.

The root surface tracks the stable legacy/current API. Explicit
`experimental.*`, `worktree.*`, and `sync.*` operation families are not
part of the V1/current completion target. The `v2` module tracks
OpenCode's preview `/api/*` and `@opencode-ai/sdk/v2` surface and may require
Rust SDK changes as upstream V2 evolves. Additive JSON fields are retained or
tolerated where practical. V2 SSE payloads remain raw. Breaking request/response
changes require a new snapshot and SDK update.

## Known limitations

- V1/current endpoint coverage is broad, but complex config/provider/MCP/TUI and
  message/part unions use `serde_json::Value` instead of exhaustive Rust enums;
- explicit `experimental.*`, `worktree.*`, and `sync.*` families are outside
  the V1/current completion target;
- V2 session coverage is broad, but message/durable-event variant payloads remain raw JSON while upstream V2 is preview;
- V2 remains preview upstream and is not claimed stable;
- full OpenAPI 3.1 model generation is not yet enabled;
- legacy/current non-text prompt parts currently use a JSON escape hatch;
- legacy/current prompt response message/part unions are retained as JSON;
- V2 admitted prompt output is retained losslessly as JSON while its event/input
  schema continues to evolve;
- `createOpencode()`-style process launching is intentionally not included;
- the required live contract smoke avoids a real model prompt because model
  credentials are not appropriate for repository CI.

## Validation

Normal CI runs:

```text
cargo fmt --check
cargo check --all-targets --all-features
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo doc --no-deps --all-features
cargo publish --dry-run
```

CI also checks the declared Rust 1.87 MSRV.

The contract workflow additionally generates the OpenAPI snapshot from the
pinned upstream source, starts that exact OpenCode server, exercises broad
read-only/ephemeral V1/current coverage plus the V2 smoke paths, opens SSE
endpoints, and commits the verified snapshot back to `main` when it changed.

## Publishing

Release packaging and crates.io Trusted Publishing are documented in
[`PUBLISHING.md`](PUBLISHING.md). The repository does not store a crates.io
API token.
