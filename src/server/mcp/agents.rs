// Agents section: /go/api/agents, API v7; job run history API v1. GoCD exposes
// no ETag/If-Match for agents, so the write tools take no `etag` argument.

use super::OmgMcp;
use crate::gocd::agents;
use rmcp::schemars::JsonSchema;
use rmcp::{handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub(crate) struct AgentUuid {
    /// The agent uuid, e.g. "adb9540a-b954-4571-9d9b-2f330739d4da".
    pub uuid: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub(crate) struct UpdateAgent {
    /// The agent uuid to update.
    pub uuid: String,
    /// The attributes to change; at least one of: {"hostname", "resources",
    /// "environments", "agent_config_state"}. Omitted attributes are left
    /// unchanged.
    pub body: Value,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub(crate) struct BulkAgentsRequest {
    /// The bulk request body: for update, {"uuids": [...], "operations":
    /// {"resources"/"environments": {"add"/"remove": [...]}}, "agent_config_state":
    /// ...}; for delete, {"uuids": [...]}.
    pub body: Value,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub(crate) struct JobRunHistory {
    /// The agent uuid whose job run history to read.
    pub uuid: String,
    /// The number of records to skip.
    pub offset: Option<i64>,
    /// The number of records per page, between 10 and 100 (defaults to 50).
    pub page_size: Option<i64>,
    /// The column to sort by: one of "pipeline", "stage", "job", "result",
    /// "completed" (defaults to "completed").
    pub sort_column: Option<String>,
    /// The sort order: "ASC" or "DESC" (defaults to "DESC").
    pub sort_order: Option<String>,
}

#[tool_router(router = agents_tool_router, vis = "pub(crate)")]
impl OmgMcp {
    #[tool(
        description = "List all agents, registered and pending (GET /go/api/agents, API v7). Returns `_embedded.agents` — uuid, hostname, ip_address, sandbox, operating_system, free_space, agent_config_state, agent_state, versions, resources, environments and build state — with all `_links` removed. Docs: https://api.gocd.org/current/#get-all-agents"
    )]
    async fn get_agents(&self) -> CallToolResult {
        self.request_shaped(agents::list()).await
    }

    #[tool(
        description = "Read one agent by uuid (GET /go/api/agents/:uuid, API v7). Returns the agent object — uuid, hostname, ip_address, sandbox, operating_system, free_space, agent_config_state, agent_state, versions, resources, environments, build_state and build_details — with all `_links` removed. Docs: https://api.gocd.org/current/#get-one-agent"
    )]
    async fn get_agent(&self, Parameters(args): Parameters<AgentUuid>) -> CallToolResult {
        self.request_shaped(agents::read(&args.uuid)).await
    }

    #[tool(
        description = "Update attributes of one agent (PATCH /go/api/agents/:uuid, API v7). `body` carries at least one of {\"hostname\", \"resources\", \"environments\", \"agent_config_state\"}; omitted attributes are left unchanged. Can approve pending agents by setting agent_config_state to \"Enabled\". Returns the updated agent object with `_links` removed. GoCD does not gate this write behind If-Match. Docs: https://api.gocd.org/current/#update-an-agent"
    )]
    async fn update_agent(&self, Parameters(args): Parameters<UpdateAgent>) -> CallToolResult {
        self.request_shaped(agents::update(&args.uuid, args.body))
            .await
    }

    #[tool(
        description = "Delete one agent by uuid (DELETE /go/api/agents/:uuid, API v7). Disable the agent first and ensure it is not Building. Returns a message confirming deletion. Docs: https://api.gocd.org/current/#delete-an-agent"
    )]
    async fn delete_agent(&self, Parameters(args): Parameters<AgentUuid>) -> CallToolResult {
        self.request_shaped(agents::remove(&args.uuid)).await
    }

    #[tool(
        description = "Bulk update agents (PATCH /go/api/agents, API v7). `body` is {\"uuids\": [...], \"operations\": {\"resources\"/\"environments\": {\"add\": [...], \"remove\": [...]}}, \"agent_config_state\": ...} — docs: https://api.gocd.org/current/#the-bulk-update-operation-attributes. Returns a message confirming the update. Docs: https://api.gocd.org/current/#bulk-update-agents"
    )]
    async fn bulk_update_agents(
        &self,
        Parameters(args): Parameters<BulkAgentsRequest>,
    ) -> CallToolResult {
        self.request_shaped(agents::bulk_update(args.body)).await
    }

    #[tool(
        description = "Bulk delete agents (DELETE /go/api/agents, API v7). `body` is {\"uuids\": [...]} listing the agents to delete. Returns a message confirming how many agents were deleted. Docs: https://api.gocd.org/current/#bulk-delete-agents"
    )]
    async fn bulk_delete_agents(
        &self,
        Parameters(args): Parameters<BulkAgentsRequest>,
    ) -> CallToolResult {
        self.request_shaped(agents::bulk_delete(args.body)).await
    }

    #[tool(
        description = "List the jobs that have run on an agent (GET /go/api/agents/:uuid/job_run_history, API v1). Optional query args offset, page_size (10-100, default 50), sort_column (pipeline|stage|job|result|completed, default completed) and sort_order (ASC|DESC, default DESC) paginate and sort the history; unset args are omitted. Returns the agent's job run history (job state transitions, names, counters, result) with `_links` removed. Docs: https://api.gocd.org/current/#agent-job-run-history"
    )]
    async fn get_agent_job_run_history(
        &self,
        Parameters(args): Parameters<JobRunHistory>,
    ) -> CallToolResult {
        let mut query = Vec::new();
        if let Some(offset) = args.offset {
            query.push(("offset".to_string(), offset.to_string()));
        }
        if let Some(page_size) = args.page_size {
            query.push(("page_size".to_string(), page_size.to_string()));
        }
        if let Some(sort_column) = args.sort_column {
            query.push(("sort_column".to_string(), sort_column));
        }
        if let Some(sort_order) = args.sort_order {
            query.push(("sort_order".to_string(), sort_order));
        }
        self.request_shaped(agents::job_run_history(&args.uuid, query))
            .await
    }

    #[tool(
        description = "Kill all running tasks on an agent (POST /go/api/agents/:uuid/kill_running_tasks, API v7). Returns GoCD's acknowledgement; 409 if the agent cannot be instructed. Docs: https://api.gocd.org/current/#kill-running-tasks"
    )]
    async fn kill_agent_running_tasks(
        &self,
        Parameters(args): Parameters<AgentUuid>,
    ) -> CallToolResult {
        self.request_shaped(agents::kill_running_tasks(&args.uuid))
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::gocd::{GocdCall, GocdError};
    use crate::server::mcp::fake::{FakeGocd, first_text, service};
    use rmcp::handler::server::wrapper::Parameters;
    use serde_json::{Value, json};

    const UUID: &str = "adb9540a-b954-4571-9d9b-2f330739d4da";

    // List body shape taken verbatim from the GoCD API docs' list example.
    fn stored_list_body() -> Value {
        json!({
            "_links": {
                "self": { "href": "https://ci.example.com/go/api/agents" },
                "doc": { "href": "https://api.gocd.org/#agents" }
            },
            "_embedded": {
                "agents": [ {
                    "_links": {
                        "self": { "href": "https://ci.example.com/go/api/agents/adb9540a-b954-4571-9d9b-2f330739d4da" },
                        "find": { "href": "https://ci.example.com/go/api/agents/:uuid" }
                    },
                    "uuid": "adb9540a-b954-4571-9d9b-2f330739d4da",
                    "hostname": "agent01.example.com",
                    "ip_address": "10.12.20.47",
                    "sandbox": "/var/lib/go-agent",
                    "operating_system": "Mac OS X",
                    "free_space": 84983328768u64,
                    "agent_config_state": "Enabled",
                    "agent_state": "Idle",
                    "agent_version": "20.5.0",
                    "agent_bootstrapper_version": "20.5.0",
                    "resources": ["java", "linux", "firefox"],
                    "environments": [ {
                        "name": "perf",
                        "origin": {
                            "type": "gocd",
                            "_links": { "self": { "href": "https://ci.example.com/go/admin/config_xml" } }
                        }
                    } ],
                    "build_state": "Idle"
                } ]
            }
        })
    }

    fn shaped_list_agent() -> Value {
        json!({
            "_embedded": {
                "agents": [ {
                    "uuid": "adb9540a-b954-4571-9d9b-2f330739d4da",
                    "hostname": "agent01.example.com",
                    "ip_address": "10.12.20.47",
                    "sandbox": "/var/lib/go-agent",
                    "operating_system": "Mac OS X",
                    "free_space": 84983328768u64,
                    "agent_config_state": "Enabled",
                    "agent_state": "Idle",
                    "agent_version": "20.5.0",
                    "agent_bootstrapper_version": "20.5.0",
                    "resources": ["java", "linux", "firefox"],
                    "environments": [ {
                        "name": "perf",
                        "origin": { "type": "gocd" }
                    } ],
                    "build_state": "Idle"
                } ]
            }
        })
    }

    // Single-agent body taken verbatim from the GoCD API docs' get-one example.
    fn docs_single_agent_body() -> Value {
        json!({
            "_links": {
                "self": { "href": "https://ci.example.com/go/api/agents/adb9540a-b954-4571-9d9b-2f330739d4da" },
                "doc": { "href": "https://api.gocd.org/#agents" },
                "find": { "href": "https://ci.example.com/go/api/agents/:uuid" }
            },
            "uuid": "adb9540a-b954-4571-9d9b-2f330739d4da",
            "hostname": "ketanpkr.corporate.thoughtworks.com",
            "ip_address": "10.12.20.47",
            "sandbox": "/Users/ketanpadegaonkar/projects/gocd/gocd/agent",
            "operating_system": "Mac OS X",
            "free_space": 85890146304u64,
            "agent_config_state": "Enabled",
            "agent_state": "Building",
            "agent_version": "20.5.0",
            "agent_bootstrapper_version": "20.5.0",
            "resources": ["java", "linux", "firefox"],
            "environments": [ {
                "name": "perf",
                "origin": {
                    "type": "gocd",
                    "_links": { "self": { "href": "https://ci.example.com/go/admin/config_xml" } }
                }
            } ],
            "build_state": "Building",
            "build_details": {
                "_links": { "job": { "href": "https://ci.example.com/go/tab/build/detail/up42/1/up42_stage/1/up42_job" } },
                "pipeline_name": "up42",
                "stage_name": "up42_stage",
                "job_name": "up42_job"
            }
        })
    }

    #[tokio::test]
    async fn get_agents_lists_through_a_version_7_get_and_shapes_the_answer() {
        let fake = FakeGocd::replies(stored_list_body(), None);
        let result = service(fake.clone()).get_agents().await;

        assert_eq!(
            fake.recorded(),
            vec![GocdCall::get("api/agents").version(7)]
        );
        assert_ne!(result.is_error, Some(true));
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(shaped, shaped_list_agent());
    }

    #[tokio::test]
    async fn get_agent_reads_one_agent_by_uuid() {
        let fake = FakeGocd::replies(docs_single_agent_body(), None);
        let result = service(fake.clone())
            .get_agent(Parameters(super::AgentUuid { uuid: UUID.into() }))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![GocdCall::get(&format!("api/agents/{UUID}")).version(7)]
        );
        assert_ne!(result.is_error, Some(true));
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(shaped.get("_links"), None);
        assert_eq!(shaped["uuid"], json!(UUID));
        assert_eq!(shaped["build_details"]["_links"], Value::Null);
    }

    #[tokio::test]
    async fn update_agent_patches_the_body_by_uuid() {
        let body = json!({ "agent_config_state": "Disabled", "resources": ["Java", "Linux"] });
        let fake = FakeGocd::replies(docs_single_agent_body(), None);
        let result = service(fake.clone())
            .update_agent(Parameters(super::UpdateAgent {
                uuid: UUID.into(),
                body: body.clone(),
            }))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![
                GocdCall::patch(&format!("api/agents/{UUID}"))
                    .version(7)
                    .body(body)
            ]
        );
        assert_ne!(result.is_error, Some(true));
    }

    #[tokio::test]
    async fn delete_agent_deletes_by_uuid_and_returns_the_message() {
        let fake = FakeGocd::replies(json!({ "message": "Deleted 1 agent(s)." }), None);
        let result = service(fake.clone())
            .delete_agent(Parameters(super::AgentUuid { uuid: UUID.into() }))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![GocdCall::delete(&format!("api/agents/{UUID}")).version(7)]
        );
        assert_ne!(result.is_error, Some(true));
        assert!(first_text(&result).contains("Deleted 1 agent(s)."));
    }

    #[tokio::test]
    async fn bulk_update_agents_patches_the_collection_with_the_body() {
        let body = json!({
            "uuids": [UUID, "adb528b2-b954-1234-9d9b-b27ag4h568e1"],
            "operations": { "environments": { "add": ["Dev"], "remove": [] } },
            "agent_config_state": "Enabled"
        });
        let fake = FakeGocd::replies(
            json!({ "message": "Updated agent(s) with uuid(s): [a, b]." }),
            None,
        );
        let result = service(fake.clone())
            .bulk_update_agents(Parameters(super::BulkAgentsRequest { body: body.clone() }))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![GocdCall::patch("api/agents").version(7).body(body)]
        );
        assert_ne!(result.is_error, Some(true));
    }

    #[tokio::test]
    async fn bulk_delete_agents_deletes_the_collection_with_the_body() {
        let body = json!({ "uuids": [UUID, "adb528b2-b954-1234-9d9b-b27ag4h568e1"] });
        let fake = FakeGocd::replies(json!({ "message": "Deleted 2 agent(s)." }), None);
        let result = service(fake.clone())
            .bulk_delete_agents(Parameters(super::BulkAgentsRequest { body: body.clone() }))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![GocdCall::delete("api/agents").version(7).body(body)]
        );
        assert_ne!(result.is_error, Some(true));
    }

    #[tokio::test]
    async fn get_agent_job_run_history_sends_only_the_set_query_params() {
        let fake = FakeGocd::replies(json!({ "uuid": UUID, "jobs": [] }), None);
        let result = service(fake.clone())
            .get_agent_job_run_history(Parameters(super::JobRunHistory {
                uuid: UUID.into(),
                offset: Some(40),
                page_size: None,
                sort_column: Some("completed".into()),
                sort_order: None,
            }))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![
                GocdCall::get(&format!("api/agents/{UUID}/job_run_history"))
                    .version(1)
                    .query(vec![
                        ("offset".to_string(), "40".to_string()),
                        ("sort_column".to_string(), "completed".to_string()),
                    ])
            ]
        );
        assert_ne!(result.is_error, Some(true));
    }

    #[tokio::test]
    async fn get_agent_job_run_history_omits_query_when_nothing_is_set() {
        let fake = FakeGocd::replies(json!({ "uuid": UUID, "jobs": [] }), None);
        service(fake.clone())
            .get_agent_job_run_history(Parameters(super::JobRunHistory {
                uuid: UUID.into(),
                offset: None,
                page_size: None,
                sort_column: None,
                sort_order: None,
            }))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![GocdCall::get(&format!("api/agents/{UUID}/job_run_history")).version(1)]
        );
    }

    #[tokio::test]
    async fn kill_agent_running_tasks_posts_to_the_endpoint() {
        let fake = FakeGocd::replies(json!({ "message": "Killed running tasks." }), None);
        let result = service(fake.clone())
            .kill_agent_running_tasks(Parameters(super::AgentUuid { uuid: UUID.into() }))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![GocdCall::post(&format!("api/agents/{UUID}/kill_running_tasks")).version(7)]
        );
        assert_ne!(result.is_error, Some(true));
    }

    #[tokio::test]
    async fn an_unauthorized_read_surfaces_the_token_hint() {
        let fake = FakeGocd::fails(GocdError::Http { status: 401 });
        let result = service(fake).get_agents().await;

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
        for expected in [
            "bulk_delete_agents",
            "bulk_update_agents",
            "delete_agent",
            "get_agent",
            "get_agent_job_run_history",
            "get_agents",
            "kill_agent_running_tasks",
            "update_agent",
        ] {
            assert!(names.contains(&expected.to_string()), "{expected} missing");
        }
    }
}
