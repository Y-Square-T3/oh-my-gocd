// The shared test double for the GocdApi seam: one path-asserting fake,
// reused by every section's tool tests. It records each outgoing GocdCall
// and replays one canned outcome.

use crate::gocd::{GocdApi, GocdCall, GocdError, GocdReply};
use rmcp::model::{CallToolResult, ContentBlock};
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
pub(crate) struct FakeGocd {
    calls: Mutex<Vec<GocdCall>>,
    outcome: Result<GocdReply, GocdError>,
}

impl FakeGocd {
    pub(crate) fn replies(body: Value, etag: Option<String>) -> Arc<Self> {
        Arc::new(Self {
            calls: Mutex::new(Vec::new()),
            outcome: Ok(GocdReply { body, etag }),
        })
    }

    pub(crate) fn fails(err: GocdError) -> Arc<Self> {
        Arc::new(Self {
            calls: Mutex::new(Vec::new()),
            outcome: Err(err),
        })
    }

    pub(crate) fn recorded(&self) -> Vec<GocdCall> {
        self.calls.lock().unwrap().clone()
    }
}

impl GocdApi for FakeGocd {
    fn request(
        &self,
        call: GocdCall,
    ) -> Pin<Box<dyn Future<Output = Result<GocdReply, GocdError>> + Send + '_>> {
        self.calls.lock().unwrap().push(call);
        let outcome = self.outcome.clone();
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
