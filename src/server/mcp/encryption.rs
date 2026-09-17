// Encryption section: POST /go/api/admin/encrypt, API v1.

use super::OmgMcp;
use crate::gocd::encryption;
use rmcp::schemars::JsonSchema;
use rmcp::{handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub(crate) struct EncryptBody {
    /// The encryption request object: {"value": "<plain text>"}.
    pub body: Value,
}

#[tool_router(router = encryption_tool_router, vis = "pub(crate)")]
impl OmgMcp {
    #[tool(
        description = "Get the cipher text for a plain text value (POST /go/api/admin/encrypt, API v1), for pasting into other APIs that take `encrypted_value`. `body` is {\"value\": \"<plain text>\"}. GoCD rate-limits this endpoint to 30 requests per minute to blunt brute-force attacks (server-side knob: `go.encryption.api.max.requests`). Returns {\"encrypted_value\": ...} with `_links` removed and `_etag` included when GoCD sent one. Docs: https://api.gocd.org/current/#encryption"
    )]
    async fn encrypt_value(&self, Parameters(args): Parameters<EncryptBody>) -> CallToolResult {
        self.request_shaped(encryption::encrypt(args.body)).await
    }
}

#[cfg(test)]
mod tests {
    use crate::gocd::{GocdCall, GocdError};
    use crate::server::mcp::fake::{FakeGocd, first_text, service};
    use rmcp::handler::server::wrapper::Parameters;
    use serde_json::{Value, json};

    #[tokio::test]
    async fn encrypt_value_posts_the_given_object_and_returns_the_cipher_text() {
        // Body taken verbatim from the GoCD API docs' encryption example.
        let fake = FakeGocd::replies(
            json!({
                "_links": {
                    "self": { "href": "http://ci.example.com/go/api/admin/encrypt" },
                    "doc": { "href": "https://api.gocd.org/#encryption" }
                },
                "encrypted_value": "aSdiFgRRZ6A="
            }),
            None,
        );
        let result = service(fake.clone())
            .encrypt_value(Parameters(super::EncryptBody {
                body: json!({ "value": "badger" }),
            }))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![
                GocdCall::post("api/admin/encrypt")
                    .version(1)
                    .body(json!({ "value": "badger" }))
            ]
        );
        assert_ne!(result.is_error, Some(true));
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(shaped, json!({ "encrypted_value": "aSdiFgRRZ6A=" }));
    }

    #[tokio::test]
    async fn encrypt_value_surfaces_the_etag_when_go_cd_sent_one() {
        let fake = FakeGocd::replies(
            json!({ "encrypted_value": "aSdiFgRRZ6A=" }),
            Some("\"deadbeef\"".into()),
        );
        let result = service(fake)
            .encrypt_value(Parameters(super::EncryptBody {
                body: json!({ "value": "badger" }),
            }))
            .await;

        assert_ne!(result.is_error, Some(true));
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(
            shaped,
            json!({ "encrypted_value": "aSdiFgRRZ6A=", "_etag": "\"deadbeef\"" })
        );
    }

    #[tokio::test]
    async fn an_unauthorized_encrypt_surfaces_the_token_hint() {
        let fake = FakeGocd::fails(GocdError::Http { status: 401 });
        let result = service(fake)
            .encrypt_value(Parameters(super::EncryptBody {
                body: json!({ "value": "badger" }),
            }))
            .await;

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
        assert!(
            names.contains(&"encrypt_value".to_string()),
            "encrypt_value missing"
        );
    }

    #[test]
    fn the_tool_description_discloses_the_rate_limit() {
        let service = service(FakeGocd::replies(json!({}), None));
        let tool = service
            .tool_router
            .list_all()
            .into_iter()
            .find(|t| t.name == "encrypt_value")
            .expect("encrypt_value registered");
        let description = tool.description.as_deref().unwrap_or_default();
        assert!(description.contains("POST /go/api/admin/encrypt"));
        assert!(description.contains("API v1"));
        assert!(description.contains("#encryption"));
        assert!(
            description.contains("30 requests per minute"),
            "got: {description}"
        );
    }
}
