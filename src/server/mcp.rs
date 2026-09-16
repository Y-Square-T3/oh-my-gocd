// The MCP service: tools live here, GoCD access behind the gocd::GocdApi trait.

use crate::gocd::{GocdApi, GocdCall};
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
        description = "Get the GoCD user the configured token authenticates as (GET /go/api/current_user, API v1). Returns the user JSON — login_name, display_name, enabled, email, checkin_aliases and any other documented fields — with `_links` removed and `_etag` included when GoCD sent one."
    )]
    async fn get_current_user(&self) -> CallToolResult {
        let call = GocdCall::get("api/current_user").version(1);
        match self.api.request(call).await {
            Ok(reply) => {
                CallToolResult::success(vec![ContentBlock::text(reply.shaped().to_string())])
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
    use crate::gocd::{GocdCall, GocdError, GocdReply};
    use serde_json::{Value, json};
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::{Arc, Mutex};

    #[derive(Debug)]
    struct FakeGocd {
        calls: Mutex<Vec<GocdCall>>,
        outcome: Result<GocdReply, GocdError>,
    }

    impl FakeGocd {
        fn replies_with(reply: GocdReply) -> Arc<Self> {
            Arc::new(Self {
                calls: Mutex::new(Vec::new()),
                outcome: Ok(reply),
            })
        }
    }

    impl crate::gocd::GocdApi for FakeGocd {
        fn request(
            &self,
            call: GocdCall,
        ) -> Pin<Box<dyn Future<Output = Result<GocdReply, GocdError>> + Send + '_>> {
            self.calls.lock().unwrap().push(call);
            let outcome = self.outcome.clone();
            Box::pin(async move { outcome })
        }
    }

    fn service(fake: Arc<FakeGocd>) -> OmgMcp {
        OmgMcp::new(fake)
    }

    fn recorded(fake: &FakeGocd) -> Vec<GocdCall> {
        fake.calls.lock().unwrap().clone()
    }

    fn first_text(result: &rmcp::model::CallToolResult) -> String {
        match result.content.first() {
            Some(rmcp::model::ContentBlock::Text(text)) => text.text.clone(),
            other => panic!("expected one text block, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn get_current_user_sends_a_version_1_get_to_the_documented_path() {
        let fake = FakeGocd::replies_with(GocdReply {
            body: json!({}),
            etag: None,
        });
        service(fake.clone()).get_current_user().await;

        assert_eq!(
            recorded(&fake),
            vec![GocdCall::get("api/current_user").version(1)]
        );
    }

    #[tokio::test]
    async fn get_current_user_returns_the_body_shaped_by_the_unified_pattern() {
        // Shape taken verbatim from the GoCD API docs' current-user example.
        let fake = FakeGocd::replies_with(GocdReply {
            body: json!({
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
            etag: Some("\"deadbeef\"".into()),
        });
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
        let fake = Arc::new(FakeGocd {
            calls: Mutex::new(Vec::new()),
            outcome: Err(GocdError::Http { status: 401 }),
        });
        let result = service(fake).get_current_user().await;

        assert_eq!(result.is_error, Some(true));
        let text = first_text(&result);
        assert!(text.contains("401"), "got: {text}");
        assert!(text.contains("token"), "got: {text}");
    }

    #[test]
    fn server_advertises_omg_with_the_crate_version() {
        use rmcp::ServerHandler;
        let fake = FakeGocd::replies_with(GocdReply {
            body: json!({}),
            etag: None,
        });
        let info = service(fake).get_info();
        assert_eq!(info.server_info.name, "omg");
        assert_eq!(info.server_info.version, env!("CARGO_PKG_VERSION"));
        assert!(info.capabilities.tools.is_some());
    }
}
