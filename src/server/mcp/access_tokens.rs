// Access Tokens section: /go/api/current_user/access_tokens and
// /go/api/admin/access_tokens, API v1. Token creation is deliberately not
// exposed (Basic-auth-only, credential-minting risk — spec #1). GoCD
// documents no If-Match on the revoke POSTs, so — as on the Users section —
// the write tools take no `etag` argument and send none.

use super::OmgMcp;
use crate::gocd::access_tokens;
use rmcp::schemars::JsonSchema;
use rmcp::{handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub(crate) struct AccessTokenId {
    /// The numeric id of the access token, e.g. "42".
    pub token_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub(crate) struct RevokeAccessToken {
    /// The numeric id of the access token, e.g. "42".
    pub token_id: String,
    /// The revoke request object: {"revoke_cause": "<text>"}.
    pub body: Value,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub(crate) struct AdminAccessTokensQuery {
    /// Which tokens to list: "all", "active" or "revoked". Unset means GoCD's
    /// default of active tokens only.
    pub filter: Option<String>,
}

#[tool_router(router = access_tokens_tool_router, vis = "pub(crate)")]
impl OmgMcp {
    #[tool(
        description = "List the access tokens of the current user (GET /go/api/current_user/access_tokens, API v1). Returns `_embedded.access_tokens` — id, description, username, revoked, revoke_cause, revoked_by, revoked_at, created_at, last_used_at — with all `_links` removed. Token creation is deliberately not exposed. Docs: https://api.gocd.org/current/#get-all-tokens-for-current-user"
    )]
    async fn get_current_user_access_tokens(&self) -> CallToolResult {
        self.request_shaped(access_tokens::current_user_list())
            .await
    }

    #[tool(
        description = "Read one access token of the current user by id (GET /go/api/current_user/access_tokens/:id, API v1). Returns the access token object — id, description, username, revoked, revoke_cause, revoked_by, revoked_at, created_at, last_used_at — with all `_links` removed and `_etag` included when GoCD sent one. Docs: https://api.gocd.org/current/#get-one-token-for-current-user"
    )]
    async fn get_current_user_access_token(
        &self,
        Parameters(args): Parameters<AccessTokenId>,
    ) -> CallToolResult {
        self.request_shaped(access_tokens::current_user_read(&args.token_id))
            .await
    }

    #[tool(
        description = "Revoke one access token of the current user so it cannot be reused (POST /go/api/current_user/access_tokens/:id/revoke, API v1). `body` is {\"revoke_cause\": \"<text>\"}. Requires the user's own credentials — GoCD documents no If-Match guard here. Returns the revoked access token object with `_links` removed. Docs: https://api.gocd.org/current/#revoke-token-for-current-user"
    )]
    async fn revoke_current_user_access_token(
        &self,
        Parameters(args): Parameters<RevokeAccessToken>,
    ) -> CallToolResult {
        self.request_shaped(access_tokens::current_user_revoke(
            &args.token_id,
            args.body,
        ))
        .await
    }

    #[tool(
        description = "List access tokens belonging to all users, for system administrators (GET /go/api/admin/access_tokens, API v1). Optional query arg `filter` takes \"all\", \"active\" or \"revoked\"; unset means GoCD's default of active tokens only and is omitted from the request. Returns `_embedded.access_tokens` — including revoked_because_user_deleted — with all `_links` removed. Token creation is deliberately not exposed. Docs: https://api.gocd.org/current/#get-all-tokens-for-all-users"
    )]
    async fn get_admin_access_tokens(
        &self,
        Parameters(args): Parameters<AdminAccessTokensQuery>,
    ) -> CallToolResult {
        let mut query = Vec::new();
        if let Some(filter) = args.filter {
            query.push(("filter".to_string(), filter));
        }
        self.request_shaped(access_tokens::admin_list(query)).await
    }

    #[tool(
        description = "Read one access token belonging to any user by id, for system administrators (GET /go/api/admin/access_tokens/:id, API v1). Returns the access token object — id, description, username, revoked, revoke_cause, revoked_by, revoked_at, created_at, last_used_at — with all `_links` removed and `_etag` included when GoCD sent one. Docs: https://api.gocd.org/current/#get-one-token-for-any-user"
    )]
    async fn get_admin_access_token(
        &self,
        Parameters(args): Parameters<AccessTokenId>,
    ) -> CallToolResult {
        self.request_shaped(access_tokens::admin_read(&args.token_id))
            .await
    }

    #[tool(
        description = "Revoke one access token belonging to any user so it cannot be reused, for system administrators (POST /go/api/admin/access_tokens/:id/revoke, API v1). `body` is {\"revoke_cause\": \"<text>\"}. Requires admin rights — GoCD documents no If-Match guard here. Returns the revoked access token object with `_links` removed. Docs: https://api.gocd.org/current/#revoke-token-for-any-user"
    )]
    async fn revoke_admin_access_token(
        &self,
        Parameters(args): Parameters<RevokeAccessToken>,
    ) -> CallToolResult {
        self.request_shaped(access_tokens::admin_revoke(&args.token_id, args.body))
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::gocd::{GocdCall, GocdError};
    use crate::server::mcp::fake::{FakeGocd, first_text, service};
    use rmcp::handler::server::wrapper::Parameters;
    use serde_json::{Value, json};

    // List body taken verbatim from the GoCD API docs'
    // get-all-tokens-for-current-user example.
    fn docs_current_user_list_body() -> Value {
        json!({
            "_links": {
                "self": { "href": "https://ci.example.com/go/api/current_user/access_tokens" },
                "doc": { "href": "https://api.gocd.org/19.2.0/#access-token" }
            },
            "_embedded": {
                "access_tokens": [ {
                    "_links": {
                        "self": { "href": "https://ci.example.com/go/api/current_user/access_tokens/42" },
                        "doc": { "href": "https://api.gocd.org/19.2.0/#access-token" },
                        "find": { "href": "https://ci.example.com/go/api/current_user/access_tokens/:id" }
                    },
                    "id": 42,
                    "description": "my first token",
                    "username": "username",
                    "revoked": false,
                    "revoke_cause": null,
                    "revoked_by": null,
                    "revoked_at": null,
                    "created_at": "2019-02-20T10:45:10Z",
                    "last_used_at": null
                } ]
            }
        })
    }

    fn shaped_current_user_list() -> Value {
        json!({
            "_embedded": {
                "access_tokens": [ {
                    "id": 42,
                    "description": "my first token",
                    "username": "username",
                    "revoked": false,
                    "revoke_cause": null,
                    "revoked_by": null,
                    "revoked_at": null,
                    "created_at": "2019-02-20T10:45:10Z",
                    "last_used_at": null
                } ]
            }
        })
    }

    // Single-token body taken verbatim from the docs'
    // get-one-token-for-current-user example.
    fn docs_current_user_token_body() -> Value {
        json!({
            "_links": {
                "self": { "href": "https://ci.example.com/go/api/current_user/access_tokens/42" },
                "doc": { "href": "https://api.gocd.org/19.2.0/#access-token" },
                "find": { "href": "https://ci.example.com/go/api/current_user/access_tokens/:id" }
            },
            "id": 42,
            "description": "my first token",
            "username": "username",
            "revoked": false,
            "revoke_cause": null,
            "revoked_by": null,
            "revoked_at": null,
            "created_at": "2019-02-20T10:45:10Z",
            "last_used_at": null
        })
    }

    fn shaped_current_user_token(etag: &str) -> Value {
        json!({
            "id": 42,
            "description": "my first token",
            "username": "username",
            "revoked": false,
            "revoke_cause": null,
            "revoked_by": null,
            "revoked_at": null,
            "created_at": "2019-02-20T10:45:10Z",
            "last_used_at": null,
            "_etag": etag
        })
    }

    // Revoke reply taken verbatim from the docs' revoke-token-for-current-user
    // example.
    fn docs_current_user_revoked_body() -> Value {
        json!({
            "_links": {
                "self": { "href": "https://ci.example.com/go/api/current_user/access_tokens/42" },
                "doc": { "href": "https://api.gocd.org/19.2.0/#access-token" },
                "find": { "href": "https://ci.example.com/go/api/current_user/access_tokens/:id" }
            },
            "id": 42,
            "description": "token was compromised",
            "username": "username",
            "revoked": true,
            "revoke_cause": "foo",
            "revoked_by": "jez",
            "revoked_at": "2019-02-20T11:25:03Z",
            "created_at": "2019-02-20T10:45:10Z",
            "last_used_at": null
        })
    }

    fn shaped_current_user_revoked() -> Value {
        json!({
            "id": 42,
            "description": "token was compromised",
            "username": "username",
            "revoked": true,
            "revoke_cause": "foo",
            "revoked_by": "jez",
            "revoked_at": "2019-02-20T11:25:03Z",
            "created_at": "2019-02-20T10:45:10Z",
            "last_used_at": null
        })
    }

    // Admin list body taken verbatim from the docs'
    // get-all-tokens-for-all-users filter=all example.
    fn docs_admin_list_body() -> Value {
        json!({
            "_links": {
                "self": { "href": "https://ci.example.com/go/api/admin/access_tokens" },
                "doc": { "href": "https://api.gocd.org/19.2.0/#access-token" }
            },
            "_embedded": {
                "access_tokens": [ {
                    "_links": {
                        "self": { "href": "https://ci.example.com/go/api/admin/access_tokens/42" }
                    },
                    "id": 42,
                    "description": "dummy access token",
                    "username": "username",
                    "revoked": false,
                    "revoke_cause": null,
                    "revoked_by": null,
                    "revoked_at": null,
                    "created_at": "2019-03-06T05:02:24Z",
                    "last_used_at": null,
                    "revoked_because_user_deleted": false
                },
                {
                    "_links": {
                        "self": { "href": "https://ci.example.com/go/api/admin/access_tokens/43" }
                    },
                    "id": 43,
                    "description": "another access token",
                    "username": "username",
                    "revoked": true,
                    "revoke_cause": "revoke cause",
                    "revoked_by": "username",
                    "revoked_at": "2019-03-07T05:02:45Z",
                    "created_at": "2019-03-06T05:02:34Z",
                    "last_used_at": null,
                    "revoked_because_user_deleted": false
                } ]
            }
        })
    }

    fn shaped_admin_list() -> Value {
        json!({
            "_embedded": {
                "access_tokens": [ {
                    "id": 42,
                    "description": "dummy access token",
                    "username": "username",
                    "revoked": false,
                    "revoke_cause": null,
                    "revoked_by": null,
                    "revoked_at": null,
                    "created_at": "2019-03-06T05:02:24Z",
                    "last_used_at": null,
                    "revoked_because_user_deleted": false
                },
                {
                    "id": 43,
                    "description": "another access token",
                    "username": "username",
                    "revoked": true,
                    "revoke_cause": "revoke cause",
                    "revoked_by": "username",
                    "revoked_at": "2019-03-07T05:02:45Z",
                    "created_at": "2019-03-06T05:02:34Z",
                    "last_used_at": null,
                    "revoked_because_user_deleted": false
                } ]
            }
        })
    }

    // Single admin token body taken verbatim from the docs'
    // get-one-token-for-any-user example.
    fn docs_admin_token_body() -> Value {
        json!({
            "_links": {
                "self": { "href": "https://ci.example.com/go/api/admin/access_tokens/42" },
                "doc": { "href": "https://api.gocd.org/19.2.0/#access-token" },
                "find": { "href": "https://ci.example.com/go/api/admin/access_tokens/:id" }
            },
            "id": 42,
            "description": "my first token",
            "username": "username",
            "revoked": false,
            "revoke_cause": null,
            "revoked_by": null,
            "revoked_at": null,
            "created_at": "2019-02-20T10:45:10Z",
            "last_used_at": null
        })
    }

    fn shaped_admin_token() -> Value {
        json!({
            "id": 42,
            "description": "my first token",
            "username": "username",
            "revoked": false,
            "revoke_cause": null,
            "revoked_by": null,
            "revoked_at": null,
            "created_at": "2019-02-20T10:45:10Z",
            "last_used_at": null
        })
    }

    // Revoke reply taken verbatim from the docs' revoke-token-for-any-user
    // example.
    fn docs_admin_revoked_body() -> Value {
        json!({
            "_links": {
                "self": { "href": "https://ci.example.com/go/api/admin/access_tokens/42" },
                "doc": { "href": "https://api.gocd.org/19.2.0/#access-token" },
                "find": { "href": "https://ci.example.com/go/api/admin/access_tokens/:id" }
            },
            "id": 42,
            "description": "token was compromised",
            "username": "username",
            "revoked": true,
            "revoke_cause": "foo",
            "revoked_by": "jez",
            "revoked_at": "2019-02-20T11:25:03Z",
            "created_at": "2019-02-20T10:45:10Z",
            "last_used_at": null
        })
    }

    fn shaped_admin_revoked() -> Value {
        json!({
            "id": 42,
            "description": "token was compromised",
            "username": "username",
            "revoked": true,
            "revoke_cause": "foo",
            "revoked_by": "jez",
            "revoked_at": "2019-02-20T11:25:03Z",
            "created_at": "2019-02-20T10:45:10Z",
            "last_used_at": null
        })
    }

    #[tokio::test]
    async fn get_current_user_access_tokens_lists_through_a_version_1_get_and_shapes_the_answer() {
        let fake = FakeGocd::replies(docs_current_user_list_body(), None);
        let result = service(fake.clone()).get_current_user_access_tokens().await;

        assert_eq!(
            fake.recorded(),
            vec![GocdCall::get("api/current_user/access_tokens").version(1)]
        );
        assert_ne!(result.is_error, Some(true));
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(shaped, shaped_current_user_list());
    }

    #[tokio::test]
    async fn get_current_user_access_token_reads_by_id_and_injects_a_sent_etag() {
        let fake = FakeGocd::replies(docs_current_user_token_body(), Some("\"deadbeef\"".into()));
        let result = service(fake.clone())
            .get_current_user_access_token(Parameters(super::AccessTokenId {
                token_id: "42".into(),
            }))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![GocdCall::get("api/current_user/access_tokens/42").version(1)]
        );
        assert_ne!(result.is_error, Some(true));
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(shaped, shaped_current_user_token("\"deadbeef\""));
    }

    #[tokio::test]
    async fn get_current_user_access_token_keeps_the_body_free_of_etag_when_go_cd_sends_none() {
        let fake = FakeGocd::replies(docs_current_user_token_body(), None);
        let result = service(fake.clone())
            .get_current_user_access_token(Parameters(super::AccessTokenId {
                token_id: "42".into(),
            }))
            .await;

        assert_ne!(result.is_error, Some(true));
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(shaped.get("_etag"), None);
    }

    #[tokio::test]
    async fn revoke_current_user_access_token_posts_the_cause_body_to_the_revoke_path() {
        let body = json!({ "revoke_cause": "token was compromised" });
        let fake = FakeGocd::replies(docs_current_user_revoked_body(), None);
        let result = service(fake.clone())
            .revoke_current_user_access_token(Parameters(super::RevokeAccessToken {
                token_id: "42".into(),
                body: body.clone(),
            }))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![
                GocdCall::post("api/current_user/access_tokens/42/revoke")
                    .version(1)
                    .body(body)
            ]
        );
        assert_ne!(result.is_error, Some(true));
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(shaped, shaped_current_user_revoked());
    }

    #[tokio::test]
    async fn get_admin_access_tokens_sends_no_query_when_the_filter_is_unset() {
        let fake = FakeGocd::replies(docs_admin_list_body(), None);
        let result = service(fake.clone())
            .get_admin_access_tokens(Parameters(super::AdminAccessTokensQuery { filter: None }))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![GocdCall::get("api/admin/access_tokens").version(1)]
        );
        assert_ne!(result.is_error, Some(true));
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(shaped, shaped_admin_list());
    }

    #[tokio::test]
    async fn get_admin_access_tokens_sends_the_optional_filter_as_a_query_pair() {
        let fake = FakeGocd::replies(docs_admin_list_body(), None);
        let result = service(fake.clone())
            .get_admin_access_tokens(Parameters(super::AdminAccessTokensQuery {
                filter: Some("all".into()),
            }))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![
                GocdCall::get("api/admin/access_tokens")
                    .version(1)
                    .query(vec![("filter".to_string(), "all".to_string())])
            ]
        );
        assert_ne!(result.is_error, Some(true));
    }

    #[tokio::test]
    async fn get_admin_access_token_reads_any_users_token_by_id() {
        let fake = FakeGocd::replies(docs_admin_token_body(), None);
        let result = service(fake.clone())
            .get_admin_access_token(Parameters(super::AccessTokenId {
                token_id: "42".into(),
            }))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![GocdCall::get("api/admin/access_tokens/42").version(1)]
        );
        assert_ne!(result.is_error, Some(true));
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(shaped, shaped_admin_token());
    }

    #[tokio::test]
    async fn revoke_admin_access_token_posts_the_cause_body_to_the_revoke_path() {
        let body = json!({ "revoke_cause": "token was compromised" });
        let fake = FakeGocd::replies(docs_admin_revoked_body(), None);
        let result = service(fake.clone())
            .revoke_admin_access_token(Parameters(super::RevokeAccessToken {
                token_id: "42".into(),
                body: body.clone(),
            }))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![
                GocdCall::post("api/admin/access_tokens/42/revoke")
                    .version(1)
                    .body(body)
            ]
        );
        assert_ne!(result.is_error, Some(true));
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(shaped, shaped_admin_revoked());
    }

    #[tokio::test]
    async fn an_unauthorized_read_surfaces_the_token_hint() {
        let fake = FakeGocd::fails(GocdError::Http { status: 401 });
        let result = service(fake).get_current_user_access_tokens().await;

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
        for expected in [
            "get_admin_access_token",
            "get_admin_access_tokens",
            "get_current_user_access_token",
            "get_current_user_access_tokens",
            "revoke_admin_access_token",
            "revoke_current_user_access_token",
        ] {
            assert!(names.contains(&expected.to_string()), "{expected} missing");
        }
    }
}
