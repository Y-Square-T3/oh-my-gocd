// Pipelines section: /go/api/pipelines/:pipeline_name/status, /pause,
// /unpause, /unlock, /schedule (API v1) and /compare/:from_counter/:to_counter
// (API v2). GoCD documents no If-Match-gated write here, so no tool takes an
// etag arg; its body-less POSTs send the documented X-GoCD-Confirm header.

use super::OmgMcp;
use crate::gocd::pipelines;
use rmcp::schemars::JsonSchema;
use rmcp::{handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub(crate) struct PipelineName {
    /// The pipeline name, e.g. "pipeline1".
    pub pipeline_name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub(crate) struct PausePipeline {
    /// The pipeline name, e.g. "pipeline1".
    pub pipeline_name: String,
    /// Optional pause request object, {"pause_cause": "<text>"}. When unset
    /// the request is sent as the docs show it, with the X-GoCD-Confirm
    /// header instead of a body.
    pub body: Option<Value>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub(crate) struct SchedulePipeline {
    /// The pipeline name, e.g. "pipeline1".
    pub pipeline_name: String,
    /// Optional scheduling options object with the documented keys
    /// `environment_variables`, `materials` and
    /// `update_materials_before_scheduling`. When unset the request is sent
    /// as the docs show it, with the X-GoCD-Confirm header instead of a body.
    pub body: Option<Value>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub(crate) struct PipelineCompare {
    /// The pipeline name, e.g. "pipeline1".
    pub pipeline_name: String,
    /// The first pipeline instance counter, e.g. "1".
    pub from_counter: String,
    /// The second pipeline instance counter, e.g. "3".
    pub to_counter: String,
}

#[tool_router(router = pipelines_tool_router, vis = "pub(crate)")]
impl OmgMcp {
    #[tool(
        description = "Read a pipeline's operation state (GET /go/api/pipelines/:pipeline_name/status, API v1). Returns {\"paused\", \"paused_cause\", \"paused_by\", \"locked\", \"schedulable\"} with all `_links` removed and `_etag` included when GoCD sent one. Docs: https://api.gocd.org/current/#get-pipeline-status"
    )]
    async fn get_pipeline_status(
        &self,
        Parameters(args): Parameters<PipelineName>,
    ) -> CallToolResult {
        self.request_shaped(pipelines::status(&args.pipeline_name))
            .await
    }

    #[tool(
        description = "Pause a pipeline (POST /go/api/pipelines/:pipeline_name/pause, API v1). `body` is the optional {\"pause_cause\": \"<text>\"} object; when omitted omg sends the documented X-GoCD-Confirm header instead. Requires operate permission on the pipeline; GoCD documents no If-Match guard here. Returns {\"message\": ...} with `_links` removed and `_etag` included when GoCD sent one. Docs: https://api.gocd.org/current/#pause-a-pipeline"
    )]
    async fn pause_pipeline(&self, Parameters(args): Parameters<PausePipeline>) -> CallToolResult {
        self.request_shaped(pipelines::pause(&args.pipeline_name, args.body))
            .await
    }

    #[tool(
        description = "Unpause a pipeline (POST /go/api/pipelines/:pipeline_name/unpause, API v1). Sent exactly as documented: no request body, with the X-GoCD-Confirm header. Requires operate permission on the pipeline; GoCD documents no If-Match guard here. Returns {\"message\": ...} with `_links` removed and `_etag` included when GoCD sent one. Docs: https://api.gocd.org/current/#unpause-a-pipeline"
    )]
    async fn unpause_pipeline(&self, Parameters(args): Parameters<PipelineName>) -> CallToolResult {
        self.request_shaped(pipelines::unpause(&args.pipeline_name))
            .await
    }

    #[tool(
        description = "Release a pipeline lock (POST /go/api/pipelines/:pipeline_name/unlock, API v1). Sent exactly as documented: no request body, with the X-GoCD-Confirm header. A lock can only be released while the pipeline is locked and has no running instance; GoCD documents no If-Match guard here. Returns {\"message\": ...} with `_links` removed and `_etag` included when GoCD sent one. Docs: https://api.gocd.org/current/#releasing-a-pipeline-lock"
    )]
    async fn unlock_pipeline(&self, Parameters(args): Parameters<PipelineName>) -> CallToolResult {
        self.request_shaped(pipelines::unlock(&args.pipeline_name))
            .await
    }

    #[tool(
        description = "Schedule a new instance of a pipeline (POST /go/api/pipelines/:pipeline_name/schedule, API v1). `body` is the optional documented options object {\"environment_variables\": [...], \"materials\": [{\"fingerprint\": ..., \"revision\": ...}], \"update_materials_before_scheduling\": ...}; when omitted omg sends the X-GoCD-Confirm header instead. Requires operate permission; GoCD documents no If-Match guard here. Returns {\"message\": ...} with `_links` removed and `_etag` included when GoCD sent one. Docs: https://api.gocd.org/current/#scheduling-pipelines"
    )]
    async fn schedule_pipeline(
        &self,
        Parameters(args): Parameters<SchedulePipeline>,
    ) -> CallToolResult {
        self.request_shaped(pipelines::schedule(&args.pipeline_name, args.body))
            .await
    }

    #[tool(
        description = "Compare the material changes between two pipeline instances (GET /go/api/pipelines/:pipeline_name/compare/:from_counter/:to_counter, API v2). Returns the comparison result object — pipeline_name, from_counter, to_counter, is_bisect and changes — with all `_links` removed and `_etag` included when GoCD sent one; the current docs define no query params for this endpoint. Docs: https://api.gocd.org/current/#compare-pipeline-instances"
    )]
    async fn compare_pipeline_instances(
        &self,
        Parameters(args): Parameters<PipelineCompare>,
    ) -> CallToolResult {
        self.request_shaped(pipelines::compare(
            &args.pipeline_name,
            &args.from_counter,
            &args.to_counter,
        ))
        .await
    }
}

#[cfg(test)]
mod tests {
    use crate::gocd::{GocdCall, GocdError};
    use crate::server::mcp::fake::{FakeGocd, first_text, service};
    use rmcp::handler::server::wrapper::Parameters;
    use serde_json::{Value, json};

    fn name_args() -> super::PipelineName {
        super::PipelineName {
            pipeline_name: "pipeline1".into(),
        }
    }

    #[tokio::test]
    async fn get_pipeline_status_reads_through_a_version_1_get_and_shapes_the_answer() {
        // Body taken verbatim from the docs' status example, plus the
        // `_links` real answers carry, so the drop is exercised.
        let mut body = json!({
            "paused": true,
            "paused_cause": "Reason for pausing this pipeline",
            "paused_by": "admin",
            "locked": false,
            "schedulable": false
        });
        body["_links"] = json!({
            "self": { "href": "http://ci.example.com/go/api/pipelines/pipeline1/status" }
        });
        let expected = json!({
            "paused": true,
            "paused_cause": "Reason for pausing this pipeline",
            "paused_by": "admin",
            "locked": false,
            "schedulable": false,
            "_etag": "\"feedbeef\""
        });
        let fake = FakeGocd::replies(body, Some("\"feedbeef\"".into()));
        let result = service(fake.clone())
            .get_pipeline_status(Parameters(name_args()))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![GocdCall::get("api/pipelines/pipeline1/status").version(1)]
        );
        assert_ne!(result.is_error, Some(true));
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(shaped, expected);
    }

    #[tokio::test]
    async fn pause_pipeline_posts_the_documented_pause_cause_body() {
        let fake = FakeGocd::replies(
            json!({ "message": "Pipeline 'pipeline1' paused successfully." }),
            None,
        );
        let result = service(fake.clone())
            .pause_pipeline(Parameters(super::PausePipeline {
                pipeline_name: "pipeline1".into(),
                body: Some(json!({ "pause_cause": "Investigating build failures" })),
            }))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![
                GocdCall::post("api/pipelines/pipeline1/pause")
                    .version(1)
                    .body(json!({ "pause_cause": "Investigating build failures" }))
            ]
        );
        assert_ne!(result.is_error, Some(true));
        assert!(first_text(&result).contains("paused successfully"));
    }

    #[tokio::test]
    async fn pause_pipeline_without_a_body_sends_the_documented_confirmation_instead() {
        let fake = FakeGocd::replies(json!({ "message": "ok" }), None);
        service(fake.clone())
            .pause_pipeline(Parameters(super::PausePipeline {
                pipeline_name: "pipeline1".into(),
                body: None,
            }))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![
                GocdCall::post("api/pipelines/pipeline1/pause")
                    .version(1)
                    .confirm()
            ]
        );
    }

    #[tokio::test]
    async fn unpause_pipeline_posts_the_documented_confirmation() {
        let fake = FakeGocd::replies(
            json!({ "message": "Pipeline 'pipeline1' unpaused successfully." }),
            None,
        );
        let result = service(fake.clone())
            .unpause_pipeline(Parameters(name_args()))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![
                GocdCall::post("api/pipelines/pipeline1/unpause")
                    .version(1)
                    .confirm()
            ]
        );
        assert_ne!(result.is_error, Some(true));
        assert!(first_text(&result).contains("unpaused successfully"));
    }

    #[tokio::test]
    async fn unlock_pipeline_posts_the_documented_confirmation() {
        let fake = FakeGocd::replies(
            json!({ "message": "Pipeline lock released for pipeline1." }),
            None,
        );
        let result = service(fake.clone())
            .unlock_pipeline(Parameters(name_args()))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![
                GocdCall::post("api/pipelines/pipeline1/unlock")
                    .version(1)
                    .confirm()
            ]
        );
        assert_ne!(result.is_error, Some(true));
        assert!(first_text(&result).contains("lock released"));
    }

    #[tokio::test]
    async fn schedule_pipeline_posts_the_documented_options_body() {
        let body = json!({
            "environment_variables": [ { "name": "USERNAME", "secure": false, "value": "bob" } ],
            "materials": [ {
                "fingerprint": "b5bb9d8014a0f9b1d61e21e796d78dccdf1352f23cd32812f4850b878ae4944c",
                "revision": "123"
            } ],
            "update_materials_before_scheduling": true
        });
        let fake = FakeGocd::replies(
            json!({ "message": "Request to schedule pipeline pipeline1 accepted" }),
            None,
        );
        let result = service(fake.clone())
            .schedule_pipeline(Parameters(super::SchedulePipeline {
                pipeline_name: "pipeline1".into(),
                body: Some(body.clone()),
            }))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![
                GocdCall::post("api/pipelines/pipeline1/schedule")
                    .version(1)
                    .body(body)
            ]
        );
        assert_ne!(result.is_error, Some(true));
        assert!(first_text(&result).contains("accepted"));
    }

    #[tokio::test]
    async fn schedule_pipeline_without_a_body_sends_the_documented_confirmation_instead() {
        let fake = FakeGocd::replies(json!({ "message": "ok" }), None);
        service(fake.clone())
            .schedule_pipeline(Parameters(super::SchedulePipeline {
                pipeline_name: "pipeline1".into(),
                body: None,
            }))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![
                GocdCall::post("api/pipelines/pipeline1/schedule")
                    .version(1)
                    .confirm()
            ]
        );
    }

    #[tokio::test]
    async fn compare_pipeline_instances_reads_through_a_version_2_get_and_shapes_the_answer() {
        // Changes trimmed from the docs' v2 comparison example; the
        // `_links` it carries exercises the recursive drop.
        let body = json!({
            "_links": {
                "self": { "href": "http://ci.example.com/go/api/pipelines/pipeline1/compare/1/3" },
                "doc": { "href": "https://api.gocd.org/current/#compare-pipeline-instances" }
            },
            "pipeline_name": "pipeline1",
            "from_counter": 1,
            "to_counter": 3,
            "is_bisect": false,
            "changes": [ {
                "material": { "type": "git", "attributes": { "branch": "master" } },
                "revision": [ { "revision_sha": "some-random-sha" } ]
            } ]
        });
        let fake = FakeGocd::replies(body, Some("\"c0ffee\"".into()));
        let result = service(fake.clone())
            .compare_pipeline_instances(Parameters(super::PipelineCompare {
                pipeline_name: "pipeline1".into(),
                from_counter: "1".into(),
                to_counter: "3".into(),
            }))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![GocdCall::get("api/pipelines/pipeline1/compare/1/3").version(2)]
        );
        assert_ne!(result.is_error, Some(true));
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(
            shaped,
            json!({
                "pipeline_name": "pipeline1",
                "from_counter": 1,
                "to_counter": 3,
                "is_bisect": false,
                "changes": [ {
                    "material": { "type": "git", "attributes": { "branch": "master" } },
                    "revision": [ { "revision_sha": "some-random-sha" } ]
                } ],
                "_etag": "\"c0ffee\""
            })
        );
    }

    #[tokio::test]
    async fn an_unauthorized_call_surfaces_the_token_hint() {
        let fake = FakeGocd::fails(GocdError::Http { status: 401 });
        let result = service(fake)
            .get_pipeline_status(Parameters(name_args()))
            .await;

        assert_eq!(result.is_error, Some(true));
        let text = first_text(&result);
        assert!(text.contains("401"), "got: {text}");
        assert!(text.contains("token"), "got: {text}");
    }

    #[test]
    fn the_tool_descriptions_cite_the_documented_endpoints() {
        let service = service(FakeGocd::replies(json!({}), None));
        let described = |name: &str| -> String {
            service
                .tool_router
                .list_all()
                .into_iter()
                .find(|t| t.name == name)
                .unwrap_or_else(|| panic!("{name} registered"))
                .description
                .map(|d| d.to_string())
                .unwrap_or_default()
        };

        let status = described("get_pipeline_status");
        assert!(status.contains("GET /go/api/pipelines/:pipeline_name/status"));
        assert!(status.contains("API v1"));
        assert!(status.contains("#get-pipeline-status"));

        let pause = described("pause_pipeline");
        assert!(pause.contains("POST /go/api/pipelines/:pipeline_name/pause"));
        assert!(pause.contains("#pause-a-pipeline"));

        let unpause = described("unpause_pipeline");
        assert!(unpause.contains("POST /go/api/pipelines/:pipeline_name/unpause"));
        assert!(unpause.contains("#unpause-a-pipeline"));

        let unlock = described("unlock_pipeline");
        assert!(unlock.contains("POST /go/api/pipelines/:pipeline_name/unlock"));
        assert!(unlock.contains("#releasing-a-pipeline-lock"));

        let schedule = described("schedule_pipeline");
        assert!(schedule.contains("POST /go/api/pipelines/:pipeline_name/schedule"));
        assert!(schedule.contains("#scheduling-pipelines"));

        let compare = described("compare_pipeline_instances");
        assert!(
            compare
                .contains("GET /go/api/pipelines/:pipeline_name/compare/:from_counter/:to_counter")
        );
        assert!(compare.contains("API v2"));
        assert!(compare.contains("#compare-pipeline-instances"));
    }

    #[test]
    fn the_section_tools_are_merged_into_the_service_router() {
        let service = service(FakeGocd::replies(json!({}), None));
        let names: Vec<String> = service
            .tool_router
            .list_all()
            .into_iter()
            .map(|t| t.name.to_string())
            .collect();
        for expected in [
            "compare_pipeline_instances",
            "get_pipeline_status",
            "pause_pipeline",
            "schedule_pipeline",
            "unlock_pipeline",
            "unpause_pipeline",
        ] {
            assert!(names.contains(&expected.to_string()), "{expected} missing");
        }
    }
}
