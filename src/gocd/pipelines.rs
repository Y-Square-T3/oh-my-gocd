// Pipelines section of the GoCD API docs: thin GocdCall constructors for
// /go/api/pipelines, API v1 for the operations and v2 for the comparison.
// GoCD documents no If-Match guard here; its body-less POSTs instead carry
// the documented X-GoCD-Confirm header, which pause and schedule also accept
// in place of an optional JSON body.

use super::GocdCall;
use serde_json::Value;

const PATH: &str = "api/pipelines";

pub fn status(pipeline_name: &str) -> GocdCall {
    GocdCall::get(&format!("{PATH}/{pipeline_name}/status")).version(1)
}

pub fn pause(pipeline_name: &str, body: Option<Value>) -> GocdCall {
    body_or_confirm(&format!("{PATH}/{pipeline_name}/pause"), body)
}

pub fn unpause(pipeline_name: &str) -> GocdCall {
    GocdCall::post(&format!("{PATH}/{pipeline_name}/unpause"))
        .version(1)
        .confirm()
}

pub fn unlock(pipeline_name: &str) -> GocdCall {
    GocdCall::post(&format!("{PATH}/{pipeline_name}/unlock"))
        .version(1)
        .confirm()
}

pub fn schedule(pipeline_name: &str, body: Option<Value>) -> GocdCall {
    body_or_confirm(&format!("{PATH}/{pipeline_name}/schedule"), body)
}

pub fn compare(pipeline_name: &str, from_counter: &str, to_counter: &str) -> GocdCall {
    GocdCall::get(&format!(
        "{PATH}/{pipeline_name}/compare/{from_counter}/{to_counter}"
    ))
    .version(2)
}

/// GoCD accepts a JSON body on these writes; without one it requires the
/// documented X-GoCD-Confirm header instead.
fn body_or_confirm(path: &str, body: Option<Value>) -> GocdCall {
    let call = GocdCall::post(path).version(1);
    match body {
        Some(body) => call.body(body),
        None => call.confirm(),
    }
}
