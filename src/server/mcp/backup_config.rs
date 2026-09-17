// Backup Config section: /go/api/config/backup, API v1.

use super::OmgMcp;
use crate::gocd::backup_config;
use rmcp::schemars::JsonSchema;
use rmcp::{handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub(crate) struct UpdateBackupConfig {
    /// The backup config object: {"schedule", "post_backup_script",
    /// "email_on_failure", "email_on_success"}.
    pub body: Value,
}

#[tool_router(router = backup_config_tool_router, vis = "pub(crate)")]
impl OmgMcp {
    #[tool(
        description = "Read the server's backup config (GET /go/api/config/backup, API v1). Returns the backup config object — schedule, post_backup_script, email_on_failure, email_on_success — with `_links` removed and `_etag` included when GoCD sent one. Docs: https://api.gocd.org/current/#get-backup-config"
    )]
    async fn get_backup_config(&self) -> CallToolResult {
        self.request_shaped(backup_config::read()).await
    }

    #[tool(
        description = "Create or replace the server's backup config (PUT /go/api/config/backup, API v1; the doc's POST+PUT pair is collapsed into this one tool). `body` is the whole backup config object {\"schedule\", \"post_backup_script\", \"email_on_failure\", \"email_on_success\"} — docs: https://api.gocd.org/current/#the-backup-config-object. GoCD documents no If-Match guard here, so there is no etag argument. Returns the stored backup config with `_links` removed."
    )]
    async fn update_backup_config(
        &self,
        Parameters(args): Parameters<UpdateBackupConfig>,
    ) -> CallToolResult {
        self.request_shaped(backup_config::update(args.body)).await
    }

    #[tool(
        description = "Delete the server's backup config (DELETE /go/api/config/backup, API v1). Returns GoCD's confirmation message with `_links` removed. Docs: https://api.gocd.org/current/#delete-backup-config"
    )]
    async fn delete_backup_config(&self) -> CallToolResult {
        self.request_shaped(backup_config::remove()).await
    }
}

#[cfg(test)]
mod tests {
    use crate::gocd::{GocdCall, GocdError};
    use crate::server::mcp::fake::{FakeGocd, first_text, service};
    use rmcp::handler::server::wrapper::Parameters;
    use serde_json::{Value, json};

    // Body taken verbatim from the GoCD API docs' get example.
    fn docs_config_body() -> Value {
        json!({
            "_links": {
                "doc": { "href": "https://api.gocd.org/#backup-config" },
                "self": { "href": "https://ci.example.com/go/api/config/backup" }
            },
            "email_on_failure": false,
            "email_on_success": false,
            "post_backup_script": "/usr/local/bin/copy-gocd-backup-to-s3",
            "schedule": "0 0 2 * * ?"
        })
    }

    #[tokio::test]
    async fn get_backup_config_reads_through_a_version_1_get_and_shapes_the_answer() {
        let fake = FakeGocd::replies(docs_config_body(), Some("\"abc123\"".into()));
        let result = service(fake.clone()).get_backup_config().await;

        assert_eq!(
            fake.recorded(),
            vec![GocdCall::get("api/config/backup").version(1)]
        );
        assert_ne!(result.is_error, Some(true));
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(
            shaped,
            json!({
                "email_on_failure": false,
                "email_on_success": false,
                "post_backup_script": "/usr/local/bin/copy-gocd-backup-to-s3",
                "schedule": "0 0 2 * * ?",
                "_etag": "\"abc123\""
            })
        );
    }

    #[tokio::test]
    async fn update_backup_config_puts_the_whole_config_object() {
        let body = json!({
            "email_on_failure": false,
            "email_on_success": false,
            "post_backup_script": "/usr/local/bin/copy-gocd-backup-to-s3",
            "schedule": "0 0 2 * * ?"
        });
        let fake = FakeGocd::replies(docs_config_body(), None);
        let result = service(fake.clone())
            .update_backup_config(Parameters(super::UpdateBackupConfig { body: body.clone() }))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![GocdCall::put("api/config/backup").version(1).body(body)]
        );
        assert_ne!(result.is_error, Some(true));
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(shaped["schedule"], "0 0 2 * * ?");
        assert!(shaped.get("_links").is_none());
        assert!(shaped.get("_etag").is_none());
    }

    #[tokio::test]
    async fn delete_backup_config_deletes_and_returns_the_confirmation_message() {
        let fake = FakeGocd::replies(
            json!({ "message": "Backup config was deleted successfully!" }),
            None,
        );
        let result = service(fake.clone()).delete_backup_config().await;

        assert_eq!(
            fake.recorded(),
            vec![GocdCall::delete("api/config/backup").version(1)]
        );
        assert_ne!(result.is_error, Some(true));
        assert!(first_text(&result).contains("deleted successfully"));
    }

    #[tokio::test]
    async fn an_unauthorized_read_surfaces_the_token_hint() {
        let fake = FakeGocd::fails(GocdError::Http { status: 401 });
        let result = service(fake).get_backup_config().await;

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
            "get_backup_config",
            "update_backup_config",
            "delete_backup_config",
        ] {
            assert!(names.contains(&expected.to_string()), "{expected} missing");
        }
    }
}
