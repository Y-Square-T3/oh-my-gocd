// Dashboard section: /go/api/dashboard, API v4. A read-only, unparameterized
// GET: the docs define no query params, and GoCD requires no If-Match here.

use super::OmgMcp;
use crate::gocd::dashboard;
use rmcp::{model::CallToolResult, tool, tool_router};

#[tool_router(router = dashboard_tool_router, vis = "pub(crate)")]
impl OmgMcp {
    #[tool(
        description = "Get the dashboard of pipelines, their latest instances and stage status, pipeline groups and environments (GET /go/api/dashboard, API v4). The answer is personalized to the caller's permissions; returned with all `_links` removed and `_etag` included when GoCD sent one. Docs: https://api.gocd.org/current/#get-dashboard"
    )]
    async fn get_dashboard(&self) -> CallToolResult {
        self.request_shaped(dashboard::read()).await
    }
}

#[cfg(test)]
mod tests {
    use crate::gocd::{GocdCall, GocdError};
    use crate::server::mcp::fake::{FakeGocd, first_text, service};
    use serde_json::{Value, json};

    // Body shape taken verbatim from the GoCD API docs' get-dashboard example.
    fn docs_dashboard_body() -> Value {
        json!({
            "_links": {
                "self": { "href": "https://ci.example.com/go/api/dashboard" },
                "doc": { "href": "https://api.go.cd/current/#dashboard" }
            },
            "_personalization": "2356e5d4241f22987d8f8cb8920913fda237385b423bd7eec7e97b2a9eb1be1a",
            "_embedded": {
                "pipeline_groups": [ {
                    "_links": {
                        "doc": { "href": "https://api.go.cd/current/#pipeline-groups" },
                        "self": { "href": "https://ci.example.com/go/api/config/pipeline_groups" }
                    },
                    "name": "first",
                    "pipelines": [ "up42" ],
                    "can_administer": true
                } ],
                "environments": [ {
                    "_links": {
                        "doc": { "href": "https://api.gocd.org/current/#environment-config" },
                        "self": { "href": "https://ci.example.com/go/api/admin/environments/asd" }
                    },
                    "name": "asd",
                    "pipelines": [ "up42" ],
                    "can_administer": true
                } ],
                "pipelines": [ {
                    "_links": {
                        "self": { "href": "https://ci.example.com/go/api/pipelines/up42/history" },
                        "doc": { "href": "https://api.go.cd/current/#pipelines" }
                    },
                    "name": "up42",
                    "last_updated_timestamp": 1542863609039u64,
                    "locked": false,
                    "pause_info": { "paused": false, "paused_by": null, "pause_reason": null },
                    "can_operate": true,
                    "can_administer": true,
                    "can_unlock": true,
                    "can_pause": true,
                    "from_config_repo": false,
                    "_embedded": {
                        "instances": [ {
                            "_links": {
                                "self": { "href": "https://ci.example.com/go/api/pipelines/up42/instance/1" }
                            },
                            "label": "1",
                            "counter": 1,
                            "triggered_by": "Triggered by changes",
                            "scheduled_at": "2018-11-21T09:43:07Z",
                            "_embedded": {
                                "stages": [ {
                                    "_links": {
                                        "self": { "href": "https://ci.example.com/go/api/stages/up42/1/up42_stage/1" }
                                    },
                                    "name": "up42_stage",
                                    "counter": "1",
                                    "status": "Building",
                                    "approved_by": "changes",
                                    "scheduled_at": "2018-11-21T09:43:07Z"
                                } ]
                            }
                        } ]
                    }
                } ]
            }
        })
    }

    #[tokio::test]
    async fn get_dashboard_sends_a_version_4_get_to_the_documented_path() {
        let fake = FakeGocd::replies(json!({}), None);
        service(fake.clone()).get_dashboard().await;

        assert_eq!(
            fake.recorded(),
            vec![GocdCall::get("api/dashboard").version(4)]
        );
    }

    #[tokio::test]
    async fn get_dashboard_drops_every_links_object_and_keeps_the_rest() {
        let fake = FakeGocd::replies(docs_dashboard_body(), None);
        let result = service(fake).get_dashboard().await;

        assert_ne!(result.is_error, Some(true));
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(
            shaped,
            json!({
                "_personalization": "2356e5d4241f22987d8f8cb8920913fda237385b423bd7eec7e97b2a9eb1be1a",
                "_embedded": {
                    "pipeline_groups": [ {
                        "name": "first",
                        "pipelines": [ "up42" ],
                        "can_administer": true
                    } ],
                    "environments": [ {
                        "name": "asd",
                        "pipelines": [ "up42" ],
                        "can_administer": true
                    } ],
                    "pipelines": [ {
                        "name": "up42",
                        "last_updated_timestamp": 1542863609039u64,
                        "locked": false,
                        "pause_info": { "paused": false, "paused_by": null, "pause_reason": null },
                        "can_operate": true,
                        "can_administer": true,
                        "can_unlock": true,
                        "can_pause": true,
                        "from_config_repo": false,
                        "_embedded": {
                            "instances": [ {
                                "label": "1",
                                "counter": 1,
                                "triggered_by": "Triggered by changes",
                                "scheduled_at": "2018-11-21T09:43:07Z",
                                "_embedded": {
                                    "stages": [ {
                                        "name": "up42_stage",
                                        "counter": "1",
                                        "status": "Building",
                                        "approved_by": "changes",
                                        "scheduled_at": "2018-11-21T09:43:07Z"
                                    } ]
                                }
                            } ]
                        }
                    } ]
                }
            })
        );
    }

    #[tokio::test]
    async fn get_dashboard_injects_the_etag_only_when_go_cd_sent_one() {
        let with_etag = FakeGocd::replies(docs_dashboard_body(), Some("\"feedbeef\"".into()));
        let result = service(with_etag).get_dashboard().await;
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(shaped["_etag"], json!("\"feedbeef\""));

        let without_etag = FakeGocd::replies(docs_dashboard_body(), None);
        let result = service(without_etag).get_dashboard().await;
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert!(shaped.get("_etag").is_none());
    }

    #[tokio::test]
    async fn an_unauthorized_read_surfaces_the_token_hint() {
        let fake = FakeGocd::fails(GocdError::Http { status: 401 });
        let result = service(fake).get_dashboard().await;

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
        assert!(names.contains(&"get_dashboard".to_string()));
    }
}
