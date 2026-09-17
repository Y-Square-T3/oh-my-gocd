// Stages section: /go/api/stages/:pipeline_name/:pipeline_counter/:stage_name/
// run, API v2. GoCD documents no If-Match-gated write here, so no tool takes
// an etag arg; the bodyless POST carries GoCD's documented X-GoCD-Confirm.

use super::OmgMcp;
use crate::gocd::stages;
use rmcp::schemars::JsonSchema;
use rmcp::{handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router};
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub(crate) struct StageRunPath {
    /// The pipeline name, e.g. "myPipeline".
    pub pipeline_name: String,
    /// The pipeline counter, e.g. "1".
    pub pipeline_counter: String,
    /// The stage name, e.g. "myStages".
    pub stage_name: String,
}

#[tool_router(router = stages_tool_router, vis = "pub(crate)")]
impl OmgMcp {
    #[tool(
        description = "Trigger a stage against an existing pipeline instance (POST /go/api/stages/:pipeline_name/:pipeline_counter/:stage_name/run, API v2). Sent exactly as documented: no request body, with the X-GoCD-Confirm header; GoCD answers 202 and documents no If-Match guard here. Returns {\"message\": ...} with `_links` removed and `_etag` included when GoCD sent one. Docs: https://api.gocd.org/current/#run-stage"
    )]
    async fn run_stage(&self, Parameters(args): Parameters<StageRunPath>) -> CallToolResult {
        self.request_shaped(stages::run(
            &args.pipeline_name,
            &args.pipeline_counter,
            &args.stage_name,
        ))
        .await
    }
}

#[cfg(test)]
mod tests {
    use crate::gocd::{GocdCall, GocdError};
    use crate::server::mcp::fake::{FakeGocd, first_text, service};
    use rmcp::handler::server::wrapper::Parameters;
    use serde_json::json;

    fn run_args() -> super::StageRunPath {
        super::StageRunPath {
            pipeline_name: "myPipeline".into(),
            pipeline_counter: "42".into(),
            stage_name: "myStages".into(),
        }
    }

    #[tokio::test]
    async fn run_stage_posts_a_confirmed_bodyless_request_and_returns_the_message() {
        let fake = FakeGocd::replies(
            json!({ "message": "Request to schedule stage testAPI/1/defaultStage accepted" }),
            None,
        );
        let result = service(fake.clone())
            .run_stage(Parameters(run_args()))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![
                GocdCall::post("api/stages/myPipeline/42/myStages/run")
                    .version(2)
                    .confirm()
            ]
        );
        assert_ne!(result.is_error, Some(true));
        assert!(first_text(&result).contains("Request to schedule stage testAPI/1/defaultStage"));
    }

    #[tokio::test]
    async fn an_unauthorized_run_surfaces_the_token_hint() {
        let fake = FakeGocd::fails(GocdError::Http { status: 401 });
        let result = service(fake).run_stage(Parameters(run_args())).await;

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
            .find(|t| t.name == "run_stage")
            .expect("run_stage registered")
            .description
            .map(|d| d.to_string())
            .unwrap_or_default();

        assert!(
            description
                .contains("POST /go/api/stages/:pipeline_name/:pipeline_counter/:stage_name/run")
        );
        assert!(description.contains("API v2"));
        assert!(description.contains("#run-stage"));
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
        assert!(names.contains(&"run_stage".to_string()));
    }
}
