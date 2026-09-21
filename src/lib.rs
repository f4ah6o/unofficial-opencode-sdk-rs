//! Unofficial Rust SDK for the OpenCode server.
//!
//! This crate is not affiliated with or endorsed by the OpenCode project.

mod client;
mod error;
pub mod generated;
mod models;
mod sse;

pub use client::{Client, ClientBuilder, EventsApi, SessionApi};
pub use error::{ApiError, Error};
pub use models::{
    CreateSessionRequest, MessageWithParts, ModelRef, PromptPart, PromptRequest, Session,
    SessionTime,
};
pub use sse::{
    EventData, EventStream, KnownEvent, KnownEventKind, OpenCodeEvent, SseDecoder, SseFrame,
};
