// Current User section: /go/api/current_user, API v1. The GET tool lives in the
// root router (migrated in #2); this module adds the PATCH. GoCD neither
// documents nor requires If-Match on it — as on the Users API — so the tool
// takes no etag and sends none.

use super::OmgMcp;
use crate::gocd::current_user;
use rmcp::schemars::JsonSchema;
use rmcp::{handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub(crate) struct UpdateCurrentUser {
    /// A JSON object with at least one of "email" (string), "email_me"
    /// (boolean), "checkin_aliases" (array of strings). Attributes you omit
    /// are left unchanged; login_name and display_name are not updatable here.
    pub body: Value,
}

#[tool_router(router = current_user_tool_router, vis = "pub(crate)")]
impl OmgMcp {
    #[tool(
        description = "Update attributes of the GoCD user the configured token authenticates as (PATCH /go/api/current_user, API v1). `body` is a JSON object with at least one of \"email\", \"email_me\", \"checkin_aliases\"; omitted attributes are left unchanged, and no If-Match is required. Returns the updated user object with `_links` removed and `_etag` included when GoCD sent one. Docs: https://api.gocd.org/current/#current-user"
    )]
    async fn update_current_user(
        &self,
        Parameters(args): Parameters<UpdateCurrentUser>,
    ) -> CallToolResult {
        self.request_shaped(current_user::update(args.body)).await
    }
}

#[cfg(test)]
mod tests {
    use crate::gocd::{GocdCall, GocdError};
    use crate::server::mcp::fake::{FakeGocd, first_text, service};
    use rmcp::handler::server::wrapper::Parameters;
    use serde_json::{Value, json};

    // Body and ETag taken verbatim from the GoCD API docs' update-current-user
    // example response.
    fn docs_updated_user_body() -> Value {
        json!({
            "_links": {
                "doc": { "href": "https://api.gocd.org/#users" },
                "self": { "href": "https://ci.example.com/go/api/users/jdoe" },
                "find": { "href": "https://ci.example.com/go/api/users/:login_name" }
            },
            "login_name": "jdoe",
            "display_name": "jdoe",
            "enabled": false,
            "email": "jdoe@example.com",
            "email_me": true,
            "checkin_aliases": ["jdoe", "johndoe"]
        })
    }

    fn shaped_updated_user() -> Value {
        json!({
            "login_name": "jdoe",
            "display_name": "jdoe",
            "enabled": false,
            "email": "jdoe@example.com",
            "email_me": true,
            "checkin_aliases": ["jdoe", "johndoe"],
            "_etag": "\"deadbeef\""
        })
    }

    #[tokio::test]
    async fn update_current_user_patches_the_documented_path_with_the_body() {
        let body = json!({
            "email": "jdoe@example.com",
            "email_me": true,
            "checkin_aliases": ["jdoe", "johndoe"]
        });
        let fake = FakeGocd::replies(docs_updated_user_body(), Some("\"deadbeef\"".into()));
        let result = service(fake.clone())
            .update_current_user(Parameters(super::UpdateCurrentUser { body: body.clone() }))
            .await;

        // The doc's request carries no If-Match, so the recorded call must not
        // gain one.
        assert_eq!(
            fake.recorded(),
            vec![GocdCall::patch("api/current_user").version(1).body(body)]
        );
        assert_ne!(result.is_error, Some(true));
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(shaped, shaped_updated_user());
    }

    #[tokio::test]
    async fn an_unauthorized_write_surfaces_the_token_hint() {
        let fake = FakeGocd::fails(GocdError::Http { status: 401 });
        let result = service(fake)
            .update_current_user(Parameters(super::UpdateCurrentUser {
                body: json!({ "email": "jdoe@example.com" }),
            }))
            .await;

        assert_eq!(result.is_error, Some(true));
        let text = first_text(&result);
        assert!(text.contains("401"), "got: {text}");
        assert!(text.contains("token"), "got: {text}");
    }

    #[tokio::test]
    async fn a_precondition_failure_surfaces_the_412_reread_hint() {
        let fake = FakeGocd::fails(GocdError::Http { status: 412 });
        let result = service(fake)
            .update_current_user(Parameters(super::UpdateCurrentUser {
                body: json!({ "email": "jdoe@example.com" }),
            }))
            .await;

        assert_eq!(result.is_error, Some(true));
        let text = first_text(&result);
        assert!(text.contains("412"), "got: {text}");
        assert!(text.contains("_etag"), "got: {text}");
    }

    #[test]
    fn the_section_tool_is_merged_into_the_service_router() {
        let service = service(FakeGocd::replies(json!({}), None));
        let names: Vec<String> = service
            .tool_router
            .list_all()
            .into_iter()
            .map(|t| t.name.to_string())
            .collect();
        for expected in ["get_current_user", "update_current_user"] {
            assert!(names.contains(&expected.to_string()), "{expected} missing");
        }
    }
}
