// Artifacts Config section: /go/api/admin/config/server/artifact_config, API v1.

use super::OmgMcp;
use crate::gocd::artifacts_config as config;
use rmcp::schemars::JsonSchema;
use rmcp::{handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub(crate) struct UpdateArtifactsConfig {
    /// The current "_etag" read from get_artifacts_config; GoCD answers 412 if
    /// the config changed since.
    pub etag: String,
    /// The artifacts config object: {"artifacts_dir": ..., "purge_settings":
    /// {"purge_start_disk_space": ..., "purge_upto_disk_space": ...}}.
    pub body: Value,
}

#[tool_router(router = artifacts_config_tool_router, vis = "pub(crate)")]
impl OmgMcp {
    #[tool(
        description = "Read the server-wide artifacts config (GET /go/api/admin/config/server/artifact_config, API v1). Returns the artifacts config object — artifacts_dir and purge_settings — with `_links` removed and `_etag` included when GoCD sent one; pass that `_etag` as `etag` to update_artifacts_config. Docs: https://api.gocd.org/current/#artifacts-config"
    )]
    async fn get_artifacts_config(&self) -> CallToolResult {
        self.request_shaped(config::read()).await
    }

    #[tool(
        description = "Update the server-wide artifacts config (PUT /go/api/admin/config/server/artifact_config, API v1; the doc's POST+PUT pair is collapsed into this one tool). `body` is the artifacts config object {\"artifacts_dir\", \"purge_settings\": {\"purge_start_disk_space\", \"purge_upto_disk_space\"}} — docs: https://api.gocd.org/current/#the-artifacts-config-object. `etag` must be the current `_etag` read from get_artifacts_config — GoCD answers 412 if the config changed since it was read. Returns the updated config with a fresh `_etag` when GoCD sent one."
    )]
    async fn update_artifacts_config(
        &self,
        Parameters(args): Parameters<UpdateArtifactsConfig>,
    ) -> CallToolResult {
        self.request_shaped(config::update(&args.etag, args.body))
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::gocd::{GocdCall, GocdError};
    use crate::server::mcp::fake::{FakeGocd, first_text, service};
    use rmcp::handler::server::wrapper::Parameters;
    use serde_json::{Value, json};

    // Body and ETag taken verbatim from the GoCD API docs' get example.
    fn docs_config_body() -> Value {
        json!({
            "_links" : {
                "doc" : { "href" : "https://api.gocd.org/current/#artifacts-config" },
                "self" : { "href" : "http://ci.example.com/go/api/admin/config/server/artifact_config" }
            },
            "artifacts_dir" : "foo",
            "purge_settings" : {
                "purge_start_disk_space" : 10.0,
                "purge_upto_disk_space" : 20.0
            }
        })
    }

    #[tokio::test]
    async fn get_artifacts_config_reads_through_a_version_1_get_and_shapes_the_answer() {
        let fake = FakeGocd::replies(
            docs_config_body(),
            Some("\"17f5a9edf150884e5fc4315b4a7814cd\"".into()),
        );
        let result = service(fake.clone()).get_artifacts_config().await;

        assert_eq!(
            fake.recorded(),
            vec![GocdCall::get("api/admin/config/server/artifact_config").version(1)]
        );
        assert_ne!(result.is_error, Some(true));
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(
            shaped,
            json!({
                "artifacts_dir" : "foo",
                "purge_settings" : {
                    "purge_start_disk_space" : 10.0,
                    "purge_upto_disk_space" : 20.0
                },
                "_etag": "\"17f5a9edf150884e5fc4315b4a7814cd\""
            })
        );
    }

    #[tokio::test]
    async fn update_artifacts_config_puts_the_body_guarded_by_the_required_etag() {
        let body = json!({
            "artifacts_dir": "foobar",
            "purge_settings": {
                "purge_start_disk_space": 30,
                "purge_upto_disk_space": 60
            }
        });
        let fake = FakeGocd::replies(
            json!({ "_links": {}, "artifacts_dir": "foobar" }),
            Some("\"fresh\"".into()),
        );
        let result = service(fake.clone())
            .update_artifacts_config(Parameters(super::UpdateArtifactsConfig {
                etag: "\"17f5a9edf150884e5fc4315b4a7814cd\"".into(),
                body: body.clone(),
            }))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![
                GocdCall::put("api/admin/config/server/artifact_config")
                    .version(1)
                    .body(body)
                    .etag("\"17f5a9edf150884e5fc4315b4a7814cd\"".to_string())
            ]
        );
        assert_ne!(result.is_error, Some(true));
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(
            shaped,
            json!({ "artifacts_dir": "foobar", "_etag": "\"fresh\"" })
        );
    }

    #[tokio::test]
    async fn a_stale_update_surfaces_the_412_reread_hint() {
        let fake = FakeGocd::fails(GocdError::Http { status: 412 });
        let result = service(fake)
            .update_artifacts_config(Parameters(super::UpdateArtifactsConfig {
                etag: "\"gone\"".into(),
                body: json!({}),
            }))
            .await;

        assert_eq!(result.is_error, Some(true));
        let text = first_text(&result);
        assert!(text.contains("412"), "got: {text}");
        assert!(text.contains("_etag"), "got: {text}");
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
        for expected in ["get_artifacts_config", "update_artifacts_config"] {
            assert!(names.contains(&expected.to_string()), "{expected} missing");
        }
    }
}
