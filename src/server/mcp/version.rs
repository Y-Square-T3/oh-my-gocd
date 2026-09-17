// Version section: /go/api/version, API v1. A read-only, unparameterized
// GET: the docs define no query params and no writes, so no body or etag args
// exist in this section.

use super::OmgMcp;
use crate::gocd::version;
use rmcp::{model::CallToolResult, tool, tool_router};

#[tool_router(router = version_tool_router, vis = "pub(crate)")]
impl OmgMcp {
    #[tool(
        description = "Get the GoCD server version details (GET /go/api/version, API v1). Returns the version object — version, build_number, git_sha, full_version and commit_url — with `_links` removed and `_etag` included when GoCD sent one. Docs: https://api.gocd.org/current/#get-version"
    )]
    async fn get_version(&self) -> CallToolResult {
        self.request_shaped(version::read()).await
    }
}

#[cfg(test)]
mod tests {
    use crate::gocd::{GocdCall, GocdError};
    use crate::server::mcp::fake::{FakeGocd, first_text, service};
    use serde_json::{Value, json};

    // Body and ETag handling per the GoCD API docs' get-version example.
    fn docs_version_body() -> Value {
        json!({
            "_links": {
                "self": { "href": "https://build.go.cd/go/api/version" },
                "doc": { "href": "https://api.gocd.org/#version" }
            },
            "version": "16.6.0",
            "build_number": "3348",
            "git_sha": "a7a5717cbd60c30006314fb8dd529796c93adaf0",
            "full_version": "16.6.0 (3348-a7a5717cbd60c30006314fb8dd529796c93adaf0)",
            "commit_url": "https://github.com/gocd/gocd/commits/a7a5717cbd60c30006314fb8dd529796c93adaf0"
        })
    }

    #[tokio::test]
    async fn get_version_sends_a_version_1_get_to_the_documented_path() {
        let fake = FakeGocd::replies(json!({}), None);
        service(fake.clone()).get_version().await;

        assert_eq!(
            fake.recorded(),
            vec![GocdCall::get("api/version").version(1)]
        );
    }

    #[tokio::test]
    async fn get_version_drops_the_links_object_and_keeps_the_rest() {
        let fake = FakeGocd::replies(docs_version_body(), None);
        let result = service(fake).get_version().await;

        assert_ne!(result.is_error, Some(true));
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(
            shaped,
            json!({
                "version": "16.6.0",
                "build_number": "3348",
                "git_sha": "a7a5717cbd60c30006314fb8dd529796c93adaf0",
                "full_version": "16.6.0 (3348-a7a5717cbd60c30006314fb8dd529796c93adaf0)",
                "commit_url": "https://github.com/gocd/gocd/commits/a7a5717cbd60c30006314fb8dd529796c93adaf0"
            })
        );
    }

    #[tokio::test]
    async fn get_version_injects_the_etag_only_when_go_cd_sent_one() {
        let with_etag = FakeGocd::replies(docs_version_body(), Some("\"feedbeef\"".into()));
        let result = service(with_etag).get_version().await;
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(shaped["_etag"], json!("\"feedbeef\""));

        let without_etag = FakeGocd::replies(docs_version_body(), None);
        let result = service(without_etag).get_version().await;
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert!(shaped.get("_etag").is_none());
    }

    #[tokio::test]
    async fn an_unauthorized_read_surfaces_the_token_hint() {
        let fake = FakeGocd::fails(GocdError::Http { status: 401 });
        let result = service(fake).get_version().await;

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
            .find(|t| t.name == "get_version")
            .expect("get_version registered")
            .description
            .map(|d| d.to_string())
            .unwrap_or_default();

        assert!(description.contains("GET /go/api/version"));
        assert!(description.contains("API v1"));
        assert!(description.contains("#get-version"));
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
        assert!(names.contains(&"get_version".to_string()));
    }
}
