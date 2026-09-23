// Derived conveniences: higher-level tools composed over the same GocdApi
// seam the low-level section tools use. Each answers a question agents ask
// ("what did this job print?", "what is this pipeline doing right now?") in
// one call, replacing a multi-tool walk of the raw GoCD endpoints. The
// family is recorded as a decision in docs/adr/0002.

use super::artifacts::JobArtifactsPath;
use super::pipelines::PipelineName;
use super::{OmgMcp, tool_error};
use crate::gocd::{GocdError, artifacts, pipeline_instances, pipelines};
use rmcp::model::ContentBlock;
use rmcp::{handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router};
use serde_json::Value;

/// One of a job's invariant artifact files: its path plus the tag the
/// missing-file notice carries. Path and tag travel together by design —
/// they are named side by side so a path move cannot strand the tag.
struct JobFile {
    path: &'static str,
    notice: &'static str,
}

/// The console log the UI's Job Console tab renders.
const CONSOLE_LOG: JobFile = JobFile {
    path: "cruise-output/console.log",
    notice: "no console log",
};

/// The review report this server's code-review pipelines publish.
const CODE_REVIEW: JobFile = JobFile {
    path: "code-review/code_review/reports/index.html",
    notice: "no code review",
};

#[tool_router(router = derived_tool_router, vis = "pub(crate)")]
impl OmgMcp {
    #[tool(
        description = "Read a job's console log as plain text (GET /go/files/:pipeline_name/:pipeline_counter/:stage_name/:stage_counter/:job_name/cruise-output/console.log, API v1). Convenience derived from get_job_artifact_file with the path fixed; take the coordinates from get_pipeline_latest_status or the pipeline/stage tools. Docs: https://api.gocd.org/current/#get-artifact-file"
    )]
    async fn get_job_console_log(
        &self,
        Parameters(args): Parameters<JobArtifactsPath>,
    ) -> CallToolResult {
        self.fetch_job_file(&args, &CONSOLE_LOG).await
    }

    #[tool(
        description = "Read a job's code-review report as plain text (GET /go/files/:pipeline_name/:pipeline_counter/:stage_name/:stage_counter/:job_name/code-review/code_review/reports/index.html, API v1). Convenience derived from get_job_artifact_file for the fixed report path these pipelines publish; when the job ran no review, a `[no code review] ...` notice, not an error. Docs: https://api.gocd.org/current/#get-artifact-file"
    )]
    async fn get_job_code_review(
        &self,
        Parameters(args): Parameters<JobArtifactsPath>,
    ) -> CallToolResult {
        self.fetch_job_file(&args, &CODE_REVIEW).await
    }

    #[tool(
        description = "Answer 'what is this pipeline doing right now' in one call (GET /go/api/pipelines/:pipeline_name/history + GET /go/api/pipelines/:pipeline_name/status, API v1). Convenience derived from get_pipeline_history and get_pipeline_status: merges the latest instance — counter, label, scheduled_date, build_cause and its stages[] with jobs[] (the coordinates the get_job_* tools take) — with the operational flags {paused, paused_cause, paused_by, locked, schedulable}. `_links` removed, no `_etag` (two sources). Docs: https://api.gocd.org/current/#get-pipeline-history"
    )]
    async fn get_pipeline_latest_status(
        &self,
        Parameters(args): Parameters<PipelineName>,
    ) -> CallToolResult {
        let mut history = match self
            .api
            .request(pipeline_instances::history(&args.pipeline_name, vec![]))
            .await
        {
            Ok(reply) => reply.shaped(),
            Err(err) => return tool_error(&err),
        };
        let unexpected_history = || {
            CallToolResult::error(vec![ContentBlock::text(format!(
                "`{}`'s history answer carried no `pipelines` list — cannot tell what it is doing",
                args.pipeline_name
            ))])
        };
        let Some(instances) = history.get("pipelines").and_then(Value::as_array) else {
            return unexpected_history();
        };
        if instances.is_empty() {
            return CallToolResult::error(vec![ContentBlock::text(format!(
                "`{}` has never run — there is no pipeline instance to report",
                args.pipeline_name
            ))]);
        }
        let status = match self
            .api
            .request(pipelines::status(&args.pipeline_name))
            .await
        {
            Ok(reply) => reply.shaped(),
            Err(err) => return tool_error(&err),
        };
        let Some(Value::Object(entry)) = history
            .get_mut("pipelines")
            .and_then(Value::as_array_mut)
            .and_then(|list| list.first_mut())
        else {
            return unexpected_history();
        };
        if let Value::Object(fields) = status {
            for (key, value) in fields {
                if key != "_etag" {
                    entry.insert(key, value);
                }
            }
        }
        let merged = Value::Object(std::mem::take(entry));
        CallToolResult::success(vec![ContentBlock::text(merged.to_string())])
    }
}

impl OmgMcp {
    /// One fixed artifact file of a job, answered as the pure text of its
    /// bytes: no metadata line, a GoCD 404 becomes the successful
    /// `missing_notice`, every other failure stays a tool error.
    async fn fetch_job_file(&self, args: &JobArtifactsPath, file: &JobFile) -> CallToolResult {
        let call = artifacts::file(
            &args.pipeline_name,
            &args.pipeline_counter,
            &args.stage_name,
            &args.stage_counter,
            &args.job_name,
            file.path,
        );
        match self.api.request(call).await {
            Ok(reply) => CallToolResult::success(vec![ContentBlock::text(
                reply.body.as_str().unwrap_or_default().to_string(),
            )]),
            Err(GocdError::Http { status: 404 }) => {
                CallToolResult::success(vec![ContentBlock::text(missing_notice(file, args))])
            }
            Err(err) => tool_error(&err),
        }
    }
}

/// The successful answer for a file GoCD does not have: a one-line notice
/// naming the tag, the path tried and the job coordinates. Absence of a
/// derived tool's fixed file is meaningful data, not a failed request.
fn missing_notice(file: &JobFile, args: &JobArtifactsPath) -> String {
    format!(
        "[{}] {} was not published by {}/{}/{}/{}/{}",
        file.notice,
        file.path,
        args.pipeline_name,
        args.pipeline_counter,
        args.stage_name,
        args.stage_counter,
        args.job_name
    )
}

#[cfg(test)]
mod tests {
    use crate::config::Mode;
    use crate::gocd::{GocdCall, GocdError};
    use crate::server::mcp::fake::{FakeGocd, first_text, service, service_in};
    use rmcp::handler::server::wrapper::Parameters;
    use serde_json::{Value, json};

    fn coords() -> super::JobArtifactsPath {
        super::JobArtifactsPath {
            pipeline_name: "pipeline1".into(),
            pipeline_counter: "1".into(),
            stage_name: "defaultStage".into(),
            stage_counter: "1".into(),
            job_name: "defaultJob".into(),
        }
    }

    #[tokio::test]
    async fn get_job_console_log_sends_a_raw_version_1_get_to_the_console_log_path() {
        let fake = FakeGocd::raw_replies("", None);
        service(fake.clone())
            .get_job_console_log(Parameters(coords()))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![
                GocdCall::get(
                    "files/pipeline1/1/defaultStage/1/defaultJob/cruise-output/console.log"
                )
                .version(1)
                .raw()
            ]
        );
    }

    #[tokio::test]
    async fn get_job_console_log_answers_the_log_as_pure_text_with_no_header() {
        let fake = FakeGocd::raw_replies("Building pipeline1\ndone\n", None);
        let result = service(fake)
            .get_job_console_log(Parameters(coords()))
            .await;

        assert_eq!(result.is_error, Some(false));
        assert_eq!(first_text(&result), "Building pipeline1\ndone\n");
    }

    #[tokio::test]
    async fn an_absent_console_log_answers_a_plain_text_notice_not_an_error() {
        let fake = FakeGocd::fails(GocdError::Http { status: 404 });
        let result = service(fake)
            .get_job_console_log(Parameters(coords()))
            .await;

        assert_eq!(result.is_error, Some(false));
        assert_eq!(
            first_text(&result),
            "[no console log] cruise-output/console.log was not published by \
             pipeline1/1/defaultStage/1/defaultJob"
        );
    }

    #[tokio::test]
    async fn an_unauthorized_console_log_fetch_still_answers_a_tool_error() {
        let fake = FakeGocd::fails(GocdError::Http { status: 401 });
        let result = service(fake)
            .get_job_console_log(Parameters(coords()))
            .await;

        assert_eq!(result.is_error, Some(true));
        let text = first_text(&result);
        assert!(text.contains("401"), "got: {text}");
        assert!(text.contains("token"), "got: {text}");
    }

    #[tokio::test]
    async fn get_job_code_review_sends_a_raw_version_1_get_to_the_review_report_path() {
        let fake = FakeGocd::raw_replies("", None);
        service(fake.clone())
            .get_job_code_review(Parameters(coords()))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![GocdCall::get(
                "files/pipeline1/1/defaultStage/1/defaultJob/code-review/code_review/reports/index.html"
            )
            .version(1)
            .raw()]
        );
    }

    #[tokio::test]
    async fn get_job_code_review_answers_the_report_verbatim_with_no_header() {
        let fake = FakeGocd::raw_replies("<html>review</html>", None);
        let result = service(fake)
            .get_job_code_review(Parameters(coords()))
            .await;

        assert_eq!(result.is_error, Some(false));
        assert_eq!(first_text(&result), "<html>review</html>");
    }

    #[tokio::test]
    async fn an_absent_review_report_answers_a_plain_text_notice_not_an_error() {
        let fake = FakeGocd::fails(GocdError::Http { status: 404 });
        let result = service(fake)
            .get_job_code_review(Parameters(coords()))
            .await;

        assert_eq!(result.is_error, Some(false));
        assert_eq!(
            first_text(&result),
            "[no code review] code-review/code_review/reports/index.html was not published by \
             pipeline1/1/defaultStage/1/defaultJob"
        );
    }

    // Fixtures from the GoCD docs' pipeline-history and pipeline-status
    // examples, trimmed to the fields the merge carries.
    fn docs_history_body() -> Value {
        json!({
            "_links": { "next": { "href": "http://ci.example.com/go/api/pipelines/pipeline1/history?after=26" } },
            "pipelines": [ {
                "name": "pipeline1",
                "counter": 27,
                "label": "27",
                "natural_order": 27.0,
                "can_run": true,
                "preparing_to_schedule": false,
                "comment": null,
                "scheduled_date": 1436519914578i64,
                "build_cause": {
                    "trigger_forced": false,
                    "trigger_message": "modified by user <user@users.noreply.github.com>"
                },
                "stages": [ {
                    "name": "compile_test",
                    "counter": "15",
                    "status": "Passed",
                    "result": "Passed",
                    "scheduled": true,
                    "rerun_of_counter": null,
                    "jobs": [ {
                        "name": "defaultJob",
                        "scheduled_date": 1436582744378i64,
                        "state": "Completed",
                        "result": "Passed"
                    } ]
                } ]
            } ]
        })
    }

    fn docs_status_body() -> Value {
        json!({
            "_links": { "doc": { "href": "https://api.gocd.org/current/#get-pipeline-status" } },
            "paused": false,
            "paused_cause": null,
            "paused_by": null,
            "locked": false,
            "schedulable": true
        })
    }

    fn status_args() -> super::PipelineName {
        super::PipelineName {
            pipeline_name: "pipeline1".into(),
        }
    }

    #[tokio::test]
    async fn get_pipeline_latest_status_chains_history_then_status_and_returns_the_flat_merge() {
        let fake = FakeGocd::sequence(vec![
            (docs_history_body(), Some("\"c0ffee\"".into())),
            (docs_status_body(), Some("\"beef\"".into())),
        ]);
        let result = service(fake.clone())
            .get_pipeline_latest_status(Parameters(status_args()))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![
                GocdCall::get("api/pipelines/pipeline1/history").version(1),
                GocdCall::get("api/pipelines/pipeline1/status").version(1)
            ]
        );
        assert_eq!(result.is_error, Some(false));
        let merged: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(
            merged,
            json!({
                "name": "pipeline1",
                "counter": 27,
                "label": "27",
                "natural_order": 27.0,
                "can_run": true,
                "preparing_to_schedule": false,
                "comment": null,
                "scheduled_date": 1436519914578i64,
                "build_cause": {
                    "trigger_forced": false,
                    "trigger_message": "modified by user <user@users.noreply.github.com>"
                },
                "stages": [ {
                    "name": "compile_test",
                    "counter": "15",
                    "status": "Passed",
                    "result": "Passed",
                    "scheduled": true,
                    "rerun_of_counter": null,
                    "jobs": [ {
                        "name": "defaultJob",
                        "scheduled_date": 1436582744378i64,
                        "state": "Completed",
                        "result": "Passed"
                    } ]
                } ],
                "paused": false,
                "paused_cause": null,
                "paused_by": null,
                "locked": false,
                "schedulable": true
            })
        );
    }

    #[tokio::test]
    async fn a_pipeline_that_has_never_run_answers_a_tool_error_before_the_status_call() {
        let fake = FakeGocd::sequence(vec![(json!({ "pipelines": [] }), None)]);
        let result = service(fake.clone())
            .get_pipeline_latest_status(Parameters(status_args()))
            .await;

        assert_eq!(result.is_error, Some(true));
        assert_eq!(
            fake.recorded(),
            vec![GocdCall::get("api/pipelines/pipeline1/history").version(1)]
        );
        let text = first_text(&result);
        assert!(text.contains("pipeline1"), "got: {text}");
        assert!(text.contains("never run"), "got: {text}");
    }

    #[tokio::test]
    async fn a_failing_status_call_answers_the_shared_tool_error() {
        let fake = FakeGocd::sequence_then_fails(
            vec![(docs_history_body(), None)],
            GocdError::Http { status: 403 },
        );
        let result = service(fake.clone())
            .get_pipeline_latest_status(Parameters(status_args()))
            .await;

        assert_eq!(result.is_error, Some(true));
        assert_eq!(
            fake.recorded(),
            vec![
                GocdCall::get("api/pipelines/pipeline1/history").version(1),
                GocdCall::get("api/pipelines/pipeline1/status").version(1)
            ]
        );
        let text = first_text(&result);
        assert!(text.contains("403"), "got: {text}");
    }

    #[tokio::test]
    async fn a_failing_history_call_short_circuits_before_the_status_call() {
        let fake = FakeGocd::fails(GocdError::Http { status: 404 });
        let result = service(fake.clone())
            .get_pipeline_latest_status(Parameters(status_args()))
            .await;

        assert_eq!(result.is_error, Some(true));
        assert_eq!(
            fake.recorded(),
            vec![GocdCall::get("api/pipelines/pipeline1/history").version(1)]
        );
        let text = first_text(&result);
        assert!(text.contains("404"), "got: {text}");
    }

    #[test]
    fn view_mode_exposes_every_derived_tool() {
        let names = service_in(FakeGocd::replies(json!({}), None), Mode::View).tool_names();
        for derived in [
            "get_job_console_log",
            "get_job_code_review",
            "get_pipeline_latest_status",
        ] {
            assert!(names.contains(&derived.to_string()), "missing {derived}");
        }
    }

    fn description(name: &str) -> String {
        let service = service(FakeGocd::replies(json!({}), None));
        service
            .tool_router
            .list_all()
            .into_iter()
            .find(|tool| tool.name == name)
            .unwrap_or_else(|| panic!("{name} registered"))
            .description
            .map(|d| d.to_string())
            .unwrap_or_default()
    }

    #[test]
    fn every_derived_description_names_its_underlying_endpoints_and_sources() {
        let console = description("get_job_console_log");
        assert!(
            console.contains("cruise-output/console.log"),
            "got: {console}"
        );
        assert!(console.contains("API v1"), "got: {console}");

        let review = description("get_job_code_review");
        assert!(
            review.contains("code-review/code_review/reports/index.html"),
            "got: {review}"
        );
        assert!(review.contains("API v1"), "got: {review}");

        let latest = description("get_pipeline_latest_status");
        assert!(latest.contains("API v1"), "got: {latest}");
        assert!(
            latest.contains("get_pipeline_history") && latest.contains("get_pipeline_status"),
            "got: {latest}"
        );
    }

    #[tokio::test]
    async fn a_history_answer_without_a_pipelines_list_is_not_reported_as_never_run() {
        let fake = FakeGocd::replies(json!({ "unexpected": true }), None);
        let result = service(fake)
            .get_pipeline_latest_status(Parameters(status_args()))
            .await;

        assert_eq!(result.is_error, Some(true));
        let text = first_text(&result);
        assert!(text.contains("pipelines"), "got: {text}");
        assert!(!text.contains("never run"), "got: {text}");
    }
}
