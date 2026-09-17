// Jobs section of the GoCD API docs: thin GocdCall constructors for
// /go/api/jobs, API v1. Read-only — GoCD documents no body and no If-Match
// for either endpoint.

use super::GocdCall;

const PATH: &str = "api/jobs";

pub fn read(
    pipeline_name: &str,
    pipeline_counter: &str,
    stage_name: &str,
    stage_counter: &str,
    job_name: &str,
) -> GocdCall {
    GocdCall::get(&format!(
        "{PATH}/{pipeline_name}/{pipeline_counter}/{stage_name}/{stage_counter}/{job_name}"
    ))
    .version(1)
}

pub fn history(
    pipeline_name: &str,
    stage_name: &str,
    job_name: &str,
    query: Vec<(String, String)>,
) -> GocdCall {
    GocdCall::get(&format!(
        "{PATH}/{pipeline_name}/{stage_name}/{job_name}/history"
    ))
    .version(1)
    .query(query)
}
