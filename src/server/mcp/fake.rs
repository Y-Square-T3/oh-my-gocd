// The shared test double for the GocdApi seam: one path-asserting fake,
// reused by every section's tool tests. It records each outgoing GocdCall
// and replays canned answers: one outcome for every call, or — for the
// tools that compose several calls — a queue replayed in request order.

use crate::gocd::{GocdApi, GocdCall, GocdError, GocdReply, raw_reply};
use rmcp::model::{CallToolResult, ContentBlock};
use serde_json::Value;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub(crate) struct FakeGocd {
    calls: Mutex<Vec<GocdCall>>,
    outcome: Result<GocdReply, GocdError>,
    queue: Mutex<VecDeque<GocdReply>>,
}

impl FakeGocd {
    fn serving(outcome: Result<GocdReply, GocdError>, queue: VecDeque<GocdReply>) -> Arc<Self> {
        Arc::new(Self {
            calls: Mutex::new(Vec::new()),
            outcome,
            queue: Mutex::new(queue),
        })
    }

    pub(crate) fn replies(body: Value, etag: Option<String>) -> Arc<Self> {
        Self::serving(
            Ok(GocdReply {
                body,
                etag,
                content_type: None,
            }),
            VecDeque::new(),
        )
    }

    /// A canned answer per outgoing call, replayed in request order — for
    /// tools that compose several calls, where the calls' recorded paths
    /// and the exact recorded count pin which reply was which. A null-body
    /// answer backs any call beyond the queue, so an unexpected extra call
    /// shows up in a test's output rather than hanging or guessing.
    pub(crate) fn sequence(replies: Vec<(Value, Option<String>)>) -> Arc<Self> {
        Self::serving(
            Ok(GocdReply {
                body: Value::Null,
                etag: None,
                content_type: None,
            }),
            queue_of(replies),
        )
    }

    /// Like [`Self::sequence`], but every call beyond the queue fails with
    /// `err` — to make one call of a composed tool fail while its
    /// predecessors answer.
    pub(crate) fn sequence_then_fails(
        replies: Vec<(Value, Option<String>)>,
        err: GocdError,
    ) -> Arc<Self> {
        Self::serving(Err(err), queue_of(replies))
    }

    /// A canned `.raw()` reply built by the production mapper, from text.
    pub(crate) fn raw_replies(text: &str, content_type: Option<&str>) -> Arc<Self> {
        Self::raw_bytes_replies(text.as_bytes(), content_type)
    }

    /// A canned `.raw()` reply from the exact bytes GoCD would have sent —
    /// through the production lossy mapper, so a test can feed invalid UTF-8.
    pub(crate) fn raw_bytes_replies(bytes: &[u8], content_type: Option<&str>) -> Arc<Self> {
        Self::serving(
            Ok(raw_reply(
                bytes.to_vec(),
                None,
                content_type.map(str::to_owned),
            )),
            VecDeque::new(),
        )
    }

    pub(crate) fn fails(err: GocdError) -> Arc<Self> {
        Self::serving(Err(err), VecDeque::new())
    }

    pub(crate) fn recorded(&self) -> Vec<GocdCall> {
        self.calls.lock().unwrap().clone()
    }
}

fn queue_of(replies: Vec<(Value, Option<String>)>) -> VecDeque<GocdReply> {
    replies
        .into_iter()
        .map(|(body, etag)| GocdReply {
            body,
            etag,
            content_type: None,
        })
        .collect()
}

impl GocdApi for FakeGocd {
    fn request(
        &self,
        call: GocdCall,
    ) -> Pin<Box<dyn Future<Output = Result<GocdReply, GocdError>> + Send + '_>> {
        self.calls.lock().unwrap().push(call);
        let queued = self.queue.lock().unwrap().pop_front();
        let outcome = match queued {
            Some(reply) => Ok(reply),
            None => self.outcome.clone(),
        };
        Box::pin(async move { outcome })
    }
}

// Tests that call tool methods directly or inspect the whole merged router
// use the full mode; mode-filtered exposure is tested explicitly.
pub(crate) fn service(fake: Arc<FakeGocd>) -> super::OmgMcp {
    service_in(fake, crate::config::Mode::Full)
}

pub(crate) fn service_in(fake: Arc<FakeGocd>, mode: crate::config::Mode) -> super::OmgMcp {
    super::OmgMcp::new(fake, mode)
}

pub(crate) fn first_text(result: &CallToolResult) -> String {
    match result.content.first() {
        Some(ContentBlock::Text(text)) => text.text.clone(),
        other => panic!("expected one text block, got {other:?}"),
    }
}
