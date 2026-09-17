// Server Health section: /go/api/v1/health, API v1 — the version lives in the
// path, so GoCD serves plain JSON without a documented accept version. A
// read-only, unparameterized GET: the docs define no query params, body or
// If-Match for it, so no such args exist in this section.

use super::OmgMcp;
use crate::gocd::server_health;
use rmcp::{model::CallToolResult, tool, tool_router};

#[tool_router(router = server_health_tool_router, vis = "pub(crate)")]
impl OmgMcp {
    #[tool(
        description = "Check whether the GoCD server is up and running (GET /go/api/v1/health, API v1 — the version lives in the path, so no `application/vnd.go.cd` accept version is required). Returns the health object, e.g. `{\"health\": \"OK\"}`, with `_links` removed and `_etag` included when GoCD sent one. Usually used for health checks in Docker and Kubernetes. Docs: https://api.gocd.org/current/#check-server-health"
    )]
    async fn check_server_health(&self) -> CallToolResult {
        self.request_shaped(server_health::check()).await
    }
}

#[cfg(test)]
mod tests {
    use crate::gocd::{GocdCall, GocdError};
    use crate::server::mcp::fake::{FakeGocd, first_text, service};
    use serde_json::{Value, json};

    // Body per the GoCD API docs' check-server-health example, with a
    // `_links` object added to prove the uniform shaping applies.
    fn docs_health_body() -> Value {
        json!({
            "_links": {
                "self": { "href": "https://ci.example.com/go/api/v1/health" }
            },
            "health": "OK"
        })
    }

    #[tokio::test]
    async fn check_server_health_sends_a_version_1_get_to_the_documented_path() {
        let fake = FakeGocd::replies(json!({}), None);
        service(fake.clone()).check_server_health().await;

        assert_eq!(
            fake.recorded(),
            vec![GocdCall::get("api/v1/health").version(1)]
        );
    }

    #[tokio::test]
    async fn check_server_health_drops_the_links_object_and_keeps_the_rest() {
        let fake = FakeGocd::replies(docs_health_body(), None);
        let result = service(fake).check_server_health().await;

        assert_ne!(result.is_error, Some(true));
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(shaped, json!({ "health": "OK" }));
    }

    #[tokio::test]
    async fn check_server_health_injects_the_etag_only_when_go_cd_sent_one() {
        let with_etag = FakeGocd::replies(docs_health_body(), Some("\"feedbeef\"".into()));
        let result = service(with_etag).check_server_health().await;
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(shaped["_etag"], json!("\"feedbeef\""));

        let without_etag = FakeGocd::replies(docs_health_body(), None);
        let result = service(without_etag).check_server_health().await;
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert!(shaped.get("_etag").is_none());
    }

    #[tokio::test]
    async fn an_unauthorized_read_surfaces_the_token_hint() {
        let fake = FakeGocd::fails(GocdError::Http { status: 401 });
        let result = service(fake).check_server_health().await;

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
            .find(|t| t.name == "check_server_health")
            .expect("check_server_health registered")
            .description
            .map(|d| d.to_string())
            .unwrap_or_default();

        assert!(description.contains("GET /go/api/v1/health"));
        assert!(description.contains("API v1"));
        assert!(description.contains("#check-server-health"));
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
        assert!(names.contains(&"check_server_health".to_string()));
    }
}
