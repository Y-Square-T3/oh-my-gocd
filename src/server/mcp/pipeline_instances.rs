// Pipeline Instances section: /go/api/pipelines/:pipeline_name/:pipeline_counter,
// /go/api/pipelines/:pipeline_name/history and .../comment, API v1.
// GoCD documents no If-Match-gated write here, so no tool takes an etag arg.

use super::OmgMcp;
use crate::gocd::pipeline_instances;
use rmcp::schemars::JsonSchema;
use rmcp::{handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub(crate) struct PipelineInstancePath {
    /// The pipeline name, e.g. "PipelineName".
    pub pipeline_name: String,
    /// The pipeline counter, e.g. "1".
    pub pipeline_counter: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub(crate) struct PipelineHistory {
    /// The pipeline name, e.g. "pipeline1".
    pub pipeline_name: String,
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
pub(crate) struct PipelineComment {
    /// The pipeline name, e.g. "pipeline1".
    pub pipeline_name: String,
    /// The pipeline counter, e.g. "1".
    pub pipeline_counter: String,
    /// The comment request object: {"comment": "<text>"}.
    pub body: Value,
}

#[tool_router(router = pipeline_instances_tool_router, vis = "pub(crate)")]
impl OmgMcp {
    #[tool(
        description = "Read one pipeline instance (GET /go/api/pipelines/:pipeline_name/:pipeline_counter, API v1). Returns the pipeline instance object — name, counter, label, natural_order, can_run, preparing_to_schedule, comment, scheduled_date, build_cause and stages — with all `_links` removed and `_etag` included when GoCD sent one. Docs: https://api.gocd.org/current/#get-pipeline-instance"
    )]
    async fn get_pipeline_instance(
        &self,
        Parameters(args): Parameters<PipelineInstancePath>,
    ) -> CallToolResult {
        self.request_shaped(pipeline_instances::read(
            &args.pipeline_name,
            &args.pipeline_counter,
        ))
        .await
    }

    #[tool(
        description = "List the past instances of a pipeline (GET /go/api/pipelines/:pipeline_name/history, API v1). Supports cursor based pagination: optional query args page_size (10-100, default 10), after and before take their values from the history's own next/previous links; unset args are omitted. Returns the `pipelines` array of pipeline instance objects with all `_links` removed and `_etag` included when GoCD sent one. Docs: https://api.gocd.org/current/#get-pipeline-history"
    )]
    async fn get_pipeline_history(
        &self,
        Parameters(args): Parameters<PipelineHistory>,
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
        self.request_shaped(pipeline_instances::history(&args.pipeline_name, query))
            .await
    }

    #[tool(
        description = "Add a comment to a pipeline instance (POST /go/api/pipelines/:pipeline_name/:pipeline_counter/comment, API v1), e.g. the reason of failure or cancellation. `body` is {\"comment\": \"<text>\"}. Requires operate permission on the pipeline; GoCD documents no If-Match guard here. Returns {\"message\": ...} with `_links` removed and `_etag` included when GoCD sent one. Docs: https://api.gocd.org/current/#comment-on-pipeline-instance"
    )]
    async fn comment_pipeline_instance(
        &self,
        Parameters(args): Parameters<PipelineComment>,
    ) -> CallToolResult {
        self.request_shaped(pipeline_instances::comment(
            &args.pipeline_name,
            &args.pipeline_counter,
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

    fn instance_args() -> super::PipelineInstancePath {
        super::PipelineInstancePath {
            pipeline_name: "PipelineName".into(),
            pipeline_counter: "1".into(),
        }
    }

    fn history_args() -> super::PipelineHistory {
        super::PipelineHistory {
            pipeline_name: "pipeline1".into(),
            page_size: None,
            after: None,
            before: None,
        }
    }

    // Body taken verbatim from the GoCD API docs' pipeline instance example.
    fn docs_instance_body() -> Value {
        json!({
            "name" : "PipelineName",
            "counter" : 1,
            "label" : "1",
            "natural_order" : 1.0,
            "can_run" : true,
            "preparing_to_schedule" : false,
            "comment" : null,
            "scheduled_date" : 1436519914578i64,
            "build_cause" : {
                "trigger_message" : "modified by user <user@users.noreply.github.com>",
                "trigger_forced" : false,
                "approver" : "",
                "material_revisions" : [ {
                    "changed" : true,
                    "material" : {
                        "name" : "https://github.com/gocd/gocd",
                        "fingerprint" : "de08b34d116a1c0cf57cd76683bf21",
                        "type" : "Git",
                        "description" : "URL: https://github.com/gocd/gocd, Branch: master"
                    },
                    "modifications" : [ {
                        "revision" : "40f0a7ef224a0a2fba438b158483b",
                        "modified_time" : 1436519914378i64,
                        "user_name" : "user <user@users.noreply.github.com>",
                        "comment" : "some commit message.",
                        "email_address" : null
                    } ]
                } ]
            },
            "stages" : [ {
                "result" : "Passed",
                "status" : "Passed",
                "rerun_of_counter" : null,
                "name" : "stage",
                "counter" : "1",
                "scheduled" : true,
                "approval_type" : "success",
                "approved_by" : "changes",
                "operate_permission" : true,
                "can_run" : true,
                "jobs" : [ {
                    "name" : "job",
                    "scheduled_date" : 1436782534378i64,
                    "state" : "Completed",
                    "result" : "Passed"
                } ]
            } ]
        })
    }

    // Pipelines array from the docs' history example; the top-level `_links`
    // carry the cursor pagination data the docs' text describes.
    fn docs_history_body() -> Value {
        json!({
            "_links": {
                "next": { "href": "http://ci.example.com/go/api/pipelines/pipeline1/history?after=35" },
                "previous": { "href": "http://ci.example.com/go/api/pipelines/pipeline1/history?before=9" }
            },
            "pipelines": [
                {
                    "name": "pipeline1",
                    "counter": 13,
                    "label": "13",
                    "natural_order": 13.0,
                    "can_run": true,
                    "preparing_to_schedule": false,
                    "comment": null,
                    "scheduled_date": 1436519914578i64,
                    "build_cause": {
                        "trigger_message": "modified by user <user@users.noreply.github.com>",
                        "trigger_forced": false,
                        "approver": "",
                        "material_revisions": [ {
                            "changed": false,
                            "material": {
                                "name": "https://github.com/gocd/gocd",
                                "fingerprint": "de08b34d116a1cfc1fb988b6683bf21",
                                "type": "Git",
                                "description": "URL: https://github.com/gocd/gocd, Branch: master"
                            },
                            "modifications": [ {
                                "revision": "40f0a7ba43df37794cfc78b158483b",
                                "modified_time": 1436519914378i64,
                                "user_name": "user <user@users.noreply.github.com>",
                                "comment": "some commit message.",
                                "email_address": null
                            } ]
                        } ]
                    },
                    "stages": [ {
                        "result": "Failed",
                        "status": "Failed",
                        "rerun_of_counter": null,
                        "name": "pipeline1_stage",
                        "counter": "1",
                        "scheduled": true,
                        "approval_type": "success",
                        "approved_by": "changes",
                        "operate_permission": true,
                        "can_run": true,
                        "jobs": [ {
                            "name": "pipeline1_job",
                            "scheduled_date": 1436582744378i64,
                            "state": "Completed",
                            "result": "Failed"
                        } ]
                    } ]
                }
            ]
        })
    }

    #[tokio::test]
    async fn get_pipeline_instance_sends_a_version_1_get_to_the_documented_path() {
        let fake = FakeGocd::replies(docs_instance_body(), None);
        let result = service(fake.clone())
            .get_pipeline_instance(Parameters(instance_args()))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![GocdCall::get("api/pipelines/PipelineName/1").version(1)]
        );
        assert_ne!(result.is_error, Some(true));
    }

    #[tokio::test]
    async fn get_pipeline_instance_drops_links_and_keeps_the_documented_fields() {
        // The docs' example carries no `_links`; one is bolted on the way
        // real answers do, so the drop is exercised.
        let mut body = docs_instance_body();
        body["_links"] = json!({
            "self": { "href": "http://ci.example.com/go/api/pipelines/PipelineName/1" }
        });
        let fake = FakeGocd::replies(body.clone(), None);
        let result = service(fake)
            .get_pipeline_instance(Parameters(instance_args()))
            .await;

        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        body.as_object_mut().unwrap().remove("_links");
        assert_eq!(shaped, body);
    }

    #[tokio::test]
    async fn get_pipeline_instance_injects_the_etag_only_when_go_cd_sent_one() {
        let with_etag = FakeGocd::replies(docs_instance_body(), Some("\"feedbeef\"".into()));
        let result = service(with_etag)
            .get_pipeline_instance(Parameters(instance_args()))
            .await;
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(shaped["_etag"], json!("\"feedbeef\""));

        let without_etag = FakeGocd::replies(docs_instance_body(), None);
        let result = service(without_etag)
            .get_pipeline_instance(Parameters(instance_args()))
            .await;
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert!(shaped.get("_etag").is_none());
    }

    #[tokio::test]
    async fn get_pipeline_history_sends_a_version_1_get_to_the_documented_path() {
        let fake = FakeGocd::replies(docs_history_body(), None);
        let result = service(fake.clone())
            .get_pipeline_history(Parameters(history_args()))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![GocdCall::get("api/pipelines/pipeline1/history").version(1)]
        );
        assert_ne!(result.is_error, Some(true));
    }

    #[tokio::test]
    async fn get_pipeline_history_drops_every_links_object_recursively() {
        // The docs' example carries a top-level `_links` of cursor pointers;
        // a nested one is bolted on the way real answers do, so the drop is
        // exercised recursively.
        let mut body = docs_history_body();
        body["pipelines"][0]["_links"] =
            json!({ "self": { "href": "http://ci.example.com/go/api/pipelines/pipeline1/1" } });
        let mut expected = body.clone();
        expected.as_object_mut().unwrap().remove("_links");
        expected["pipelines"][0]
            .as_object_mut()
            .unwrap()
            .remove("_links");
        expected
            .as_object_mut()
            .unwrap()
            .insert("_etag".to_string(), json!("\"c0ffee\""));

        let fake = FakeGocd::replies(body, Some("\"c0ffee\"".into()));
        let result = service(fake)
            .get_pipeline_history(Parameters(history_args()))
            .await;

        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(shaped, expected);
    }

    #[tokio::test]
    async fn get_pipeline_history_omits_unset_optional_params() {
        let fake = FakeGocd::replies(docs_history_body(), None);
        service(fake.clone())
            .get_pipeline_history(Parameters(history_args()))
            .await;

        let call = &fake.recorded()[0];
        assert!(call.query.is_empty());
    }

    #[tokio::test]
    async fn get_pipeline_history_sends_the_documented_optional_params_when_set() {
        let fake = FakeGocd::replies(docs_history_body(), None);
        service(fake.clone())
            .get_pipeline_history(Parameters(super::PipelineHistory {
                page_size: Some(20),
                after: Some("35".into()),
                before: None,
                ..history_args()
            }))
            .await;

        let call = &fake.recorded()[0];
        assert_eq!(
            call.query,
            vec![
                ("page_size".to_string(), "20".to_string()),
                ("after".to_string(), "35".to_string())
            ]
        );
    }

    #[tokio::test]
    async fn comment_pipeline_instance_posts_the_documented_body_to_the_comment_path() {
        let fake = FakeGocd::replies(json!({ "message": "Comment successfully updated." }), None);
        let result = service(fake.clone())
            .comment_pipeline_instance(Parameters(super::PipelineComment {
                pipeline_name: "pipeline1".into(),
                pipeline_counter: "1".into(),
                body: json!({ "comment": "Failed bacause of flaky tests" }),
            }))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![
                GocdCall::post("api/pipelines/pipeline1/1/comment")
                    .version(1)
                    .body(json!({ "comment": "Failed bacause of flaky tests" }))
            ]
        );
        assert_ne!(result.is_error, Some(true));
        assert!(first_text(&result).contains("Comment successfully updated"));
    }

    #[tokio::test]
    async fn an_unauthorized_read_surfaces_the_token_hint() {
        let fake = FakeGocd::fails(GocdError::Http { status: 401 });
        let result = service(fake)
            .get_pipeline_instance(Parameters(instance_args()))
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

        let instance = described("get_pipeline_instance");
        assert!(instance.contains("GET /go/api/pipelines/:pipeline_name/:pipeline_counter"));
        assert!(instance.contains("API v1"));
        assert!(instance.contains("#get-pipeline-instance"));

        let history = described("get_pipeline_history");
        assert!(history.contains("GET /go/api/pipelines/:pipeline_name/history"));
        assert!(history.contains("#get-pipeline-history"));

        let comment = described("comment_pipeline_instance");
        assert!(
            comment.contains("POST /go/api/pipelines/:pipeline_name/:pipeline_counter/comment")
        );
        assert!(comment.contains("#comment-on-pipeline-instance"));
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
            "comment_pipeline_instance",
            "get_pipeline_history",
            "get_pipeline_instance",
        ] {
            assert!(names.contains(&expected.to_string()), "{expected} missing");
        }
    }
}
