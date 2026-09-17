// Authorization Configuration section: /go/api/admin/security/auth_configs, API v2.

use super::OmgMcp;
use crate::gocd::authorization_config as auth_configs;
use rmcp::schemars::JsonSchema;
use rmcp::{handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub(crate) struct AuthConfigId {
    /// The authorization configuration id, e.g. "ldap".
    pub auth_config_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub(crate) struct WriteAuthConfig {
    /// The authorization configuration object: {"id": ..., "plugin_id": ...,
    /// "allow_only_known_users_to_login": ..., "properties":
    /// [{"key": ..., "value": ...}]}.
    pub body: Value,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub(crate) struct UpdateAuthConfig {
    /// The authorization configuration id to replace, e.g. "ldap".
    pub auth_config_id: String,
    /// The current "_etag" read from get_auth_config; GoCD answers 412 if the
    /// config changed since.
    pub etag: String,
    /// The full authorization configuration object: {"plugin_id": ...,
    /// "allow_only_known_users_to_login": ..., "properties":
    /// [{"key": ..., "value": ...}]}.
    pub body: Value,
}

#[tool_router(router = authorization_config_tool_router, vis = "pub(crate)")]
impl OmgMcp {
    #[tool(
        description = "List every configured authorization configuration (GET /go/api/admin/security/auth_configs, API v2). Returns `_embedded.auth_configs` — id, plugin_id, allow_only_known_users_to_login and properties per config — with all `_links` removed. Docs: https://api.gocd.org/current/#authorization-configuration"
    )]
    async fn get_auth_configs(&self) -> CallToolResult {
        self.request_shaped(auth_configs::list()).await
    }

    #[tool(
        description = "Read one authorization configuration by id (GET /go/api/admin/security/auth_configs/:auth_config_id, API v2). Returns the authorization configuration object — id, plugin_id, allow_only_known_users_to_login, properties — with `_links` removed and `_etag` included when GoCD sent one; pass that `_etag` as `etag` to update_auth_config. Docs: https://api.gocd.org/current/#authorization-configuration"
    )]
    async fn get_auth_config(&self, Parameters(args): Parameters<AuthConfigId>) -> CallToolResult {
        self.request_shaped(auth_configs::read(&args.auth_config_id))
            .await
    }

    #[tool(
        description = "Create an authorization configuration (POST /go/api/admin/security/auth_configs, API v2). `body` is the authorization configuration object {\"id\", \"plugin_id\", \"allow_only_known_users_to_login\", \"properties\": [{\"key\", \"value\"}]} — docs: https://api.gocd.org/current/#authorization-configuration. Returns the created config, with secret properties replaced by `encrypted_value`, `_links` removed and `_etag` included when GoCD sent one."
    )]
    async fn create_auth_config(
        &self,
        Parameters(args): Parameters<WriteAuthConfig>,
    ) -> CallToolResult {
        self.request_shaped(auth_configs::create(args.body)).await
    }

    #[tool(
        description = "Replace an authorization configuration (PUT /go/api/admin/security/auth_configs/:auth_config_id, API v2). `body` is the full authorization configuration object; `etag` must be the current `_etag` read from get_auth_config — GoCD answers 412 if the config changed since it was read. Returns the updated config with a fresh `_etag` when GoCD sent one. Docs: https://api.gocd.org/current/#authorization-configuration"
    )]
    async fn update_auth_config(
        &self,
        Parameters(args): Parameters<UpdateAuthConfig>,
    ) -> CallToolResult {
        self.request_shaped(auth_configs::update(
            &args.auth_config_id,
            &args.etag,
            args.body,
        ))
        .await
    }

    #[tool(
        description = "Delete an authorization configuration (DELETE /go/api/admin/security/auth_configs/:auth_config_id, API v2). Returns GoCD's deletion message. Docs: https://api.gocd.org/current/#authorization-configuration"
    )]
    async fn delete_auth_config(
        &self,
        Parameters(args): Parameters<AuthConfigId>,
    ) -> CallToolResult {
        self.request_shaped(auth_configs::remove(&args.auth_config_id))
            .await
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
                "self": { "href": "https://ci.example.com/go/api/admin/security/auth_configs" },
                "doc": { "href": "https://api.gocd.org/#authorization-configuration" },
                "find": { "href": "https://ci.example.com/go/api/admin/security/auth_configs/:auth_config_id" }
            },
            "_embedded": {
                "auth_configs": [ {
                    "_links": {
                        "self": { "href": "https://ci.example.com/go/api/admin/security/auth_configs/ldap" }
                    },
                    "id": "ldap",
                    "plugin_id": "cd.go.authentication.ldap",
                    "allow_only_known_users_to_login": false,
                    "properties": [ { "key": "Url", "value": "ldap://ldap.server.url" } ]
                } ]
            }
        })
    }

    // Single-config body and ETag taken verbatim from the GoCD API docs' example.
    fn docs_single_config_body() -> Value {
        json!({
            "_links": {
                "self": { "href": "https://ci.example.com/go/api/admin/security/auth_configs/ldap" }
            },
            "id": "ldap",
            "plugin_id": "cd.go.authentication.ldap",
            "allow_only_known_users_to_login": false,
            "properties": [
                { "key": "Url", "value": "ldap://ldap.server.url:389" },
                { "key": "SearchBase", "value": "ou=users,ou=system" },
                { "key": "Password", "encrypted_value": "GLzARJ+Qr+M=" }
            ]
        })
    }

    fn shaped_single_config(etag: &str) -> Value {
        json!({
            "id": "ldap",
            "plugin_id": "cd.go.authentication.ldap",
            "allow_only_known_users_to_login": false,
            "properties": [
                { "key": "Url", "value": "ldap://ldap.server.url:389" },
                { "key": "SearchBase", "value": "ou=users,ou=system" },
                { "key": "Password", "encrypted_value": "GLzARJ+Qr+M=" }
            ],
            "_etag": etag
        })
    }

    #[tokio::test]
    async fn get_auth_configs_lists_through_a_version_2_get_and_shapes_the_answer() {
        let fake = FakeGocd::replies(stored_list_body(), None);
        let result = service(fake.clone()).get_auth_configs().await;

        assert_eq!(
            fake.recorded(),
            vec![GocdCall::get("api/admin/security/auth_configs").version(2)]
        );
        assert_ne!(result.is_error, Some(true));
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(
            shaped,
            json!({
                "_embedded": {
                    "auth_configs": [ {
                        "id": "ldap",
                        "plugin_id": "cd.go.authentication.ldap",
                        "allow_only_known_users_to_login": false,
                        "properties": [ { "key": "Url", "value": "ldap://ldap.server.url" } ]
                    } ]
                }
            })
        );
    }

    #[tokio::test]
    async fn get_auth_config_reads_one_config_by_id_and_surfaces_its_etag() {
        let fake = FakeGocd::replies(
            docs_single_config_body(),
            Some("\"cbc5f2d5b9c13a2cc1b1efb3d8a6155d\"".into()),
        );
        let result = service(fake.clone())
            .get_auth_config(Parameters(super::AuthConfigId {
                auth_config_id: "ldap".into(),
            }))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![GocdCall::get("api/admin/security/auth_configs/ldap").version(2)]
        );
        assert_ne!(result.is_error, Some(true));
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(
            shaped,
            shaped_single_config("\"cbc5f2d5b9c13a2cc1b1efb3d8a6155d\"")
        );
    }

    #[tokio::test]
    async fn create_auth_config_posts_the_given_object_as_json() {
        let body = json!({
            "id": "ldap",
            "plugin_id": "cd.go.authentication.ldap",
            "allow_only_known_users_to_login": false,
            "properties": [ { "key": "Url", "value": "ldap://ldap.server.url:389" } ]
        });
        let fake = FakeGocd::replies(
            docs_single_config_body(),
            Some("\"cbc5f2d5b9c13a2cc1b1efb3d8a6155d\"".into()),
        );
        let result = service(fake.clone())
            .create_auth_config(Parameters(super::WriteAuthConfig { body: body.clone() }))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![
                GocdCall::post("api/admin/security/auth_configs")
                    .version(2)
                    .body(body)
            ]
        );
        assert_ne!(result.is_error, Some(true));
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(
            shaped,
            shaped_single_config("\"cbc5f2d5b9c13a2cc1b1efb3d8a6155d\"")
        );
    }

    #[tokio::test]
    async fn update_auth_config_puts_the_body_guarded_by_the_required_etag() {
        let body = json!({
            "plugin_id": "cd.go.authentication.ldap",
            "allow_only_known_users_to_login": false,
            "properties": [ { "key": "Url", "value": "ldap://ldap.server.url:10389" } ]
        });
        let fake = FakeGocd::replies(docs_single_config_body(), Some("\"fresh\"".into()));
        let result = service(fake.clone())
            .update_auth_config(Parameters(super::UpdateAuthConfig {
                auth_config_id: "ldap".into(),
                etag: "\"cbc5f2d5b9c13a2cc1b1efb3d8a6155d\"".into(),
                body: body.clone(),
            }))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![
                GocdCall::put("api/admin/security/auth_configs/ldap")
                    .version(2)
                    .body(body)
                    .etag("\"cbc5f2d5b9c13a2cc1b1efb3d8a6155d\"".to_string())
            ]
        );
        assert_ne!(result.is_error, Some(true));
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(shaped, shaped_single_config("\"fresh\""));
    }

    #[tokio::test]
    async fn a_stale_update_surfaces_the_412_reread_hint() {
        let fake = FakeGocd::fails(GocdError::Http { status: 412 });
        let result = service(fake)
            .update_auth_config(Parameters(super::UpdateAuthConfig {
                auth_config_id: "ldap".into(),
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
    async fn an_unauthorized_read_surfaces_the_token_hint() {
        let fake = FakeGocd::fails(GocdError::Http { status: 401 });
        let result = service(fake).get_auth_configs().await;

        assert_eq!(result.is_error, Some(true));
        let text = first_text(&result);
        assert!(text.contains("401"), "got: {text}");
        assert!(text.contains("token"), "got: {text}");
    }

    #[tokio::test]
    async fn delete_auth_config_deletes_by_id_and_returns_the_message() {
        let fake = FakeGocd::replies(
            json!({ "message": "The security auth config 'ldap' was deleted successfully." }),
            None,
        );
        let result = service(fake.clone())
            .delete_auth_config(Parameters(super::AuthConfigId {
                auth_config_id: "ldap".into(),
            }))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![GocdCall::delete("api/admin/security/auth_configs/ldap").version(2)]
        );
        assert_ne!(result.is_error, Some(true));
        assert!(first_text(&result).contains("deleted successfully"));
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
        for expected in [
            "create_auth_config",
            "delete_auth_config",
            "get_auth_config",
            "get_auth_configs",
            "update_auth_config",
        ] {
            assert!(names.contains(&expected.to_string()), "{expected} missing");
        }
    }
}
