// The MCP service: tools live here, GoCD access behind the gocd::GocdApi trait.

use crate::gocd::GocdApi;
use crate::gocd::user::prune_identity;
use rmcp::{
    ServerHandler,
    handler::server::router::tool::ToolRouter,
    model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerConfig},
    tool, tool_handler, tool_router,
};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct OmgMcp {
    tool_router: ToolRouter<Self>,
    api: Arc<dyn GocdApi>,
}

#[tool_router]
impl OmgMcp {
    pub fn new(api: Arc<dyn GocdApi>) -> Self {
        Self {
            tool_router: Self::tool_router(),
            api,
        }
    }

    #[tool(
        description = "Get the GoCD user the configured token authenticates as. Returns JSON with login_name, display_name, enabled, email and checkin_aliases."
    )]
    async fn get_current_user(&self) -> CallToolResult {
        match self.api.fetch_current_user().await {
            Ok(user) => {
                CallToolResult::success(vec![ContentBlock::text(prune_identity(&user).to_string())])
            }
            Err(err) => CallToolResult::error(vec![ContentBlock::text(err.tool_message())]),
        }
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
    use super::*;
    use crate::gocd::{GocdError, user::gocd_doc_example};
    use serde_json::json;
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::Arc;

    #[derive(Debug)]
    struct FakeGocd {
        outcome: Result<serde_json::Value, String>,
    }

    impl crate::gocd::GocdApi for FakeGocd {
        fn fetch_current_user(
            &self,
        ) -> Pin<Box<dyn Future<Output = Result<serde_json::Value, GocdError>> + Send + '_>>
        {
            let outcome = self.outcome.clone().map_err(|m| match m.as_str() {
                "401" => GocdError::Http { status: 401 },
                other => GocdError::Transport(other.to_owned()),
            });
            Box::pin(async move { outcome })
        }
    }

    fn service(outcome: Result<serde_json::Value, String>) -> OmgMcp {
        OmgMcp::new(Arc::new(FakeGocd { outcome }))
    }

    fn first_text(result: &rmcp::model::CallToolResult) -> String {
        match result.content.first() {
            Some(rmcp::model::ContentBlock::Text(text)) => text.text.clone(),
            other => panic!("expected one text block, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn success_returns_pruned_identity_as_json() {
        let result = service(Ok(gocd_doc_example())).get_current_user().await;

        assert_ne!(result.is_error, Some(true));
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&first_text(&result)).unwrap(),
            json!({
                "login_name": "jdoe",
                "display_name": "John Doe",
                "enabled": true,
                "email": null,
                "checkin_aliases": ["jdoe"]
            })
        );
    }

    #[tokio::test]
    async fn unauthorized_gocd_call_becomes_a_tool_error_with_a_hint() {
        let result = service(Err("401".into())).get_current_user().await;

        assert_eq!(result.is_error, Some(true));
        let text = first_text(&result);
        assert!(text.contains("401"), "got: {text}");
        assert!(text.contains("token"), "got: {text}");
    }

    #[test]
    fn server_advertises_omg_with_the_crate_version() {
        use rmcp::ServerHandler;
        let info = service(Ok(json!({}))).get_info();
        assert_eq!(info.server_info.name, "omg");
        assert_eq!(info.server_info.version, env!("CARGO_PKG_VERSION"));
        assert!(info.capabilities.tools.is_some());
    }
}
