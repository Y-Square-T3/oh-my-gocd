// Artifacts section of the GoCD API docs: the JSON job-artifact listing at
// /go/files, API v1 — in scope; the section's five download/upload/append
// operations are out by recorded decision. The path carries its own `files/`
// prefix (the route lives outside /go/api) and the job name takes the `.json`
// suffix. The pinned v1 Accept header is deliberate: a live probe of
// build.gocd.org showed the `.json` route answers identically with and
// without it.

use super::GocdCall;

const PATH: &str = "files";

pub fn listing(
    pipeline_name: &str,
    pipeline_counter: &str,
    stage_name: &str,
    stage_counter: &str,
    job_name: &str,
) -> GocdCall {
    GocdCall::get(&format!(
        "{PATH}/{pipeline_name}/{pipeline_counter}/{stage_name}/{stage_counter}/{job_name}.json"
    ))
    .version(1)
}
