// The MCP service: tools live here, GoCD access behind the gocd::GocdApi trait.
// Each GoCD doc section gets a submodule with its own tool block and merged
// router.

use crate::gocd::{GocdApi, GocdCall};
use rmcp::{
    ServerHandler,
    handler::server::router::tool::ToolRouter,
    model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerConfig},
    tool, tool_handler, tool_router,
};
use std::sync::Arc;

#[cfg(test)]
pub(crate) mod fake;

pub mod agents;
pub mod artifact_store;
pub mod artifacts_config;
pub mod authorization_config;
pub mod backups;
pub mod current_user;
pub mod dashboard;
pub mod encryption;
pub mod jobs;
pub mod pipeline_instances;
pub mod pipelines;
pub mod server_health;
pub mod server_health_messages;
pub mod stage_instances;
pub mod stages;
pub mod users;
pub mod version;

#[derive(Debug, Clone)]
pub struct OmgMcp {
    tool_router: ToolRouter<Self>,
    api: Arc<dyn GocdApi>,
}

#[tool_router]
impl OmgMcp {
    pub fn new(api: Arc<dyn GocdApi>) -> Self {
        let mut tool_router = Self::tool_router();
        tool_router.merge(Self::agents_tool_router());
        tool_router.merge(Self::artifact_store_tool_router());
        tool_router.merge(Self::artifacts_config_tool_router());
        tool_router.merge(Self::authorization_config_tool_router());
        tool_router.merge(Self::backups_tool_router());
        tool_router.merge(Self::current_user_tool_router());
        tool_router.merge(Self::dashboard_tool_router());
        tool_router.merge(Self::encryption_tool_router());
        tool_router.merge(Self::jobs_tool_router());
        tool_router.merge(Self::pipeline_instances_tool_router());
        tool_router.merge(Self::pipelines_tool_router());
        tool_router.merge(Self::server_health_tool_router());
        tool_router.merge(Self::server_health_messages_tool_router());
        tool_router.merge(Self::stage_instances_tool_router());
        tool_router.merge(Self::stages_tool_router());
        tool_router.merge(Self::users_tool_router());
        tool_router.merge(Self::version_tool_router());
        Self { tool_router, api }
    }

    /// One GoCD call for every tool: the reply shaped by the unified pattern
    /// on success, an actionable hint on failure.
    pub(crate) async fn request_shaped(&self, call: GocdCall) -> CallToolResult {
        match self.api.request(call).await {
            Ok(reply) => {
                CallToolResult::success(vec![ContentBlock::text(reply.shaped().to_string())])
            }
            Err(err) => CallToolResult::error(vec![ContentBlock::text(err.tool_message())]),
        }
    }

    #[tool(
        description = "Get the GoCD user the configured token authenticates as (GET /go/api/current_user, API v1). Returns the user JSON — login_name, display_name, enabled, email, checkin_aliases and any other documented fields — with `_links` removed and `_etag` included when GoCD sent one."
    )]
    async fn get_current_user(&self) -> CallToolResult {
        self.request_shaped(GocdCall::get("api/current_user").version(1))
            .await
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for OmgMcp {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("omg", env!("CARGO_PKG_VERSION")))
    }
}

#[cfg(test)]
mod tests {
    use super::fake::{FakeGocd, first_text, service};
    use crate::gocd::{GocdCall, GocdError};
    use serde_json::{Value, json};

    #[tokio::test]
    async fn get_current_user_sends_a_version_1_get_to_the_documented_path() {
        let fake = FakeGocd::replies(json!({}), None);
        service(fake.clone()).get_current_user().await;

        assert_eq!(
            fake.recorded(),
            vec![GocdCall::get("api/current_user").version(1)]
        );
    }

    #[tokio::test]
    async fn get_current_user_returns_the_body_shaped_by_the_unified_pattern() {
        // Shape taken verbatim from the GoCD API docs' current-user example.
        let fake = FakeGocd::replies(
            json!({
                "_links": {
                    "doc": { "href": "https://api.gocd.org/#current-user" },
                    "self": { "href": "https://ci.example.com/go/api/current_user" }
                },
                "login_name": "jdoe",
                "display_name": "John Doe",
                "enabled": true,
                "email": null,
                "email_me": false,
                "checkin_aliases": ["jdoe"]
            }),
            Some("\"deadbeef\"".into()),
        );
        let result = service(fake).get_current_user().await;

        assert_ne!(result.is_error, Some(true));
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(
            shaped,
            json!({
                "login_name": "jdoe",
                "display_name": "John Doe",
                "enabled": true,
                "email": null,
                "email_me": false,
                "checkin_aliases": ["jdoe"],
                "_etag": "\"deadbeef\""
            })
        );
    }

    #[tokio::test]
    async fn unauthorized_gocd_call_becomes_a_tool_error_with_a_hint() {
        let fake = FakeGocd::fails(GocdError::Http { status: 401 });
        let result = service(fake).get_current_user().await;

        assert_eq!(result.is_error, Some(true));
        let text = first_text(&result);
        assert!(text.contains("401"), "got: {text}");
        assert!(text.contains("token"), "got: {text}");
    }

    #[test]
    fn server_advertises_omg_with_the_crate_version() {
        use rmcp::ServerHandler;
        let info = service(FakeGocd::replies(json!({}), None)).get_info();
        assert_eq!(info.server_info.name, "omg");
        assert_eq!(info.server_info.version, env!("CARGO_PKG_VERSION"));
        assert!(info.capabilities.tools.is_some());
    }
}
