// Plugin Info section: /go/api/admin/plugin_info, API v7. Read-only — GoCD
// exposes no writes here, so no body or etag args exist.

use super::OmgMcp;
use crate::gocd::plugin_info as info;
use rmcp::schemars::JsonSchema;
use rmcp::{handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router};
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub(crate) struct PluginId {
    /// The plugin id, e.g. "cd.go.contrib.elastic-agent.docker".
    pub plugin_id: String,
}

#[tool_router(router = plugin_info_tool_router, vis = "pub(crate)")]
impl OmgMcp {
    #[tool(
        description = "List all plugins installed on the server (GET /go/api/admin/plugin_info, API v7). Returns `_embedded.plugin_info` — id, status, plugin_file_location, bundled_plugin, about and extensions per plugin — with all `_links` removed and `_etag` included when GoCD sent one. Docs: https://api.gocd.org/current/#get-all-plugin-info"
    )]
    async fn get_all_plugin_info(&self) -> CallToolResult {
        self.request_shaped(info::list()).await
    }

    #[tool(
        description = "Read one installed plugin's info by id (GET /go/api/admin/plugin_info/:id, API v7). Returns the plugin info object — id, status, plugin_file_location, bundled_plugin, about, extensions — with `_links` removed and `_etag` included when GoCD sent one. Docs: https://api.gocd.org/current/#get-plugin-info"
    )]
    async fn get_plugin_info(&self, Parameters(args): Parameters<PluginId>) -> CallToolResult {
        self.request_shaped(info::read(&args.plugin_id)).await
    }
}

#[cfg(test)]
mod tests {
    use crate::gocd::{GocdCall, GocdError};
    use crate::server::mcp::fake::{FakeGocd, first_text, service};
    use rmcp::handler::server::wrapper::Parameters;
    use serde_json::{Value, json};

    // Body and ETag taken verbatim from the GoCD API docs' get-all example.
    fn docs_plugin_info_list_body() -> Value {
        json!({
          "_links": {
            "self": { "href": "http://localhost:8153/go/api/admin/plugin_info" },
            "find": { "href": "http://localhost:8153/go/api/admin/plugin_info/:plugin_id" },
            "doc": { "href": "https://api.gocd.org/#plugin-info" }
          },
          "_embedded": {
            "plugin_info": [
              {
                "_links": {
                  "self": { "href": "https://ci.example.com/go/api/admin/plugin_info/json.config.plugin" },
                  "doc": { "href": "https://api.gocd.org/#plugin-info" },
                  "find": { "href": "https://ci.example.com/go/api/admin/plugin_info/:id" }
                },
                "id": "json.config.plugin",
                "status": { "state": "active" },
                "plugin_file_location": "/Users/varshavs/gocd/server/plugins/bundled/gocd-json-config-plugin.jar",
                "bundled_plugin": true,
                "about": {
                  "name": "JSON Configuration Plugin",
                  "version": "0.2",
                  "target_go_version": "16.1.0",
                  "description": "Configuration plugin that supports GoCD configuration in JSON",
                  "target_operating_systems": [],
                  "vendor": {
                    "name": "Tomasz Setkowski",
                    "url": "https://github.com/tomzo/gocd-json-config-plugin"
                  }
                },
                "extensions": [
                  {
                    "type": "configrepo",
                    "plugin_settings": {
                      "configurations": [
                        { "key": "pipeline_pattern", "metadata": { "secure": false, "required": false } },
                        { "key": "environment_pattern", "metadata": { "secure": false, "required": false } }
                      ],
                      "view": { "template": "Some view" }
                    }
                  }
                ]
              }
            ]
          }
        })
    }

    // Body and ETag taken verbatim from the GoCD API docs' get-one example.
    fn docs_plugin_info_body() -> Value {
        json!({
          "_links": {
            "self": { "href": "https://ci.example.com/go/api/admin/plugin_info/my_plugin" },
            "doc": { "href": "https://api.gocd.org/#plugin-info" },
            "find": { "href": "https://ci.example.com/go/api/admin/plugin_info/:id" }
          },
          "id": "my_plugin",
          "status": { "state": "active" },
          "plugin_file_location": "/path/to/server/plugins/external/my_plugin.jar",
          "bundled_plugin": false,
          "about": {
            "name": "My Plugin",
            "version": "0.2",
            "target_go_version": "16.1.0",
            "description": "Short desc",
            "target_operating_systems": [],
            "vendor": {
              "name": "GoCD contributors",
              "url": "https://github.com/tomzo/gocd-json-config-plugin"
            }
          },
          "extensions": [
            {
              "type": "configrepo",
              "plugin_settings": {
                "configurations": [
                  { "key": "pipeline_pattern", "metadata": { "secure": false, "required": false } },
                  { "key": "environment_pattern", "metadata": { "secure": false, "required": false } }
                ],
                "view": { "template": "Some view" }
              }
            }
          ]
        })
    }

    fn expected_shaped_list() -> Value {
        let mut shaped = docs_plugin_info_list_body();
        shaped.as_object_mut().unwrap().remove("_links");
        shaped["_embedded"]["plugin_info"][0]
            .as_object_mut()
            .unwrap()
            .remove("_links");
        shaped["_etag"] = json!("\"3924a894cf0e5bef02abe9de0df3bb84\"");
        shaped
    }

    fn expected_shaped_one() -> Value {
        let mut shaped = docs_plugin_info_body();
        shaped.as_object_mut().unwrap().remove("_links");
        shaped["_etag"] = json!("\"4167e3ec81fdac0fb29d854b36ceb981\"");
        shaped
    }

    #[tokio::test]
    async fn get_all_plugin_info_reads_through_a_version_7_get_and_shapes_the_answer() {
        let fake = FakeGocd::replies(
            docs_plugin_info_list_body(),
            Some("\"3924a894cf0e5bef02abe9de0df3bb84\"".into()),
        );
        let result = service(fake.clone()).get_all_plugin_info().await;

        assert_eq!(
            fake.recorded(),
            vec![GocdCall::get("api/admin/plugin_info").version(7)]
        );
        assert_ne!(result.is_error, Some(true));
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(shaped, expected_shaped_list());
    }

    #[tokio::test]
    async fn get_plugin_info_reads_one_plugin_by_id_and_shapes_the_answer() {
        let fake = FakeGocd::replies(
            docs_plugin_info_body(),
            Some("\"4167e3ec81fdac0fb29d854b36ceb981\"".into()),
        );
        let result = service(fake.clone())
            .get_plugin_info(Parameters(super::PluginId {
                plugin_id: "my_plugin".into(),
            }))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![GocdCall::get("api/admin/plugin_info/my_plugin").version(7)]
        );
        assert_ne!(result.is_error, Some(true));
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(shaped, expected_shaped_one());
    }

    #[tokio::test]
    async fn an_unauthorized_read_surfaces_the_token_hint() {
        let fake = FakeGocd::fails(GocdError::Http { status: 401 });
        let result = service(fake).get_all_plugin_info().await;

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
        for expected in ["get_all_plugin_info", "get_plugin_info"] {
            assert!(names.contains(&expected.to_string()), "{expected} missing");
        }
    }
}
