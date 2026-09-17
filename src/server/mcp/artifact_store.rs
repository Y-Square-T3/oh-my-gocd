// Artifact Store section: /go/api/admin/artifact_stores, API v1.

use super::OmgMcp;
use crate::gocd::artifact_store as stores;
use rmcp::schemars::JsonSchema;
use rmcp::{handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub(crate) struct StoreId {
    /// The artifact store id, e.g. "docker".
    pub store_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub(crate) struct WriteStore {
    /// The artifact store object: {"id": ..., "plugin_id": ...,
    /// "properties": [{"key": ..., "value": ...}]}.
    pub body: Value,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub(crate) struct UpdateStore {
    /// The artifact store id to replace, e.g. "docker".
    pub store_id: String,
    /// The current "_etag" read from get_artifact_store; GoCD answers 412 if
    /// the store changed since.
    pub etag: String,
    /// The full artifact store object: {"id": ..., "plugin_id": ...,
    /// "properties": [{"key": ..., "value": ...}]}.
    pub body: Value,
}

#[tool_router(router = artifact_store_tool_router, vis = "pub(crate)")]
impl OmgMcp {
    #[tool(
        description = "List every configured artifact store (GET /go/api/admin/artifact_stores, API v1). Returns `_embedded.artifact_stores` — id, plugin_id and properties per store — with all `_links` removed. Docs: https://api.gocd.org/current/#artifact-store"
    )]
    async fn get_artifact_stores(&self) -> CallToolResult {
        self.request_shaped(stores::list()).await
    }

    #[tool(
        description = "Read one artifact store by id (GET /go/api/admin/artifact_stores/:storeId, API v1). Returns the artifact store object — id, plugin_id, properties — with `_links` removed and `_etag` included when GoCD sent one; pass that `_etag` as `etag` to update_artifact_store."
    )]
    async fn get_artifact_store(&self, Parameters(args): Parameters<StoreId>) -> CallToolResult {
        self.request_shaped(stores::read(&args.store_id)).await
    }

    #[tool(
        description = "Create an artifact store (POST /go/api/admin/artifact_stores, API v1). `body` is the artifact store object {\"id\", \"plugin_id\", \"properties\": [{\"key\", \"value\"}]} — docs: https://api.gocd.org/current/#the-artifact-store-object. Returns the created store, with its secrets replaced by `encrypted_value`, `_links` removed and `_etag` included when GoCD sent one."
    )]
    async fn create_artifact_store(
        &self,
        Parameters(args): Parameters<WriteStore>,
    ) -> CallToolResult {
        self.request_shaped(stores::create(args.body)).await
    }

    #[tool(
        description = "Replace an artifact store (PUT /go/api/admin/artifact_stores/:storeId, API v1). `body` is the full artifact store object; `etag` must be the current `_etag` read from get_artifact_store — GoCD answers 412 if the store changed since it was read. Returns the updated store with a fresh `_etag` when GoCD sent one."
    )]
    async fn update_artifact_store(
        &self,
        Parameters(args): Parameters<UpdateStore>,
    ) -> CallToolResult {
        self.request_shaped(stores::update(&args.store_id, &args.etag, args.body))
            .await
    }

    #[tool(
        description = "Delete an artifact store (DELETE /go/api/admin/artifact_stores/:storeId, API v1). Returns GoCD's deletion message."
    )]
    async fn delete_artifact_store(&self, Parameters(args): Parameters<StoreId>) -> CallToolResult {
        self.request_shaped(stores::remove(&args.store_id)).await
    }
}

#[cfg(test)]
mod tests {
    use crate::gocd::{GocdCall, GocdError};
    use crate::server::mcp::fake::{FakeGocd, first_text, service};
    use rmcp::handler::server::wrapper::Parameters;
    use serde_json::{Value, json};

    // Body shape taken verbatim from the GoCD API docs' list example.
    fn stored_list_body() -> Value {
        json!({
            "_links": {
                "self": { "href": "https://ci.example.com/go/api/admin/artifact_stores" },
                "doc": { "href": "https://api.gocd.org/current/#artifact_stores" },
                "find": { "href": "https://ci.example.com/go/api/admin/artifact_stores/:id" }
            },
            "_embedded": {
                "artifact_stores": [ {
                    "_links": {
                        "self": { "href": "https://ci.example.com/go/api/admin/artifact_stores/hub.docker" }
                    },
                    "id" : "hub.docker",
                    "plugin_id" : "cd.go.artifact.docker.registry",
                    "properties" : [ { "key" : "RegistryURL", "value" : "https://your_docker_registry_url" } ]
                } ]
            }
        })
    }

    #[tokio::test]
    async fn get_artifact_stores_lists_through_a_version_1_get_and_shapes_the_answer() {
        let fake = FakeGocd::replies(stored_list_body(), None);
        let result = service(fake.clone()).get_artifact_stores().await;

        assert_eq!(
            fake.recorded(),
            vec![GocdCall::get("api/admin/artifact_stores").version(1)]
        );
        assert_ne!(result.is_error, Some(true));
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(
            shaped,
            json!({
                "_embedded": {
                    "artifact_stores": [ {
                        "id" : "hub.docker",
                        "plugin_id" : "cd.go.artifact.docker.registry",
                        "properties" : [ { "key" : "RegistryURL", "value" : "https://your_docker_registry_url" } ]
                    } ]
                }
            })
        );
    }

    #[tokio::test]
    async fn get_artifact_store_reads_one_store_by_id_and_surfaces_its_etag() {
        // Body and ETag taken verbatim from the GoCD API docs' single-store example.
        let fake = FakeGocd::replies(
            json!({
                "_links": { "self": { "href": "https://ci.example.com/go/api/admin/artifact_stores/hub.docker" } },
                "id" : "hub.docker",
                "plugin_id" : "cd.go.artifact.docker.registry",
                "properties" : [ { "key" : "RegistryURL", "value" : "https://your_docker_registry_url" } ]
            }),
            Some("\"fff30fd05db389acded4e993c1ecd5a4\"".into()),
        );
        let result = service(fake.clone())
            .get_artifact_store(Parameters(super::StoreId {
                store_id: "hub.docker".into(),
            }))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![GocdCall::get("api/admin/artifact_stores/hub.docker").version(1)]
        );
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(
            shaped,
            json!({
                "id" : "hub.docker",
                "plugin_id" : "cd.go.artifact.docker.registry",
                "properties" : [ { "key" : "RegistryURL", "value" : "https://your_docker_registry_url" } ],
                "_etag": "\"fff30fd05db389acded4e993c1ecd5a4\""
            })
        );
    }

    #[tokio::test]
    async fn create_artifact_store_posts_the_given_object_as_json() {
        let body = json!({
            "id": "docker",
            "plugin_id": "cd.go.artifact.docker.registry",
            "properties": [ { "key": "RegistryURL", "value": "https://registry" } ]
        });
        let fake = FakeGocd::replies(
            json!({ "_links": {}, "id": "docker", "plugin_id": "cd.go.artifact.docker.registry" }),
            Some("\"e239974f09be2d88565c584c01ba0954\"".into()),
        );
        let result = service(fake.clone())
            .create_artifact_store(Parameters(super::WriteStore { body: body.clone() }))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![
                GocdCall::post("api/admin/artifact_stores")
                    .version(1)
                    .body(body)
            ]
        );
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(
            shaped,
            json!({
                "id": "docker",
                "plugin_id": "cd.go.artifact.docker.registry",
                "_etag": "\"e239974f09be2d88565c584c01ba0954\""
            })
        );
    }

    #[tokio::test]
    async fn update_artifact_store_puts_the_body_guarded_by_the_required_etag() {
        let body = json!({ "id": "docker", "plugin_id": "cd.go.artifact.docker.registry" });
        let fake = FakeGocd::replies(json!({ "id": "docker" }), Some("\"new\"".into()));
        let result = service(fake.clone())
            .update_artifact_store(Parameters(super::UpdateStore {
                store_id: "docker".into(),
                etag: "\"e239974f09be2d88565c584c01ba0954\"".into(),
                body: body.clone(),
            }))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![
                GocdCall::put("api/admin/artifact_stores/docker")
                    .version(1)
                    .body(body)
                    .etag("\"e239974f09be2d88565c584c01ba0954\"".to_string())
            ]
        );
        assert_ne!(result.is_error, Some(true));
    }

    #[tokio::test]
    async fn a_stale_update_surfaces_the_412_reread_hint() {
        let fake = FakeGocd::fails(GocdError::Http { status: 412 });
        let result = service(fake)
            .update_artifact_store(Parameters(super::UpdateStore {
                store_id: "docker".into(),
                etag: "\"gone\"".into(),
                body: json!({}),
            }))
            .await;

        assert_eq!(result.is_error, Some(true));
        let text = first_text(&result);
        assert!(text.contains("412"), "got: {text}");
        assert!(text.contains("_etag"), "got: {text}");
    }

    #[tokio::test]
    async fn delete_artifact_store_deletes_by_id_and_returns_the_message() {
        let fake = FakeGocd::replies(
            json!({ "message": "The artifactStore 'docker' was deleted successfully." }),
            None,
        );
        let result = service(fake.clone())
            .delete_artifact_store(Parameters(super::StoreId {
                store_id: "docker".into(),
            }))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![GocdCall::delete("api/admin/artifact_stores/docker").version(1)]
        );
        assert_ne!(result.is_error, Some(true));
        assert!(first_text(&result).contains("deleted successfully"));
    }

    #[test]
    fn the_section_tools_are_merged_into_the_service_router() {
        let service = service(FakeGocd::replies(json!({}), None));
        let mut names: Vec<String> = service
            .tool_router
            .list_all()
            .into_iter()
            .map(|t| t.name.to_string())
            .collect();
        names.sort();
        assert_eq!(
            names,
            vec![
                "create_artifact_store",
                "delete_artifact_store",
                "get_artifact_store",
                "get_artifact_stores",
                "get_current_user",
                "update_artifact_store"
            ]
        );
    }
}
