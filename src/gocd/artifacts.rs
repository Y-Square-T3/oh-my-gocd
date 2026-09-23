// Artifacts section of the GoCD API docs: the JSON job-artifact listing and
// the single-file download at /go/files, API v1 — the listing answers plain
// JSON, the file GET flags its call `.raw()` so the transport answers with
// the (already lossy-decoded) response text instead of trying to parse it.
// The section's directory-zip download, create, create-multiple and append
// operations are out by recorded decision. The path carries its own `files/`
// prefix (the route lives outside /go/api) and the job name takes the `.json`
// suffix only on the listing. The pinned v1 Accept header is deliberate: a
// live probe of build.gocd.org showed the `/go/files` routes answer
// identically with and without it.

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

pub fn file(
    pipeline_name: &str,
    pipeline_counter: &str,
    stage_name: &str,
    stage_counter: &str,
    job_name: &str,
    path_to_file: &str,
) -> GocdCall {
    GocdCall::get(&format!(
        "{PATH}/{pipeline_name}/{pipeline_counter}/{stage_name}/{stage_counter}/{job_name}/{}",
        path_to_file.trim_start_matches('/')
    ))
    .version(1)
    .raw()
}
