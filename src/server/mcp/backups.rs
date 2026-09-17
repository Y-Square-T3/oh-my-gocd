// Backups section: /go/api/backups, API v2.

use super::OmgMcp;
use crate::gocd::backups;
use rmcp::schemars::JsonSchema;
use rmcp::{handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router};
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub(crate) struct BackupId {
    /// A numeric backup id, or the keyword "running" for the backup currently
    /// in progress — the way to reach a backup scheduled via this server.
    pub backup_id: String,
}

#[tool_router(router = backups_tool_router, vis = "pub(crate)")]
impl OmgMcp {
    #[tool(
        description = "Schedule a backup of all GoCD configuration and the database (POST /go/api/backups, API v2; admin only). The backup runs asynchronously and the server may be unavailable while it runs. GoCD answers 202 with an empty body, so the tool text is `null` on success; poll the result with get_backup using the backup_id \"running\" while it is in progress. Docs: https://api.gocd.org/current/#schedule-backup"
    )]
    async fn schedule_backup(&self) -> CallToolResult {
        self.request_shaped(backups::schedule()).await
    }

    #[tool(
        description = "Get the status of a scheduled GoCD backup (GET /go/api/backups/:backup_id, API v2). Pass the keyword \"running\" as backup_id to poll the backup currently in progress, or a numeric backup id. Returns the backup object — time, path, status (COMPLETED/IN_PROGRESS/ERROR/ABORTED), progress_status, message, user — with `_links` removed and `_etag` included when GoCD sent one. Docs: https://api.gocd.org/current/#get-backup"
    )]
    async fn get_backup(&self, Parameters(args): Parameters<BackupId>) -> CallToolResult {
        self.request_shaped(backups::status(&args.backup_id)).await
    }
}

#[cfg(test)]
mod tests {
    use crate::gocd::{GocdCall, GocdError};
    use crate::server::mcp::fake::{FakeGocd, first_text, service};
    use rmcp::handler::server::wrapper::Parameters;
    use serde_json::{Value, json};

    #[tokio::test]
    async fn schedule_backup_posts_a_version_2_request_and_reports_the_empty_ack() {
        // The docs' 202 answer is an empty body, which the transport decodes as null.
        let fake = FakeGocd::replies(Value::Null, None);
        let result = service(fake.clone()).schedule_backup().await;

        assert_eq!(
            fake.recorded(),
            vec![GocdCall::post("api/backups").version(2)]
        );
        assert_ne!(result.is_error, Some(true));
        assert_eq!(first_text(&result), "null");
    }

    // Body taken verbatim from the GoCD API docs' backup object example.
    fn docs_backup_body() -> Value {
        json!({
            "_links": {
                "doc": { "href": "https://api.gocd.org/#backups" }
            },
            "time": "2015-08-07T10:07:19.868Z",
            "path": "/var/lib/go-server/serverBackups/backup_20150807-153719",
            "status": "COMPLETED",
            "progress_status": "BACKUP_DATABASE",
            "message": "Backup was generated successfully.",
            "user": {
                "_links": {
                    "doc": { "href": "https://api.gocd.org/#users" },
                    "self": { "href": "https://ci.example.com/go/api/users/username" },
                    "find": { "href": "https://ci.example.com/go/api/users/:login_name" },
                    "current_user": { "href": "https://ci.example.com/go/api/users/current_user" }
                },
                "login_name": "username"
            }
        })
    }

    #[tokio::test]
    async fn get_backup_reads_the_documented_id_and_shapes_the_backup_object() {
        let fake = FakeGocd::replies(
            docs_backup_body(),
            Some("\"16f412cf2f72900f97d432b6ec40270e\"".into()),
        );
        let result = service(fake.clone())
            .get_backup(Parameters(super::BackupId {
                backup_id: "108417776345741632".into(),
            }))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![GocdCall::get("api/backups/108417776345741632").version(2)]
        );
        assert_ne!(result.is_error, Some(true));
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(
            shaped,
            json!({
                "time": "2015-08-07T10:07:19.868Z",
                "path": "/var/lib/go-server/serverBackups/backup_20150807-153719",
                "status": "COMPLETED",
                "progress_status": "BACKUP_DATABASE",
                "message": "Backup was generated successfully.",
                "user": { "login_name": "username" },
                "_etag": "\"16f412cf2f72900f97d432b6ec40270e\""
            })
        );
    }

    #[tokio::test]
    async fn get_backup_accepts_the_running_keyword_for_the_backup_in_progress() {
        let fake = FakeGocd::replies(docs_backup_body(), None);
        let result = service(fake.clone())
            .get_backup(Parameters(super::BackupId {
                backup_id: "running".into(),
            }))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![GocdCall::get("api/backups/running").version(2)]
        );
        assert_ne!(result.is_error, Some(true));
    }

    #[tokio::test]
    async fn an_unauthorized_read_surfaces_the_token_hint() {
        let fake = FakeGocd::fails(GocdError::Http { status: 401 });
        let result = service(fake)
            .get_backup(Parameters(super::BackupId {
                backup_id: "running".into(),
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
        for expected in ["get_backup", "schedule_backup"] {
            assert!(names.contains(&expected.to_string()), "{expected} missing");
        }
    }
}
