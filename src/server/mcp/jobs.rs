// Jobs section: /go/api/jobs, API v1. Read-only — GoCD documents no
// If-Match-gated write and no ETag for either endpoint.

use super::OmgMcp;
use crate::gocd::jobs;
use rmcp::schemars::JsonSchema;
use rmcp::{handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router};
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub(crate) struct JobInstancePath {
    /// The pipeline name, e.g. "myPipeline".
    pub pipeline_name: String,
    /// The pipeline counter, e.g. "1".
    pub pipeline_counter: String,
    /// The stage name, e.g. "myStages".
    pub stage_name: String,
    /// The stage counter, e.g. "1".
    pub stage_counter: String,
    /// The job name, e.g. "myJob".
    pub job_name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub(crate) struct JobHistory {
    /// The pipeline name, e.g. "mypipeline".
    pub pipeline_name: String,
    /// The stage name, e.g. "defaultStage".
    pub stage_name: String,
    /// The job name, e.g. "job1".
    pub job_name: String,
    /// The number of records per page, between 10 and 100 (defaults to 10).
    pub page_size: Option<i64>,
    /// The cursor value for fetching the next set of records, taken from the
    /// history's "next" link.
    pub after: Option<String>,
    /// The cursor value for fetching the previous set of records, taken from
    /// the history's "previous" link.
    pub before: Option<String>,
}

#[tool_router(router = jobs_tool_router, vis = "pub(crate)")]
impl OmgMcp {
    #[tool(
        description = "Read one job instance (GET /go/api/jobs/:pipeline_name/:pipeline_counter/:stage_name/:stage_counter/:job_name, API v1). Returns the job instance object — name, state, result, scheduled_date, rerun, agent_uuid, pipeline/stage names and counters, and job_state_transitions since GoCD 22.2.0 — with all `_links` removed. Docs: https://api.gocd.org/current/#get-job-instance"
    )]
    async fn get_job_instance(
        &self,
        Parameters(args): Parameters<JobInstancePath>,
    ) -> CallToolResult {
        self.request_shaped(jobs::read(
            &args.pipeline_name,
            &args.pipeline_counter,
            &args.stage_name,
            &args.stage_counter,
            &args.job_name,
        ))
        .await
    }

    #[tool(
        description = "List the job instances of a job (GET /go/api/jobs/:pipeline_name/:stage_name/:job_name/history, API v1). Supports cursor pagination: optional query args page_size (10-100, default 10), after and before take their values from the history's own next/previous links; unset args are omitted. Returns the `jobs` array of job instances with all `_links` removed. Docs: https://api.gocd.org/current/#get-job-history"
    )]
    async fn get_job_history(&self, Parameters(args): Parameters<JobHistory>) -> CallToolResult {
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
        self.request_shaped(jobs::history(
            &args.pipeline_name,
            &args.stage_name,
            &args.job_name,
            query,
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

    fn instance_args() -> super::JobInstancePath {
        super::JobInstancePath {
            pipeline_name: "myPipeline".into(),
            pipeline_counter: "1".into(),
            stage_name: "myStages".into(),
            stage_counter: "1".into(),
            job_name: "myJob".into(),
        }
    }

    fn history_args() -> super::JobHistory {
        super::JobHistory {
            pipeline_name: "mypipeline".into(),
            stage_name: "defaultStage".into(),
            job_name: "job1".into(),
            page_size: None,
            after: None,
            before: None,
        }
    }

    // Body taken verbatim from the GoCD API docs' job instance example.
    fn docs_instance_body() -> Value {
        json!({
            "name" : "myJob",
            "state" : "Completed",
            "result" : "Passed",
            "original_job_id" : null,
            "scheduled_date": 1436519914378i64,
            "rerun" : false,
            "agent_uuid" : "b74d88d-d8823765f63c",
            "pipeline_name" : "myPipeline",
            "pipeline_counter" : 1,
            "stage_name" : "myStages",
            "stage_counter" : "1",
            "job_state_transitions" : [
                { "state": "Scheduled", "state_change_time": 1578891141997i64 },
                { "state": "Assigned", "state_change_time": 1436509524491i64 },
                { "state": "Preparing", "state_change_time": 1436509534639i64 },
                { "state": "Building", "state_change_time": 1436509542522i64 },
                { "state": "Completing", "state_change_time": 1436509642582i64 },
                { "state": "Completed", "state_change_time": 1436509642631i64 }
            ]
        })
    }

    // Jobs array from the docs' history example; the `_links` objects are the
    // navigation data the docs' text says the JSON output provides.
    fn docs_history_body() -> Value {
        json!({
            "_links": {
                "self": { "href": "https://ci.example.com/go/api/jobs/mypipeline/defaultStage/job1/history" },
                "doc": { "href": "https://api.gocd.org/current/#jobs" },
                "next": { "href": "https://ci.example.com/go/api/jobs/mypipeline/defaultStage/job1/history?page_size=10&after=4" }
            },
            "jobs": [
                {
                    "_links": { "self": { "href": "https://ci.example.com/go/api/jobs/mypipeline/5/defaultStage/1/job1" } },
                    "name": "job1",
                    "agent_uuid": null,
                    "scheduled_date": 1436519914378i64,
                    "original_job_id": null,
                    "pipeline_counter": 5,
                    "rerun": false,
                    "pipeline_name": "mypipeline",
                    "result": "Unknown",
                    "state": "Scheduled",
                    "stage_counter": "1",
                    "stage_name": "defaultStage"
                },
                {
                    "_links": { "self": { "href": "https://ci.example.com/go/api/jobs/mypipeline/4/defaultStage/1/job1" } },
                    "name": "job1",
                    "agent_uuid": "278fb0b6-d3b8-47e1-9443-67f26bfb5c15",
                    "scheduled_date": 1436519733253i64,
                    "original_job_id": null,
                    "pipeline_counter": 4,
                    "rerun": false,
                    "pipeline_name": "mypipeline",
                    "result": "Passed",
                    "state": "Completed",
                    "stage_counter": "1",
                    "stage_name": "defaultStage"
                }
            ]
        })
    }

    #[tokio::test]
    async fn get_job_instance_sends_a_version_1_get_to_the_documented_path() {
        let fake = FakeGocd::replies(docs_instance_body(), None);
        let result = service(fake.clone())
            .get_job_instance(Parameters(instance_args()))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![GocdCall::get("api/jobs/myPipeline/1/myStages/1/myJob").version(1)]
        );
        assert_ne!(result.is_error, Some(true));
    }

    #[tokio::test]
    async fn get_job_instance_keeps_the_documented_fields_and_sends_no_query() {
        let fake = FakeGocd::replies(docs_instance_body(), None);
        let result = service(fake)
            .get_job_instance(Parameters(instance_args()))
            .await;

        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(shaped, docs_instance_body());
    }

    #[tokio::test]
    async fn get_job_history_sends_a_version_1_get_to_the_documented_path() {
        let fake = FakeGocd::replies(docs_history_body(), None);
        let result = service(fake.clone())
            .get_job_history(Parameters(history_args()))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![GocdCall::get("api/jobs/mypipeline/defaultStage/job1/history").version(1)]
        );
        assert_ne!(result.is_error, Some(true));
    }

    #[tokio::test]
    async fn get_job_history_drops_every_links_object_and_keeps_the_jobs() {
        let fake = FakeGocd::replies(docs_history_body(), Some("\"c0ffee\"".into()));
        let result = service(fake)
            .get_job_history(Parameters(history_args()))
            .await;

        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(
            shaped,
            json!({
                "jobs": [
                    {
                        "name": "job1",
                        "agent_uuid": null,
                        "scheduled_date": 1436519914378i64,
                        "original_job_id": null,
                        "pipeline_counter": 5,
                        "rerun": false,
                        "pipeline_name": "mypipeline",
                        "result": "Unknown",
                        "state": "Scheduled",
                        "stage_counter": "1",
                        "stage_name": "defaultStage"
                    },
                    {
                        "name": "job1",
                        "agent_uuid": "278fb0b6-d3b8-47e1-9443-67f26bfb5c15",
                        "scheduled_date": 1436519733253i64,
                        "original_job_id": null,
                        "pipeline_counter": 4,
                        "rerun": false,
                        "pipeline_name": "mypipeline",
                        "result": "Passed",
                        "state": "Completed",
                        "stage_counter": "1",
                        "stage_name": "defaultStage"
                    }
                ],
                "_etag": "\"c0ffee\""
            })
        );
    }

    #[tokio::test]
    async fn get_job_history_omits_unset_optional_params() {
        let fake = FakeGocd::replies(docs_history_body(), None);
        service(fake.clone())
            .get_job_history(Parameters(history_args()))
            .await;

        let call = &fake.recorded()[0];
        assert!(call.query.is_empty());
    }

    #[tokio::test]
    async fn get_job_history_sends_the_documented_optional_params_when_set() {
        let fake = FakeGocd::replies(docs_history_body(), None);
        service(fake.clone())
            .get_job_history(Parameters(super::JobHistory {
                page_size: Some(20),
                after: Some("4".into()),
                before: None,
                ..history_args()
            }))
            .await;

        let call = &fake.recorded()[0];
        assert_eq!(
            call.query,
            vec![
                ("page_size".to_string(), "20".to_string()),
                ("after".to_string(), "4".to_string())
            ]
        );
    }

    #[tokio::test]
    async fn an_unauthorized_read_surfaces_the_token_hint() {
        let fake = FakeGocd::fails(GocdError::Http { status: 401 });
        let result = service(fake)
            .get_job_instance(Parameters(instance_args()))
            .await;

        assert_eq!(result.is_error, Some(true));
        let text = first_text(&result);
        assert!(text.contains("401"), "got: {text}");
        assert!(text.contains("token"), "got: {text}");
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
        for expected in ["get_job_instance", "get_job_history"] {
            assert!(names.contains(&expected.to_string()), "{expected} missing");
        }
    }
}
