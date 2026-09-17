// Server Health Messages section: /go/api/server_health_messages, API v1.
// A read-only, unparameterized GET: the docs define no query params, body or
// If-Match for it, so no such args exist in this section. The answer is a
// JSON array, so the shaping drops `_links` inside each message; GoCD sends
// no ETag for the collection.

use super::OmgMcp;
use crate::gocd::server_health_messages;
use rmcp::{model::CallToolResult, tool, tool_router};

#[tool_router(router = server_health_messages_tool_router, vis = "pub(crate)")]
impl OmgMcp {
    #[tool(
        description = "List the current server health messages — the errors and warnings the GoCD server generates as part of its normal routine, the same set the web UI shows in its errors-and-warnings modal (GET /go/api/server_health_messages, API v1). Returns an array of message objects (`message`, `detail`, `level` WARNING/ERROR, `time` UTC generated), with every `_links` object removed. Docs: https://api.gocd.org/current/#get-server-health-messages"
    )]
    async fn get_server_health_messages(&self) -> CallToolResult {
        self.request_shaped(server_health_messages::list()).await
    }
}

#[cfg(test)]
mod tests {
    use crate::gocd::{GocdCall, GocdError};
    use crate::server::mcp::fake::{FakeGocd, first_text, service};
    use serde_json::{Value, json};

    // Messages per the GoCD API docs' get-server-health-messages example,
    // with `_links` objects added to prove the uniform shaping prunes them
    // inside array elements too.
    fn docs_messages_body() -> Value {
        json!([
            {
                "_links": { "self": { "href": "https://ci.example.com/go/api/server_health_messages" } },
                "message": "Job 'Security-Checks/test/dependency-check' is not responding",
                "detail": "Job <a href='/go/tab/build/detail/Security-Checks/847/test/1/dependency-check'>Security-Checks/test/dependency-check</a> is currently running but has not shown any console activity in the last 26 minute(s). This job may be hung.",
                "level": "WARNING",
                "time": "2018-02-27T07:36:30Z"
            },
            {
                "message": "Config repo 'foo' is errored",
                "detail": "Could not poll the repository.",
                "level": "ERROR",
                "time": "2018-02-27T08:00:00Z"
            }
        ])
    }

    #[tokio::test]
    async fn get_server_health_messages_sends_a_version_1_get_to_the_documented_path() {
        let fake = FakeGocd::replies(json!([]), None);
        service(fake.clone()).get_server_health_messages().await;

        assert_eq!(
            fake.recorded(),
            vec![GocdCall::get("api/server_health_messages").version(1)]
        );
    }

    #[tokio::test]
    async fn get_server_health_messages_drops_links_from_every_message_and_keeps_the_rest() {
        let fake = FakeGocd::replies(docs_messages_body(), None);
        let result = service(fake).get_server_health_messages().await;

        assert_ne!(result.is_error, Some(true));
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(
            shaped,
            json!([
                {
                    "message": "Job 'Security-Checks/test/dependency-check' is not responding",
                    "detail": "Job <a href='/go/tab/build/detail/Security-Checks/847/test/1/dependency-check'>Security-Checks/test/dependency-check</a> is currently running but has not shown any console activity in the last 26 minute(s). This job may be hung.",
                    "level": "WARNING",
                    "time": "2018-02-27T07:36:30Z"
                },
                {
                    "message": "Config repo 'foo' is errored",
                    "detail": "Could not poll the repository.",
                    "level": "ERROR",
                    "time": "2018-02-27T08:00:00Z"
                }
            ])
        );
    }

    #[tokio::test]
    async fn an_unauthorized_read_surfaces_the_token_hint() {
        let fake = FakeGocd::fails(GocdError::Http { status: 401 });
        let result = service(fake).get_server_health_messages().await;

        assert_eq!(result.is_error, Some(true));
        let text = first_text(&result);
        assert!(text.contains("401"), "got: {text}");
        assert!(text.contains("token"), "got: {text}");
    }

    #[test]
    fn the_tool_description_cites_the_documented_endpoint() {
        let service = service(FakeGocd::replies(json!({}), None));
        let description = service
            .tool_router
            .list_all()
            .into_iter()
            .find(|t| t.name == "get_server_health_messages")
            .expect("get_server_health_messages registered")
            .description
            .map(|d| d.to_string())
            .unwrap_or_default();

        assert!(description.contains("GET /go/api/server_health_messages"));
        assert!(description.contains("API v1"));
        assert!(description.contains("#get-server-health-messages"));
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
        assert!(names.contains(&"get_server_health_messages".to_string()));
    }
}
