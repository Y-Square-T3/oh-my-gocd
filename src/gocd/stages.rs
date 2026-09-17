// Stages section of the GoCD API docs: thin GocdCall constructors for
// /go/api/stages, API v2. GoCD documents no If-Match guard; its bodyless POST
// instead carries the documented X-GoCD-Confirm header.

use super::GocdCall;

pub fn run(pipeline_name: &str, pipeline_counter: &str, stage_name: &str) -> GocdCall {
    GocdCall::post(&format!(
        "api/stages/{pipeline_name}/{pipeline_counter}/{stage_name}/run"
    ))
    .version(2)
    .confirm()
}
