// The MCP service: tools live here, GoCD access behind the gocd::GocdApi trait.
// Each GoCD doc section gets a submodule with its own tool block and merged
// router.

use crate::config::Mode;
use crate::gocd::{GocdApi, GocdCall, GocdError};
use crate::server::security;
use rmcp::{
    ErrorData, RoleServer, ServerHandler,
    handler::server::{router::tool::ToolRouter, tool::ToolCallContext},
    model::{
        CallToolRequestParams, CallToolResponse, CallToolResult, ContentBlock, Implementation,
        ServerCapabilities, ServerConfig,
    },
    service::RequestContext,
    tool, tool_handler, tool_router,
};
use std::sync::Arc;

#[cfg(test)]
pub(crate) mod fake;

pub mod access_tokens;
pub mod agents;
pub mod artifact_store;
pub mod artifacts;
pub mod artifacts_config;
pub mod authorization_config;
pub mod backup_config;
pub mod backups;
pub mod current_user;
pub mod dashboard;
pub mod derived;
pub mod encryption;
pub mod jobs;
pub mod maintenance_mode;
pub mod notify_materials;
pub mod pipeline_instances;
pub mod pipelines;
pub mod plugin_info;
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
    mode: Mode,
}

#[tool_router]
impl OmgMcp {
    /// The service as it will face the client: the full merged router with
    /// every tool the mode does not expose already removed from it.
    pub fn new(api: Arc<dyn GocdApi>, mode: Mode) -> Self {
        let mut tool_router = Self::tool_router();
        tool_router.merge(Self::access_tokens_tool_router());
        tool_router.merge(Self::agents_tool_router());
        tool_router.merge(Self::artifact_store_tool_router());
        tool_router.merge(Self::artifacts_tool_router());
        tool_router.merge(Self::artifacts_config_tool_router());
        tool_router.merge(Self::authorization_config_tool_router());
        tool_router.merge(Self::backup_config_tool_router());
        tool_router.merge(Self::backups_tool_router());
        tool_router.merge(Self::current_user_tool_router());
        tool_router.merge(Self::dashboard_tool_router());
        tool_router.merge(Self::derived_tool_router());
        tool_router.merge(Self::encryption_tool_router());
        tool_router.merge(Self::jobs_tool_router());
        tool_router.merge(Self::maintenance_mode_tool_router());
        tool_router.merge(Self::notify_materials_tool_router());
        tool_router.merge(Self::pipeline_instances_tool_router());
        tool_router.merge(Self::pipelines_tool_router());
        tool_router.merge(Self::plugin_info_tool_router());
        tool_router.merge(Self::server_health_tool_router());
        tool_router.merge(Self::server_health_messages_tool_router());
        tool_router.merge(Self::stage_instances_tool_router());
        tool_router.merge(Self::stages_tool_router());
        tool_router.merge(Self::users_tool_router());
        tool_router.merge(Self::version_tool_router());
        if mode != Mode::Full {
            for tool in tool_router.list_all() {
                if !security::tool_visible(&tool.name, mode) {
                    tool_router.disable_route(tool.name);
                }
            }
        }
        Self {
            tool_router,
            api,
            mode,
        }
    }

    /// Names of the tools this service actually exposes — a test view.
    #[cfg(test)]
    pub(crate) fn tool_names(&self) -> Vec<String> {
        self.tool_router
            .list_all()
            .into_iter()
            .map(|tool| tool.name.to_string())
            .collect()
    }

    /// The shaping call behind almost every tool: the reply shaped by the
    /// unified pattern on success, an actionable hint on failure. (The one
    /// raw-text tool builds its success block itself but funnels failures
    /// through [`tool_error`].)
    pub(crate) async fn request_shaped(&self, call: GocdCall) -> CallToolResult {
        match self.api.request(call).await {
            Ok(reply) => {
                CallToolResult::success(vec![ContentBlock::text(reply.shaped().to_string())])
            }
            Err(err) => tool_error(&err),
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

/// The failure answer every tool shares: the error's actionable hint as a
/// tool-error text block.
pub(crate) fn tool_error(err: &GocdError) -> CallToolResult {
    CallToolResult::error(vec![ContentBlock::text(err.tool_message())])
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for OmgMcp {
    // Mode-aware rejection before dispatch, gated on routes this server
    // actually registered and then hid: a client with a stale tool list
    // gets an explanation instead of the router's bare "tool not found",
    // while names omg never had keep that not-found answer.
    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, ErrorData> {
        if self.tool_router.is_disabled(&request.name)
            && let Some(message) = security::blocked_message(&request.name, self.mode)
        {
            return Ok(CallToolResult::error(vec![ContentBlock::text(message)]).into());
        }
        let tcc = ToolCallContext::new(self, request, context);
        self.tool_router.call(tcc).await
    }

    fn get_info(&self) -> ServerConfig {
        let config = ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("omg", env!("CARGO_PKG_VERSION")));
        if self.mode == Mode::Full {
            return config;
        }
        config.with_instructions(format!(
            "omg runs in `{mode}` mode: everything this session may call is in its tool list, \
             and nothing above the `{mode}` tier is there. To widen access, run \
             `omg config --mode <view|operate|full>` and restart the session.",
            mode = self.mode,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::fake::{FakeGocd, first_text, service, service_in};
    use crate::config::Mode;
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

    fn names_in(mode: Mode) -> Vec<String> {
        service_in(FakeGocd::replies(json!({}), None), mode).tool_names()
    }

    #[test]
    fn view_mode_exposes_reads_and_hides_every_write_tool() {
        let names = names_in(Mode::View);
        assert!(names.contains(&"get_dashboard".to_string()));
        assert!(!names.contains(&"schedule_pipeline".to_string()));
        assert!(!names.contains(&"delete_user".to_string()));
    }

    #[test]
    fn operate_mode_adds_operate_tools_but_keeps_danger_hidden() {
        let names = names_in(Mode::Operate);
        assert!(names.contains(&"schedule_pipeline".to_string()));
        assert!(!names.contains(&"delete_user".to_string()));
    }

    #[test]
    fn full_mode_exposes_every_registered_tool() {
        assert_eq!(names_in(Mode::Full).len(), 76);
    }

    #[test]
    fn a_restrictive_mode_announces_itself_in_the_server_instructions() {
        use rmcp::ServerHandler;
        let info = service_in(FakeGocd::replies(json!({}), None), Mode::View).get_info();
        let instructions = info
            .instructions
            .expect("a restrictive mode sets instructions");
        assert!(instructions.contains("view"), "got: {instructions}");
        assert!(
            instructions.contains("omg config --mode"),
            "got: {instructions}"
        );
    }

    #[test]
    fn the_full_mode_sends_no_instructions() {
        use rmcp::ServerHandler;
        let info = service_in(FakeGocd::replies(json!({}), None), Mode::Full).get_info();
        assert!(info.instructions.is_none());
    }
}
