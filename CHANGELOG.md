# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Added

- OpenCode 2.x native `/api/*` support in `unofficial_opencode_sdk::v2`: the
  client probes `GET api/info` once (`v2::Client::serve_dialect()`) and
  adapts to the 1.x preview or 2.x native dialect automatically.
- `v2::ServeDialect` (`Preview1x`/`Native2x`) and `Error::Unsupported` for
  routes with no equivalent on the connected dialect.
- 2.x routes: `health()` uses `api/info`, question/form lists use `api/form`
  and `api/session/{id}/form`, `wait()` uses
  `api/experimental/session/{id}/wait`, `events()` subscribes to the global
  `api/event` stream, `revert.clear()` uses `DELETE api/session/{id}/revert`,
  `question.reject()` uses `DELETE api/session/{id}/form/{id}`, and
  `credential().activate()` calls `api/credential/{id}/activate`.
- `SessionQuestionApi::reply_answer` for keyed 2.x form answers.
- `SessionInputAdmitted::from_wire` normalizes the 2.x durable user-message
  admission shape (and remains the decoder used by `prompt()`).
- Live-contract coverage now exercises the V2 surface against both an
  OpenCode 1.x preview server and a real OpenCode 2.x server, gated by the
  detected dialect, with optional `OPENCODE_TEST_USERNAME`/
  `OPENCODE_TEST_PASSWORD` credentials (2.x serve always requires Basic auth).

### Changed

- Prompt requests send both the 1.x `prompt.text` and the 2.x top-level
  `text` so one payload is accepted by either dialect.
- `SessionInputAdmitted::admitted_seq` is now `Option<u64>` (2.x admissions
  carry no durable inbox sequence).
- `Session::title` is now `Option<String>` (absent on unsummarized 2.x
  sessions).
- `LocationInfo::project` is now `Option<ProjectLocationInfo>` (2.x list
  envelopes carry the public location ref without `project`).
- `ProviderInfo::api`/`request`, `ModelInfo::api`/`request`,
  `CommandInfo::template`, `SkillInfo::location`, and `SkillInfo::content`
  are optional or default-empty since 2.x omits them; 2.x replacements
  (`activation`, `package`, `path`, `modelID`, `idle`, `outcome`, ...)
  are preserved in each record's `extra` map.
- `QuestionRequest` also decodes 2.x `FormInfo` (`title`/`fields` fields).
- `Health` preserves the 2.x `api/info` payload in `extra`.
- Session `history()`, positional question `reply()`, and
  `api/integration/attempt/*` polling return `Error::Unsupported` on
  OpenCode 2.x, which has no equivalents.

## [0.1.0] - 2026-09-22

### Added

- Client-only Rust SDK for an existing OpenCode server.
- Broad V1/current client coverage across global/project/pty/config/tool/instance/path/vcs/session/command/provider/find/file/app/mcp/lsp/formatter/tui/auth/permission/question/event.
- V1/current session lifecycle, children/todo/diff/messages/command/shell/revert/part and permission-response operations.
- Isolated `unofficial_opencode_sdk::v2` preview namespace for OpenCode's `/api/*` contract.
- V2 session list, create, get, active, switch-agent, switch-model, prompt, compact, wait, context, history, message/messages, interrupt, and durable-event operations.
- V2 per-session revert staging/clear/commit, permission request/list/get/reply, and question list/reply/reject APIs.
- V2 health/location/agent/command/skill/reference discovery.
- V2 model/provider discovery, filesystem read/list/find, permission request/saved-permission APIs, and pending-question discovery.
- V2 integration list/get/connect/attempt APIs and credential update/remove.
- V2 native server event subscription with raw SSE data for forward compatibility.
- Basic Auth and directory/workspace request context.
- Explicit compatibility metadata for OpenCode 1.18.31 and `@opencode-ai/sdk` 1.18.31.
- Pinned OpenCode OpenAPI contract snapshot and reproducible operation manifest.
- Real-server contract smoke testing in GitHub Actions.
