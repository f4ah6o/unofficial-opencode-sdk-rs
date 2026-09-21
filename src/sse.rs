use std::collections::VecDeque;
use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::Bytes;
use futures_core::Stream;
use futures_util::StreamExt;
use futures_util::stream::{self, BoxStream};
use serde_json::Value;

use crate::Error;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SseFrame {
    pub id: Option<String>,
    pub event: Option<String>,
    pub data: String,
}

#[derive(Debug, Default)]
pub struct SseDecoder {
    buffer: Vec<u8>,
}

impl SseDecoder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, chunk: &[u8]) -> Result<Vec<SseFrame>, Error> {
        self.buffer.extend_from_slice(chunk);
        self.drain(false)
    }

    pub fn finish(&mut self) -> Result<Vec<SseFrame>, Error> {
        self.drain(true)
    }

    fn drain(&mut self, flush: bool) -> Result<Vec<SseFrame>, Error> {
        let mut frames = Vec::new();
        while let Some((end, separator_len)) = find_separator(&self.buffer) {
            let raw: Vec<u8> = self.buffer.drain(..end + separator_len).collect();
            if let Some(frame) = parse_frame(&raw[..end])? {
                frames.push(frame);
            }
        }

        if flush && !self.buffer.is_empty() {
            let raw = std::mem::take(&mut self.buffer);
            if let Some(frame) = parse_frame(&raw)? {
                frames.push(frame);
            }
        }
        Ok(frames)
    }
}

fn find_separator(input: &[u8]) -> Option<(usize, usize)> {
    let mut i = 0;
    while i < input.len() {
        if input.get(i..i + 4) == Some(b"\r\n\r\n") {
            return Some((i, 4));
        }
        if input.get(i..i + 2) == Some(b"\n\n") {
            return Some((i, 2));
        }
        i += 1;
    }
    None
}

fn parse_frame(raw: &[u8]) -> Result<Option<SseFrame>, Error> {
    let text = std::str::from_utf8(raw)
        .map_err(|error| Error::Sse(format!("invalid UTF-8 in SSE frame: {error}")))?;
    let normalized = text.replace("\r\n", "\n");
    let mut id = None;
    let mut event = None;
    let mut data = Vec::new();

    for line in normalized.lines() {
        if line.starts_with(':') || line.is_empty() {
            continue;
        }
        let (field, value) = line
            .split_once(':')
            .map(|(field, value)| (field, value.strip_prefix(' ').unwrap_or(value)))
            .unwrap_or((line, ""));
        match field {
            "id" => id = Some(value.to_owned()),
            "event" => event = Some(value.to_owned()),
            "data" => data.push(value),
            _ => {}
        }
    }

    if data.is_empty() {
        return Ok(None);
    }

    Ok(Some(SseFrame {
        id,
        event,
        data: data.join("\n"),
    }))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum KnownEventKind {
    SessionCreated,
    SessionUpdated,
    SessionStatus,
    SessionError,
    PermissionAsked,
    PermissionUpdated,
}

impl KnownEventKind {
    fn from_type(value: &str) -> Option<Self> {
        match value {
            "session.created" => Some(Self::SessionCreated),
            "session.updated" => Some(Self::SessionUpdated),
            "session.status" => Some(Self::SessionStatus),
            "session.error" => Some(Self::SessionError),
            "permission.asked" => Some(Self::PermissionAsked),
            "permission.updated" => Some(Self::PermissionUpdated),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KnownEvent {
    pub kind: KnownEventKind,
    pub event_type: String,
    pub properties: Value,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum EventData {
    Known(KnownEvent),
    Unknown(Value),
    Malformed { raw: String, error: String },
}

#[derive(Debug, Clone, PartialEq)]
pub struct OpenCodeEvent {
    pub id: Option<String>,
    pub event: Option<String>,
    pub data: EventData,
}

impl OpenCodeEvent {
    fn from_frame(frame: SseFrame) -> Self {
        let data = match serde_json::from_str::<Value>(&frame.data) {
            Ok(value) => {
                let event_type = value.get("type").and_then(Value::as_str);
                match event_type.and_then(KnownEventKind::from_type) {
                    Some(kind) => EventData::Known(KnownEvent {
                        kind,
                        event_type: event_type.unwrap_or_default().to_owned(),
                        properties: value.get("properties").cloned().unwrap_or(Value::Null),
                        value,
                    }),
                    None => EventData::Unknown(value),
                }
            }
            Err(error) => EventData::Malformed {
                raw: frame.data,
                error: error.to_string(),
            },
        };
        Self {
            id: frame.id,
            event: frame.event,
            data,
        }
    }
}

pub struct EventStream {
    inner: BoxStream<'static, Result<OpenCodeEvent, Error>>,
}

impl Stream for EventStream {
    type Item = Result<OpenCodeEvent, Error>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.inner).poll_next(cx)
    }
}

type ByteStream = BoxStream<'static, Result<Bytes, reqwest::Error>>;

struct State {
    bytes: ByteStream,
    decoder: SseDecoder,
    queued: VecDeque<OpenCodeEvent>,
    finished: bool,
}

pub(crate) fn event_stream_from_response(response: reqwest::Response) -> EventStream {
    let state = State {
        bytes: response.bytes_stream().boxed(),
        decoder: SseDecoder::new(),
        queued: VecDeque::new(),
        finished: false,
    };
    let inner = stream::unfold(state, |mut state| async move {
        loop {
            if let Some(event) = state.queued.pop_front() {
                return Some((Ok(event), state));
            }
            if state.finished {
                return None;
            }
            match state.bytes.next().await {
                Some(Ok(chunk)) => match state.decoder.push(&chunk) {
                    Ok(frames) => {
                        state
                            .queued
                            .extend(frames.into_iter().map(OpenCodeEvent::from_frame));
                    }
                    Err(error) => return Some((Err(error), state)),
                },
                Some(Err(error)) => return Some((Err(Error::Http(error)), state)),
                None => {
                    state.finished = true;
                    match state.decoder.finish() {
                        Ok(frames) => state
                            .queued
                            .extend(frames.into_iter().map(OpenCodeEvent::from_frame)),
                        Err(error) => return Some((Err(error), state)),
                    }
                }
            }
        }
    })
    .boxed();
    EventStream { inner }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fragmented_and_multiple_events() {
        let mut decoder = SseDecoder::new();
        assert!(
            decoder
                .push(b"data: {\"type\":\"session.")
                .unwrap()
                .is_empty()
        );
        let frames = decoder
            .push(b"created\"}\n\ndata: {\"type\":\"future\"}\n\n")
            .unwrap();
        assert_eq!(frames.len(), 2);
        assert!(frames[0].data.contains("session.created"));
        assert!(frames[1].data.contains("future"));
    }

    #[test]
    fn multiline_and_crlf() {
        let mut decoder = SseDecoder::new();
        let frames = decoder
            .push(b"id: 1\r\nevent: message\r\ndata: first\r\ndata: second\r\n\r\n")
            .unwrap();
        assert_eq!(
            frames,
            vec![SseFrame {
                id: Some("1".into()),
                event: Some("message".into()),
                data: "first\nsecond".into(),
            }]
        );
    }

    #[test]
    fn malformed_json_is_data_not_stream_failure() {
        let event = OpenCodeEvent::from_frame(SseFrame {
            id: None,
            event: None,
            data: "{nope".into(),
        });
        assert!(matches!(event.data, EventData::Malformed { .. }));
    }

    #[test]
    fn unknown_event_is_forward_compatible() {
        let event = OpenCodeEvent::from_frame(SseFrame {
            id: None,
            event: None,
            data: r#"{"type":"future.event","properties":{"x":1}}"#.into(),
        });
        assert!(matches!(event.data, EventData::Unknown(_)));
    }

    #[test]
    fn finish_flushes_final_frame() {
        let mut decoder = SseDecoder::new();
        assert!(
            decoder
                .push(b"data: {\"type\":\"session.status\"}")
                .unwrap()
                .is_empty()
        );
        assert_eq!(decoder.finish().unwrap().len(), 1);
    }
}
