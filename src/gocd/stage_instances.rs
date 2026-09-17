// Stage Instances section of the GoCD API docs: thin GocdCall constructors for
// /go/api/stages, API v3. GoCD documents no If-Match guard here; its bodyless
// POSTs instead carry the documented X-GoCD-Confirm header.

use super::GocdCall;
use serde_json::Value;

const PATH: &str = "api/stages";

pub fn read(
    pipeline_name: &str,
    pipeline_counter: &str,
    stage_name: &str,
    stage_counter: &str,
) -> GocdCall {
    GocdCall::get(&format!(
        "{PATH}/{pipeline_name}/{pipeline_counter}/{stage_name}/{stage_counter}"
    ))
    .version(3)
}

pub fn history(pipeline_name: &str, stage_name: &str, query: Vec<(String, String)>) -> GocdCall {
    GocdCall::get(&format!("{PATH}/{pipeline_name}/{stage_name}/history"))
        .version(3)
        .query(query)
}

pub fn cancel(
    pipeline_name: &str,
    pipeline_counter: &str,
    stage_name: &str,
    stage_counter: &str,
) -> GocdCall {
    GocdCall::post(&format!(
        "{PATH}/{pipeline_name}/{pipeline_counter}/{stage_name}/{stage_counter}/cancel"
    ))
    .version(3)
    .confirm()
}

pub fn run_failed_jobs(
    pipeline_name: &str,
    pipeline_counter: &str,
    stage_name: &str,
    stage_counter: &str,
) -> GocdCall {
    GocdCall::post(&format!(
        "{PATH}/{pipeline_name}/{pipeline_counter}/{stage_name}/{stage_counter}/run-failed-jobs"
    ))
    .version(3)
    .confirm()
}

pub fn run_selected_jobs(
    pipeline_name: &str,
    pipeline_counter: &str,
    stage_name: &str,
    stage_counter: &str,
    body: Value,
) -> GocdCall {
    GocdCall::post(&format!(
        "{PATH}/{pipeline_name}/{pipeline_counter}/{stage_name}/{stage_counter}/run-selected-jobs"
    ))
    .version(3)
    .body(body)
}
