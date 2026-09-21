# unofficial-opencode-sdk

Unofficial Rust SDK for the OpenCode server HTTP and SSE APIs.

> **Unofficial:** this project is independent from OpenCode and is not affiliated
> with or endorsed by the OpenCode project.

The current compatibility snapshot is OpenCode commit
`ba341c6cac5ed1ef867ef127245a42b5e33d4946` from its `dev` branch.

## Scope

The first vertical slice is deliberately client-only. It connects to an already
running OpenCode server and provides:

- `Client::builder()` with base URL, directory/workspace context, and Basic Auth.
- `client.session().create/list/get/prompt/abort`.
- `client.events().subscribe()` for the `/event` SSE stream.
- forward-compatible event decoding: selected common events are classified,
  unknown event types retain their JSON value, malformed JSON is surfaced as
  event data instead of terminating the stream.
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

## Client-only usage

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

## Sessions and prompts

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

Set `ClientBuilder::directory(...)` when the request must be routed to a
specific OpenCode directory/workspace context.

## Events / SSE

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

This repository checks the generated contract into `spec/openapi.json` and
records the exact source in `spec/upstream.json`.

Whole-document Rust model generation is **not** claimed in this first slice.
The current OpenAPI Generator Rust feature matrix lacks `anyOf`/union/null
support used by this OpenAPI 3.1 document, while Progenitor 0.15.0 documents
OpenAPI 3.0.x as its input target. Instead:

- `scripts/generate_contract.py` validates and generates the selected operation
  manifest from the authoritative snapshot.
- session/prompt compatibility models and the ergonomic facade are handwritten,
  small, and preserve unknown fields where practical.
- SSE is handwritten because upstream itself must supplement the OpenAPI
  response schema for SSE.

This boundary favors protocol correctness over producing a large but misleading
generated Rust tree.

## Updating OpenCode

`spec/upstream.json` is the machine-readable compatibility pin. To update:

1. change `upstream_commit` after reviewing the desired OpenCode commit;
2. run `scripts/update-openapi`, or dispatch the
   `refresh-and-smoke-contract` GitHub Actions workflow;
3. review `spec/openapi.json` and `src/generated/operations.rs`;
4. run the Rust CI suite and the real-server contract smoke test.

The generator version and upstream JS generator metadata are pinned in
`spec/upstream.json`.

## Compatibility policy

Until the crate has broader generated-model coverage, compatibility is tied to
the exact OpenCode commit in `spec/upstream.json`. Additive JSON fields are
retained or tolerated. Unknown SSE event types do not terminate the stream.
Breaking request/response changes require a new snapshot and SDK update.

## Known limitations

- only a representative session vertical slice has an ergonomic facade;
- full OpenAPI 3.1 model generation is not yet enabled;
- non-text prompt parts currently use a JSON escape hatch;
- prompt response message/part unions are retained as JSON in this slice;
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
pinned upstream source, starts that exact OpenCode server, creates/lists/gets/
aborts a session, opens the SSE endpoint, and commits the verified snapshot
back to `main` when it changed.

## Publishing

Release packaging and crates.io Trusted Publishing are documented in
[`PUBLISHING.md`](PUBLISHING.md). The repository does not store a crates.io
API token.
