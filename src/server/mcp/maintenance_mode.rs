// Maintenance Mode section: /go/api/admin/maintenance_mode/info (API v1) and
// the enable/disable actions (API v1). GoCD documents no If-Match-gated
// write here, so no tool takes an etag arg. The docs' "HTTP Request" lines
// list the actions as GET, but their own curl examples — and the GoCD
// server's registered routes — send them as body-less POSTs carrying the
// documented X-GoCD-Confirm header, which is what omg does.

use super::OmgMcp;
use crate::gocd::maintenance_mode;
use rmcp::{model::CallToolResult, tool, tool_router};

#[tool_router(router = maintenance_mode_tool_router, vis = "pub(crate)")]
impl OmgMcp {
    #[tool(
        description = "Read the server maintenance mode info (GET /go/api/admin/maintenance_mode/info, API v1). Returns the maintenance mode info object — `_embedded` with `is_maintenance_mode`, `metadata` (updated_by, updated_on) and `attributes` (has_running_systems, running_systems) — with `_links` removed and `_etag` included when GoCD sent one. Requires admin rights. Docs: https://api.gocd.org/current/#get-maintenance-mode-info"
    )]
    async fn get_maintenance_mode_info(&self) -> CallToolResult {
        self.request_shaped(maintenance_mode::info()).await
    }

    #[tool(
        description = "Enable GoCD server maintenance mode (API v1). The docs' HTTP Request line shows this as GET-as-action, but the documented curl example and the GoCD server's routes send it as a body-less POST to /go/api/admin/maintenance_mode/enable with the X-GoCD-Confirm header — which is exactly what omg does. Requires admin rights; GoCD documents no If-Match guard here, so no etag arg. GoCD answers 204 No Content (shaped as null); it answers 409 when the server is already in maintenance mode. Docs: https://api.gocd.org/current/#enable-maintenance-mode"
    )]
    async fn enable_maintenance_mode(&self) -> CallToolResult {
        self.request_shaped(maintenance_mode::enable()).await
    }

    #[tool(
        description = "Disable GoCD server maintenance mode (API v1). The docs' HTTP Request line shows this as GET-as-action, but the documented curl example and the GoCD server's routes send it as a body-less POST to /go/api/admin/maintenance_mode/disable with the X-GoCD-Confirm header — which is exactly what omg does. Requires admin rights; GoCD documents no If-Match guard here, so no etag arg. GoCD answers 204 No Content (shaped as null); it answers 409 when the server is not in maintenance mode. Docs: https://api.gocd.org/current/#disable-maintenance-mode"
    )]
    async fn disable_maintenance_mode(&self) -> CallToolResult {
        self.request_shaped(maintenance_mode::disable()).await
    }
}

#[cfg(test)]
mod tests {
    use crate::gocd::{GocdCall, GocdError};
    use crate::server::mcp::fake::{FakeGocd, first_text, service};
    use serde_json::{Value, json};

    // The info body taken verbatim from the docs' example, `_links` included
    // so the drop is exercised; the docs' response header also shows an ETag,
    // which the test below feeds to the fake separately.
    fn docs_info_body() -> Value {
        json!({
            "_links" : {
                "self" : { "href" : "https://ci.example.com/go/api/admin/maintenance_mode/info" },
                "doc" : { "href" : "https://api.gocd.org/current/#maintenance-mode-info" }
            },
            "_embedded" : {
                "is_maintenance_mode" : true,
                "metadata" : {
                    "updated_by" : "admin",
                    "updated_on" : "2019-01-02T04:18:28Z"
                },
                "attributes" : {
                    "has_running_systems" : false,
                    "running_systems" : {
                        "material_update_in_progress" : [ ],
                        "scheduled_jobs" : [ ],
                        "running_jobs" : [ ]
                    }
                }
            }
        })
    }

    #[tokio::test]
    async fn get_maintenance_mode_info_reads_through_a_version_1_get_and_shapes_the_answer() {
        let fake = FakeGocd::replies(
            docs_info_body(),
            Some("\"cbc5f2d5b9c13a2cc1b1efb3d8a6155d\"".into()),
        );
        let result = service(fake.clone()).get_maintenance_mode_info().await;

        assert_eq!(
            fake.recorded(),
            vec![GocdCall::get("api/admin/maintenance_mode/info").version(1)]
        );
        assert_ne!(result.is_error, Some(true));
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(
            shaped,
            json!({
                "_embedded" : {
                    "is_maintenance_mode" : true,
                    "metadata" : {
                        "updated_by" : "admin",
                        "updated_on" : "2019-01-02T04:18:28Z"
                    },
                    "attributes" : {
                        "has_running_systems" : false,
                        "running_systems" : {
                            "material_update_in_progress" : [ ],
                            "scheduled_jobs" : [ ],
                            "running_jobs" : [ ]
                        }
                    }
                },
                "_etag": "\"cbc5f2d5b9c13a2cc1b1efb3d8a6155d\""
            })
        );
    }

    #[tokio::test]
    async fn enable_maintenance_mode_posts_the_documented_confirmation() {
        // The docs' 204 No Content: an empty body.
        let fake = FakeGocd::replies(Value::Null, None);
        let result = service(fake.clone()).enable_maintenance_mode().await;

        assert_eq!(
            fake.recorded(),
            vec![
                GocdCall::post("api/admin/maintenance_mode/enable")
                    .version(1)
                    .confirm()
            ]
        );
        assert_ne!(result.is_error, Some(true));
    }

    #[tokio::test]
    async fn disable_maintenance_mode_posts_the_documented_confirmation() {
        let fake = FakeGocd::replies(Value::Null, None);
        let result = service(fake.clone()).disable_maintenance_mode().await;

        assert_eq!(
            fake.recorded(),
            vec![
                GocdCall::post("api/admin/maintenance_mode/disable")
                    .version(1)
                    .confirm()
            ]
        );
        assert_ne!(result.is_error, Some(true));
    }

    #[tokio::test]
    async fn a_204_reply_shapes_to_null_even_when_go_cd_sent_an_etag() {
        // Shaping is uniform: the empty 204 body stays null, so there is no
        // object to inject `_etag` into and no `_links` to drop.
        let fake = FakeGocd::replies(
            Value::Null,
            Some("\"cbc5f2d5b9c13a2cc1b1efb3d8a6155d\"".into()),
        );
        let result = service(fake).enable_maintenance_mode().await;

        assert_ne!(result.is_error, Some(true));
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(shaped, Value::Null);
    }

    #[tokio::test]
    async fn an_unauthorized_info_read_surfaces_the_token_hint() {
        let fake = FakeGocd::fails(GocdError::Http { status: 401 });
        let result = service(fake).get_maintenance_mode_info().await;

        assert_eq!(result.is_error, Some(true));
        let text = first_text(&result);
        assert!(text.contains("401"), "got: {text}");
        assert!(text.contains("token"), "got: {text}");
    }

    #[tokio::test]
    async fn an_unauthorized_enable_surfaces_the_token_hint() {
        let fake = FakeGocd::fails(GocdError::Http { status: 401 });
        let result = service(fake).enable_maintenance_mode().await;

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
            "disable_maintenance_mode",
            "enable_maintenance_mode",
            "get_maintenance_mode_info",
        ] {
            assert!(names.contains(&expected.to_string()), "{expected} missing");
        }
    }
}
