// Users section: /go/api/users, API v3. GoCD neither documents nor enforces
// If-Match on these writes, so — as in the Agents and Current User sections —
// the write tools take no `etag` argument and send none.

use super::OmgMcp;
use crate::gocd::users;
use rmcp::schemars::JsonSchema;
use rmcp::{handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub(crate) struct LoginName {
    /// The user's login name, e.g. "jdoe".
    pub login_name: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub(crate) struct CreateUser {
    /// The user JSON object; "login_name" MUST be specified, optionally with
    /// {"enabled", "email", "email_me", "checkin_aliases"}.
    pub body: Value,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub(crate) struct UpdateUser {
    /// The login name of the user to update.
    pub login_name: String,
    /// The attributes to change; at least one of: {"enabled", "email",
    /// "email_me", "checkin_aliases"}. Omitted attributes are left unchanged.
    pub body: Value,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub(crate) struct BulkUsersRequest {
    /// The bulk request body: for delete, {"users": [...]}; for enable or
    /// disable, {"users": [...], "operations": {"enable": true|false}}.
    pub body: Value,
}

#[tool_router(router = users_tool_router, vis = "pub(crate)")]
impl OmgMcp {
    #[tool(
        description = "List all users (GET /go/api/users, API v3). Returns `_embedded.users` — login_name, display_name, enabled, email, email_me, admin, roles and checkin_aliases — with all `_links` removed. Docs: https://api.gocd.org/current/#get-all-users"
    )]
    async fn get_users(&self) -> CallToolResult {
        self.request_shaped(users::list()).await
    }

    #[tool(
        description = "Read one user by login name (GET /go/api/users/:login_name, API v3). Returns the user object — login_name, display_name, enabled, email, email_me, admin, roles, checkin_aliases — with all `_links` removed and `_etag` included when GoCD sent one. Docs: https://api.gocd.org/current/#get-one-user"
    )]
    async fn get_user(&self, Parameters(args): Parameters<LoginName>) -> CallToolResult {
        self.request_shaped(users::read(&args.login_name)).await
    }

    #[tool(
        description = "Create a user (POST /go/api/users, API v3). `body` MUST specify \"login_name\" and may specify {\"enabled\", \"email\", \"email_me\", \"checkin_aliases\"}. Returns the created user object with `_links` removed. Docs: https://api.gocd.org/current/#create-a-user"
    )]
    async fn create_user(&self, Parameters(args): Parameters<CreateUser>) -> CallToolResult {
        self.request_shaped(users::create(args.body)).await
    }

    #[tool(
        description = "Update attributes of one user (PATCH /go/api/users/:login_name, API v3). `body` carries at least one of {\"enabled\", \"email\", \"email_me\", \"checkin_aliases\"}; omitted attributes are left unchanged. Returns the updated user object with `_links` removed. GoCD does not gate this write behind If-Match. Docs: https://api.gocd.org/current/#update-a-user"
    )]
    async fn update_user(&self, Parameters(args): Parameters<UpdateUser>) -> CallToolResult {
        self.request_shaped(users::update(&args.login_name, args.body))
            .await
    }

    #[tool(
        description = "Delete one user by login name (DELETE /go/api/users/:login_name, API v3). Disable the user first — GoCD refuses to delete an enabled user. Returns a message confirming deletion. GoCD does not gate this write behind If-Match. Docs: https://api.gocd.org/current/#delete-a-user"
    )]
    async fn delete_user(&self, Parameters(args): Parameters<LoginName>) -> CallToolResult {
        self.request_shaped(users::remove(&args.login_name)).await
    }

    #[tool(
        description = "Bulk delete users (DELETE /go/api/users, API v3). `body` is {\"users\": [...]} listing login names; disable the users first. Returns a message confirming the deletion. Docs: https://api.gocd.org/current/#bulk-delete-users"
    )]
    async fn bulk_delete_users(
        &self,
        Parameters(args): Parameters<BulkUsersRequest>,
    ) -> CallToolResult {
        self.request_shaped(users::bulk_delete(args.body)).await
    }

    #[tool(
        description = "Enable or disable multiple users (PATCH /go/api/users/operations/state, API v3). `body` is {\"users\": [...], \"operations\": {\"enable\": true|false}}. Returns a message confirming the enable/disable. Docs: https://api.gocd.org/current/#bulk-enable-disable-users"
    )]
    async fn bulk_enable_disable_users(
        &self,
        Parameters(args): Parameters<BulkUsersRequest>,
    ) -> CallToolResult {
        self.request_shaped(users::bulk_update_state(args.body))
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::gocd::{GocdCall, GocdError};
    use crate::server::mcp::fake::{FakeGocd, first_text, service};
    use rmcp::handler::server::wrapper::Parameters;
    use serde_json::{Value, json};

    // List body taken verbatim from the GoCD API docs' get-all-users example.
    fn stored_list_body() -> Value {
        json!({
            "_links": {
                "self": { "href": "https://ci.example.com/go/api/users" },
                "doc": { "href": "https://api.gocd.org/#users" }
            },
            "_embedded": {
                "users": [ {
                    "_links": {
                        "doc": { "href": "https://api.gocd.org/#users" },
                        "self": { "href": "https://ci.example.com/go/api/users/admin" },
                        "find": { "href": "https://ci.example.com/go/api/users/:login_name" }
                    },
                    "login_name": "jdoe",
                    "display_name": "John Doe",
                    "enabled": true,
                    "email": null,
                    "email_me": false,
                    "admin": false,
                    "roles": [ { "name": "role name", "type": "gocd" } ],
                    "checkin_aliases": [ "jdoe", "johndoe" ]
                } ]
            }
        })
    }

    fn shaped_list_user() -> Value {
        json!({
            "_embedded": {
                "users": [ {
                    "login_name": "jdoe",
                    "display_name": "John Doe",
                    "enabled": true,
                    "email": null,
                    "email_me": false,
                    "admin": false,
                    "roles": [ { "name": "role name", "type": "gocd" } ],
                    "checkin_aliases": [ "jdoe", "johndoe" ]
                } ]
            }
        })
    }

    // Single-user body taken verbatim from the GoCD API docs' get-one-user
    // example.
    fn docs_single_user_body() -> Value {
        json!({
            "_links": {
                "doc": { "href": "https://api.gocd.org/#users" },
                "self": { "href": "https://ci.example.com/go/api/users/jdoe" },
                "find": { "href": "https://ci.example.com/go/api/users/:login_name" }
            },
            "login_name": "jdoe",
            "display_name": "jdoe",
            "enabled": true,
            "email": null,
            "email_me": false,
            "admin": true,
            "roles": [ { "name": "role name", "type": "gocd" } ],
            "checkin_aliases": []
        })
    }

    fn shaped_single_user(etag: &str) -> Value {
        json!({
            "login_name": "jdoe",
            "display_name": "jdoe",
            "enabled": true,
            "email": null,
            "email_me": false,
            "admin": true,
            "roles": [ { "name": "role name", "type": "gocd" } ],
            "checkin_aliases": [],
            "_etag": etag
        })
    }

    #[tokio::test]
    async fn get_users_lists_through_a_version_3_get_and_shapes_the_answer() {
        let fake = FakeGocd::replies(stored_list_body(), None);
        let result = service(fake.clone()).get_users().await;

        assert_eq!(fake.recorded(), vec![GocdCall::get("api/users").version(3)]);
        assert_ne!(result.is_error, Some(true));
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(shaped, shaped_list_user());
    }

    #[tokio::test]
    async fn get_user_reads_one_user_by_login_name_and_injects_a_sent_etag() {
        let fake = FakeGocd::replies(docs_single_user_body(), Some("\"deadbeef\"".into()));
        let result = service(fake.clone())
            .get_user(Parameters(super::LoginName {
                login_name: "jdoe".into(),
            }))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![GocdCall::get("api/users/jdoe").version(3)]
        );
        assert_ne!(result.is_error, Some(true));
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(shaped, shaped_single_user("\"deadbeef\""));
    }

    #[tokio::test]
    async fn get_user_keeps_the_body_free_of_etag_when_go_cd_sends_none() {
        let fake = FakeGocd::replies(docs_single_user_body(), None);
        let result = service(fake.clone())
            .get_user(Parameters(super::LoginName {
                login_name: "jdoe".into(),
            }))
            .await;

        assert_ne!(result.is_error, Some(true));
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(shaped.get("_etag"), None);
    }

    #[tokio::test]
    async fn create_user_posts_the_given_object_as_json() {
        let body = json!({ "login_name": "jdoe", "email": "jdoe@example.com" });
        let fake = FakeGocd::replies(docs_single_user_body(), None);
        let result = service(fake.clone())
            .create_user(Parameters(super::CreateUser { body: body.clone() }))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![GocdCall::post("api/users").version(3).body(body)]
        );
        assert_ne!(result.is_error, Some(true));
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(shaped.get("_links"), None);
        assert_eq!(shaped["login_name"], json!("jdoe"));
    }

    #[tokio::test]
    async fn update_user_patches_the_body_by_login_name_without_if_match() {
        let body = json!({
            "email": "jdoe@example.com",
            "email_me": true,
            "checkin_aliases": [ "jdoe", "johndoe" ]
        });
        let fake = FakeGocd::replies(docs_single_user_body(), None);
        let result = service(fake.clone())
            .update_user(Parameters(super::UpdateUser {
                login_name: "jdoe".into(),
                body: body.clone(),
            }))
            .await;

        // GoCD does not gate the user PATCH behind If-Match, so the recorded
        // call must carry the body and no etag.
        assert_eq!(
            fake.recorded(),
            vec![GocdCall::patch("api/users/jdoe").version(3).body(body)]
        );
        assert_ne!(result.is_error, Some(true));
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(shaped.get("_links"), None);
    }

    #[tokio::test]
    async fn delete_user_deletes_by_login_name_and_returns_the_message() {
        let fake = FakeGocd::replies(
            json!({ "message": "User 'jdoe' was deleted successfully." }),
            None,
        );
        let result = service(fake.clone())
            .delete_user(Parameters(super::LoginName {
                login_name: "jdoe".into(),
            }))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![GocdCall::delete("api/users/jdoe").version(3)]
        );
        assert_ne!(result.is_error, Some(true));
        assert!(first_text(&result).contains("deleted successfully"));
    }

    #[tokio::test]
    async fn bulk_delete_users_deletes_the_collection_with_the_body() {
        let body = json!({ "users": [ "jez", "tez" ] });
        let fake = FakeGocd::replies(
            json!({ "message": "Users '[\"jez\", \"tez\"]' were deleted successfully." }),
            None,
        );
        let result = service(fake.clone())
            .bulk_delete_users(Parameters(super::BulkUsersRequest { body: body.clone() }))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![GocdCall::delete("api/users").version(3).body(body)]
        );
        assert_ne!(result.is_error, Some(true));
        assert!(first_text(&result).contains("were deleted successfully"));
    }

    #[tokio::test]
    async fn bulk_enable_disable_users_patches_the_state_endpoint_with_the_body() {
        let body = json!({ "users": [ "jez", "tez" ], "operations": { "enable": true } });
        let fake = FakeGocd::replies(
            json!({ "message": "Users '[\"jez\", \"tez\"]' were enabled successfully." }),
            None,
        );
        let result = service(fake.clone())
            .bulk_enable_disable_users(Parameters(super::BulkUsersRequest { body: body.clone() }))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![
                GocdCall::patch("api/users/operations/state")
                    .version(3)
                    .body(body)
            ]
        );
        assert_ne!(result.is_error, Some(true));
        assert!(first_text(&result).contains("were enabled successfully"));
    }

    #[tokio::test]
    async fn an_unauthorized_read_surfaces_the_token_hint() {
        let fake = FakeGocd::fails(GocdError::Http { status: 401 });
        let result = service(fake).get_users().await;

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
            "bulk_delete_users",
            "bulk_enable_disable_users",
            "create_user",
            "delete_user",
            "get_user",
            "get_users",
            "update_user",
        ] {
            assert!(names.contains(&expected.to_string()), "{expected} missing");
        }
    }
}
