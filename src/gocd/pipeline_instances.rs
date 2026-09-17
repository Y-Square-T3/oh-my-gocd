// Pipeline Instances section of the GoCD API docs: thin GocdCall
// constructors for /go/api/pipelines, API v1. GoCD documents no If-Match
// for any of the three endpoints, so no constructor takes an etag.

use super::GocdCall;
use serde_json::Value;

const PATH: &str = "api/pipelines";

pub fn read(pipeline_name: &str, pipeline_counter: &str) -> GocdCall {
    GocdCall::get(&format!("{PATH}/{pipeline_name}/{pipeline_counter}")).version(1)
}

pub fn history(pipeline_name: &str, query: Vec<(String, String)>) -> GocdCall {
    GocdCall::get(&format!("{PATH}/{pipeline_name}/history"))
        .version(1)
        .query(query)
}

pub fn comment(pipeline_name: &str, pipeline_counter: &str, body: Value) -> GocdCall {
    GocdCall::post(&format!(
        "{PATH}/{pipeline_name}/{pipeline_counter}/comment"
    ))
    .version(1)
    .body(body)
}
