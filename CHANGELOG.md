# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

## [0.1.0] - 2026-09-22

### Added

- Client-only Rust SDK for an existing OpenCode server.
- Legacy/current session create, list, get, prompt, and abort operations.
- Isolated `unofficial_opencode_sdk::v2` preview namespace for OpenCode's `/api/*` contract.
- V2 session list, create, get, prompt, wait, interrupt, and durable-event operations.
- V2 native server event subscription with raw SSE data for forward compatibility.
- Basic Auth and directory/workspace request context.
- Explicit compatibility metadata for OpenCode 1.18.31 and `@opencode-ai/sdk` 1.18.31.
- Pinned OpenCode OpenAPI contract snapshot and reproducible operation manifest.
- Real-server contract smoke testing in GitHub Actions.
