// Stage Instances section: /go/api/stages/:pipeline_name/:pipeline_counter/
// :stage_name/:stage_counter and .../history, plus the cancel, run-failed-jobs
// and run-selected-jobs writes, API v3. GoCD documents no If-Match-gated
// write here, so no tool takes an etag arg.

use super::OmgMcp;
use crate::gocd::stage_instances;
use rmcp::schemars::JsonSchema;
use rmcp::{handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub(crate) struct StageInstancePath {
    /// The pipeline name, e.g. "myPipeline".
    pub pipeline_name: String,
    /// The pipeline counter, e.g. "1".
    pub pipeline_counter: String,
    /// The stage name, e.g. "myStage".
    pub stage_name: String,
    /// The stage counter, e.g. "1".
    pub stage_counter: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub(crate) struct StageHistory {
    /// The pipeline name, e.g. "mypipeline".
    pub pipeline_name: String,
    /// The stage name, e.g. "defaultStage".
    pub stage_name: String,
    /// The number of records per page, between 10 and 100 (defaults to 10).
    pub page_size: Option<i64>,
    /// The cursor value for fetching the next set of records, taken from the
    /// history's "next" link.
    pub after: Option<String>,
    /// The cursor value for fetching the previous set of records, taken from
    /// the history's "previous" link.
    pub before: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub(crate) struct RunSelectedStageJobs {
    /// The pipeline name, e.g. "myPipeline".
    pub pipeline_name: String,
    /// The pipeline counter, e.g. "1".
    pub pipeline_counter: String,
    /// The stage name, e.g. "myStage".
    pub stage_name: String,
    /// The stage counter, e.g. "1".
    pub stage_counter: String,
    /// The jobs request object: {"jobs": ["<job_name>", ...]} — the `jobs`
    /// array is required.
    pub body: Value,
}

#[tool_router(router = stage_instances_tool_router, vis = "pub(crate)")]
impl OmgMcp {
    #[tool(
        description = "Read one stage instance (GET /go/api/stages/:pipeline_name/:pipeline_counter/:stage_name/:stage_counter, API v3). Returns the stage instance object — name, counter, result, approval fields, pipeline coordinates and its jobs with their state transitions — with all `_links` removed and `_etag` included when GoCD sent one. Docs: https://api.gocd.org/current/#get-stage-instance"
    )]
    async fn get_stage_instance(
        &self,
        Parameters(args): Parameters<StageInstancePath>,
    ) -> CallToolResult {
        self.request_shaped(stage_instances::read(
            &args.pipeline_name,
            &args.pipeline_counter,
            &args.stage_name,
            &args.stage_counter,
        ))
        .await
    }

    #[tool(
        description = "List a stage's past instances (GET /go/api/stages/:pipeline_name/:stage_name/history, API v3). Supports cursor based pagination: optional query args page_size (10-100, default 10), after and before take their values from the history's own next/previous links; unset args are omitted. Returns the `stages` array of stage instance objects with all `_links` removed and `_etag` included when GoCD sent one. Docs: https://api.gocd.org/current/#get-stage-history"
    )]
    async fn get_stage_history(
        &self,
        Parameters(args): Parameters<StageHistory>,
    ) -> CallToolResult {
        let mut query = Vec::new();
        if let Some(page_size) = args.page_size {
            query.push(("page_size".to_string(), page_size.to_string()));
        }
        if let Some(after) = args.after {
            query.push(("after".to_string(), after));
        }
        if let Some(before) = args.before {
            query.push(("before".to_string(), before));
        }
        self.request_shaped(stage_instances::history(
            &args.pipeline_name,
            &args.stage_name,
            query,
        ))
        .await
    }

    #[tool(
        description = "Cancel an active stage instance (POST /go/api/stages/:pipeline_name/:pipeline_counter/:stage_name/:stage_counter/cancel, API v3). Sent exactly as documented: no request body, with the X-GoCD-Confirm header; GoCD documents no If-Match guard here. Returns {\"message\": ...} with `_links` removed and `_etag` included when GoCD sent one. Docs: https://api.gocd.org/current/#cancel-stage"
    )]
    async fn cancel_stage_instance(
        &self,
        Parameters(args): Parameters<StageInstancePath>,
    ) -> CallToolResult {
        self.request_shaped(stage_instances::cancel(
            &args.pipeline_name,
            &args.pipeline_counter,
            &args.stage_name,
            &args.stage_counter,
        ))
        .await
    }

    #[tool(
        description = "Rerun the failed jobs of a completed stage instance (POST /go/api/stages/:pipeline_name/:pipeline_counter/:stage_name/:stage_counter/run-failed-jobs, API v3). Sent exactly as documented: no request body, with the X-GoCD-Confirm header; GoCD answers 202 and documents no If-Match guard here. Returns {\"message\": ...} with `_links` removed and `_etag` included when GoCD sent one. Docs: https://api.gocd.org/current/#run-failed-jobs"
    )]
    async fn run_failed_stage_jobs(
        &self,
        Parameters(args): Parameters<StageInstancePath>,
    ) -> CallToolResult {
        self.request_shaped(stage_instances::run_failed_jobs(
            &args.pipeline_name,
            &args.pipeline_counter,
            &args.stage_name,
            &args.stage_counter,
        ))
        .await
    }

    #[tool(
        description = "Rerun the named jobs of a completed stage instance (POST /go/api/stages/:pipeline_name/:pipeline_counter/:stage_name/:stage_counter/run-selected-jobs, API v3). `body` is {\"jobs\": [\"<job_name>\", ...]} — the `jobs` array is required; GoCD answers 202 and documents no If-Match guard here. Returns {\"message\": ...} with `_links` removed and `_etag` included when GoCD sent one. Docs: https://api.gocd.org/current/#run-selected-jobs"
    )]
    async fn run_selected_stage_jobs(
        &self,
        Parameters(args): Parameters<RunSelectedStageJobs>,
    ) -> CallToolResult {
        self.request_shaped(stage_instances::run_selected_jobs(
            &args.pipeline_name,
            &args.pipeline_counter,
            &args.stage_name,
            &args.stage_counter,
            args.body,
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

    fn instance_args() -> super::StageInstancePath {
        super::StageInstancePath {
            pipeline_name: "myPipeline".into(),
            pipeline_counter: "42".into(),
            stage_name: "myStages".into(),
            stage_counter: "13".into(),
        }
    }

    fn history_args() -> super::StageHistory {
        super::StageHistory {
            pipeline_name: "mypipeline".into(),
            stage_name: "defaultStage".into(),
            page_size: None,
            after: None,
            before: None,
        }
    }

    // Body taken from the GoCD API docs' stage instance example.
    fn docs_instance_body() -> Value {
        json!({
            "name": "default_stage",
            "counter": 1,
            "approval_type": "success",
            "approved_by": "changes",
            "scheduled_at": 1578891141997i64,
            "last_transitioned_time": 1436509642631i64,
            "result": "Unknown",
            "rerun_of_counter": null,
            "fetch_materials": true,
            "clean_working_directory": false,
            "artifacts_deleted": false,
            "pipeline_name": "default",
            "pipeline_counter": 1,
            "jobs": [ {
                "name": "default_job",
                "state": "Scheduled",
                "result": "Unknown",
                "scheduled_date": 1578891141997i64,
                "rerun": false,
                "original_job_id": null,
                "agent_uuid": null,
                "pipeline_name": null,
                "pipeline_counter": null,
                "stage_name": null,
                "stage_counter": null,
                "job_state_transitions": [
                    { "state": "Scheduled", "state_change_time": 1578891141997i64 },
                    { "state": "Completed", "state_change_time": 1436509642631i64 }
                ]
            } ]
        })
    }

    // Stages array from the docs' history example; the top-level `_links`
    // carry the cursor pagination data the docs' text describes.
    fn docs_history_body() -> Value {
        json!({
            "_links": {
                "next": { "href": "http://ci.example.com/go/api/stages/mypipeline/defaultStage/history?after=38" },
                "previous": { "href": "http://ci.example.com/go/api/stages/mypipeline/defaultStage/history?before=4" }
            },
            "stages": [
                {
                    "name": "defaultStage",
                    "approved_by": "admin",
                    "cancelled_by": "GoCD",
                    "jobs": [ {
                        "name": "defaultJob",
                        "result": "Cancelled",
                        "state": "Completed",
                        "scheduled_date": 1436509881002i64
                    } ],
                    "pipeline_counter": 3,
                    "pipeline_name": "mypipeline",
                    "result": "Cancelled",
                    "approval_type": "success",
                    "counter": "1",
                    "rerun_of_counter": null
                },
                {
                    "name": "defaultStage",
                    "approved_by": "changes",
                    "jobs": [ {
                        "name": "defaultJob",
                        "result": "Passed",
                        "state": "Completed",
                        "scheduled_date": 1436509518752i64
                    } ],
                    "pipeline_counter": 1,
                    "pipeline_name": "mypipeline",
                    "result": "Passed",
                    "approval_type": "success",
                    "counter": "1",
                    "rerun_of_counter": null
                }
            ]
        })
    }

    #[tokio::test]
    async fn get_stage_instance_sends_a_version_3_get_to_the_documented_path() {
        let fake = FakeGocd::replies(docs_instance_body(), None);
        let result = service(fake.clone())
            .get_stage_instance(Parameters(instance_args()))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![GocdCall::get("api/stages/myPipeline/42/myStages/13").version(3)]
        );
        assert_ne!(result.is_error, Some(true));
    }

    #[tokio::test]
    async fn get_stage_instance_drops_links_and_injects_the_etag_only_when_go_cd_sent_one() {
        let mut body = docs_instance_body();
        body["_links"] = json!({
            "self": { "href": "http://ci.example.com/go/api/stages/myPipeline/42/myStages/13" }
        });
        let fake = FakeGocd::replies(body.clone(), Some("\"feedbeef\"".into()));
        let result = service(fake)
            .get_stage_instance(Parameters(instance_args()))
            .await;

        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        body.as_object_mut().unwrap().remove("_links");
        body.as_object_mut()
            .unwrap()
            .insert("_etag".to_string(), json!("\"feedbeef\""));
        assert_eq!(shaped, body);

        let fake = FakeGocd::replies(docs_instance_body(), None);
        let result = service(fake)
            .get_stage_instance(Parameters(instance_args()))
            .await;
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert!(shaped.get("_etag").is_none());
    }

    #[tokio::test]
    async fn get_stage_history_sends_a_version_3_get_to_the_documented_path() {
        let fake = FakeGocd::replies(docs_history_body(), None);
        let result = service(fake.clone())
            .get_stage_history(Parameters(history_args()))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![GocdCall::get("api/stages/mypipeline/defaultStage/history").version(3)]
        );
        assert_ne!(result.is_error, Some(true));
    }

    #[tokio::test]
    async fn get_stage_history_drops_every_links_object_recursively() {
        // The docs' example carries a top-level `_links` of cursor pointers;
        // a nested one is bolted on the way real answers do, so the drop is
        // exercised recursively.
        let mut body = docs_history_body();
        body["stages"][0]["_links"] = json!({
            "self": { "href": "http://ci.example.com/go/api/stages/mypipeline/3/defaultStage/1" }
        });
        let mut expected = body.clone();
        expected.as_object_mut().unwrap().remove("_links");
        expected["stages"][0]
            .as_object_mut()
            .unwrap()
            .remove("_links");
        expected
            .as_object_mut()
            .unwrap()
            .insert("_etag".to_string(), json!("\"c0ffee\""));

        let fake = FakeGocd::replies(body, Some("\"c0ffee\"".into()));
        let result = service(fake)
            .get_stage_history(Parameters(history_args()))
            .await;

        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(shaped, expected);
    }

    #[tokio::test]
    async fn get_stage_history_omits_unset_optional_params() {
        let fake = FakeGocd::replies(docs_history_body(), None);
        service(fake.clone())
            .get_stage_history(Parameters(history_args()))
            .await;

        let call = &fake.recorded()[0];
        assert!(call.query.is_empty());
    }

    #[tokio::test]
    async fn get_stage_history_sends_the_documented_optional_params_when_set() {
        let fake = FakeGocd::replies(docs_history_body(), None);
        service(fake.clone())
            .get_stage_history(Parameters(super::StageHistory {
                page_size: Some(20),
                after: Some("38".into()),
                before: Some("4".into()),
                ..history_args()
            }))
            .await;

        let call = &fake.recorded()[0];
        assert_eq!(
            call.query,
            vec![
                ("page_size".to_string(), "20".to_string()),
                ("after".to_string(), "38".to_string()),
                ("before".to_string(), "4".to_string())
            ]
        );
    }

    #[tokio::test]
    async fn cancel_stage_instance_posts_a_confirmed_bodyless_request_and_returns_the_message() {
        let fake = FakeGocd::replies(json!({ "message": "Stage cancelled successfully." }), None);
        let result = service(fake.clone())
            .cancel_stage_instance(Parameters(instance_args()))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![
                GocdCall::post("api/stages/myPipeline/42/myStages/13/cancel")
                    .version(3)
                    .confirm()
            ]
        );
        assert_ne!(result.is_error, Some(true));
        assert!(first_text(&result).contains("Stage cancelled successfully"));
    }

    #[tokio::test]
    async fn run_failed_stage_jobs_posts_a_confirmed_bodyless_request_and_returns_the_message() {
        let fake = FakeGocd::replies(json!({ "message": "Request to rerun jobs accepted" }), None);
        let result = service(fake.clone())
            .run_failed_stage_jobs(Parameters(instance_args()))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![
                GocdCall::post("api/stages/myPipeline/42/myStages/13/run-failed-jobs")
                    .version(3)
                    .confirm()
            ]
        );
        assert_ne!(result.is_error, Some(true));
        assert!(first_text(&result).contains("Request to rerun jobs accepted"));
    }

    #[tokio::test]
    async fn run_selected_stage_jobs_posts_the_documented_jobs_body() {
        let fake = FakeGocd::replies(json!({ "message": "Request to rerun jobs accepted" }), None);
        let result = service(fake.clone())
            .run_selected_stage_jobs(Parameters(super::RunSelectedStageJobs {
                body: json!({ "jobs": ["job1", "job2"] }),
                ..instance_args_as_selected()
            }))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![
                GocdCall::post("api/stages/myPipeline/42/myStages/13/run-selected-jobs")
                    .version(3)
                    .body(json!({ "jobs": ["job1", "job2"] }))
            ]
        );
        assert_ne!(result.is_error, Some(true));
        assert!(first_text(&result).contains("Request to rerun jobs accepted"));
    }

    fn instance_args_as_selected() -> super::RunSelectedStageJobs {
        super::RunSelectedStageJobs {
            pipeline_name: "myPipeline".into(),
            pipeline_counter: "42".into(),
            stage_name: "myStages".into(),
            stage_counter: "13".into(),
            body: Value::Null,
        }
    }

    #[tokio::test]
    async fn an_unauthorized_read_surfaces_the_token_hint() {
        let fake = FakeGocd::fails(GocdError::Http { status: 401 });
        let result = service(fake)
            .get_stage_instance(Parameters(instance_args()))
            .await;

        assert_eq!(result.is_error, Some(true));
        let text = first_text(&result);
        assert!(text.contains("401"), "got: {text}");
        assert!(text.contains("token"), "got: {text}");
    }

    #[tokio::test]
    async fn an_unauthorized_write_surfaces_the_token_hint() {
        let fake = FakeGocd::fails(GocdError::Http { status: 401 });
        let result = service(fake)
            .cancel_stage_instance(Parameters(instance_args()))
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

        let instance = described("get_stage_instance");
        assert!(instance.contains(
            "GET /go/api/stages/:pipeline_name/:pipeline_counter/:stage_name/:stage_counter"
        ));
        assert!(instance.contains("API v3"));
        assert!(instance.contains("#get-stage-instance"));

        let history = described("get_stage_history");
        assert!(history.contains("GET /go/api/stages/:pipeline_name/:stage_name/history"));
        assert!(history.contains("#get-stage-history"));

        let cancel = described("cancel_stage_instance");
        assert!(cancel.contains(
            "POST /go/api/stages/:pipeline_name/:pipeline_counter/:stage_name/:stage_counter/cancel"
        ));
        assert!(cancel.contains("#cancel-stage"));

        let failed = described("run_failed_stage_jobs");
        assert!(failed.contains(
            "POST /go/api/stages/:pipeline_name/:pipeline_counter/:stage_name/:stage_counter/run-failed-jobs"
        ));
        assert!(failed.contains("#run-failed-jobs"));

        let selected = described("run_selected_stage_jobs");
        assert!(selected.contains(
            "POST /go/api/stages/:pipeline_name/:pipeline_counter/:stage_name/:stage_counter/run-selected-jobs"
        ));
        assert!(selected.contains("#run-selected-jobs"));
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
            "cancel_stage_instance",
            "get_stage_history",
            "get_stage_instance",
            "run_failed_stage_jobs",
            "run_selected_stage_jobs",
        ] {
            assert!(names.contains(&expected.to_string()), "{expected} missing");
        }
    }
}
