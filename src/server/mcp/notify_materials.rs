// Notify Materials section: /go/api/admin/materials/{svn,git,hg,scm}/notify,
// API v2. All four operations are admin POST writes with a JSON body and no
// If-Match guard, so — as on the Access Tokens revokes — the tools take no
// `etag` argument and send none.

use super::OmgMcp;
use crate::gocd::notify_materials;
use rmcp::schemars::JsonSchema;
use rmcp::{handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router};
use serde::Deserialize;
use serde_json::Value;

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub(crate) struct NotifyMaterial {
    /// The notify request object, e.g. {"repository_url": "..."} (git/hg),
    /// {"repository_url": "..."} or {"uuid": "..."} (svn),
    /// {"scm_name": "..."} (scm).
    pub body: Value,
}

#[tool_router(router = notify_materials_tool_router, vis = "pub(crate)")]
impl OmgMcp {
    #[tool(
        description = "Notify GoCD that an SVN material changed so it schedules an update (POST /go/api/admin/materials/svn/notify, API v2). `body` carries one of `repository_url` (the SVN repository URL as configured) or `uuid` (the subversion repository UUID). Requires admin rights — GoCD documents no If-Match guard here. Returns GoCD's 202 confirmation message with `_links` removed and `_etag` included when GoCD sent one. Materials must have polling disabled (auto_update off / webhook-only fetch) to be notified. Docs: https://api.gocd.org/current/#notify-svn-materials"
    )]
    async fn notify_svn_material(
        &self,
        Parameters(args): Parameters<NotifyMaterial>,
    ) -> CallToolResult {
        self.request_shaped(notify_materials::notify_svn(args.body))
            .await
    }

    #[tool(
        description = "Notify GoCD that a git material changed so it schedules an update (POST /go/api/admin/materials/git/notify, API v2). `body` is {\"repository_url\": \"<git repo url as configured>\"}. Requires admin rights — GoCD documents no If-Match guard here. Returns GoCD's 202 confirmation message with `_links` removed and `_etag` included when GoCD sent one. Materials must have polling disabled (auto_update off / webhook-only fetch) to be notified. Docs: https://api.gocd.org/current/#notify-git-materials"
    )]
    async fn notify_git_material(
        &self,
        Parameters(args): Parameters<NotifyMaterial>,
    ) -> CallToolResult {
        self.request_shaped(notify_materials::notify_git(args.body))
            .await
    }

    #[tool(
        description = "Notify GoCD that a Mercurial material changed so it schedules an update (POST /go/api/admin/materials/hg/notify, API v2). `body` is {\"repository_url\": \"<hg repo URL as configured>\"}. Requires admin rights — GoCD documents no If-Match guard here. Returns GoCD's 202 confirmation message with `_links` removed and `_etag` included when GoCD sent one. Materials must have polling disabled (auto_update off / webhook-only fetch) to be notified. Docs: https://api.gocd.org/current/#notify-mercurial-materials"
    )]
    async fn notify_hg_material(
        &self,
        Parameters(args): Parameters<NotifyMaterial>,
    ) -> CallToolResult {
        self.request_shaped(notify_materials::notify_hg(args.body))
            .await
    }

    #[tool(
        description = "Notify GoCD that a pluggable-SCM material changed so it schedules an update (POST /go/api/admin/materials/scm/notify, API v2). `body` is {\"scm_name\": \"<the material name from the pipeline material page>\"}. Requires admin rights — GoCD documents no If-Match guard here. Returns GoCD's 202 confirmation message with `_links` removed and `_etag` included when GoCD sent one. Materials must have polling disabled (auto_update off / webhook-only fetch) to be notified. Docs: https://api.gocd.org/current/#notify-other-scm-materials"
    )]
    async fn notify_scm_material(
        &self,
        Parameters(args): Parameters<NotifyMaterial>,
    ) -> CallToolResult {
        self.request_shaped(notify_materials::notify_scm(args.body))
            .await
    }
}

#[cfg(test)]
mod tests {
    use crate::gocd::{GocdCall, GocdError};
    use crate::server::mcp::fake::{FakeGocd, first_text, service};
    use rmcp::handler::server::wrapper::Parameters;
    use serde_json::{Value, json};

    // The 202 body every notify operation documents, verbatim.
    fn docs_message_body() -> Value {
        json!({ "message": "The material is now scheduled for an update. Please check relevant pipeline(s) for status." })
    }

    fn notify_args(body: Value) -> Parameters<super::NotifyMaterial> {
        Parameters(super::NotifyMaterial { body })
    }

    #[tokio::test]
    async fn notify_svn_material_posts_the_body_to_the_documented_svn_path() {
        let body = json!({ "repository_url": "http://svn.example.com/test-repo" });
        let fake = FakeGocd::replies(docs_message_body(), None);
        let result = service(fake.clone())
            .notify_svn_material(notify_args(body.clone()))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![
                GocdCall::post("api/admin/materials/svn/notify")
                    .version(2)
                    .body(body)
            ]
        );
        assert_ne!(result.is_error, Some(true));
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(shaped, docs_message_body());
    }

    #[tokio::test]
    async fn notify_git_material_posts_the_body_to_the_documented_git_path() {
        let body = json!({ "repository_url": "git://git.example.com/git/funky-widgets.git" });
        let fake = FakeGocd::replies(docs_message_body(), None);
        let result = service(fake.clone())
            .notify_git_material(notify_args(body.clone()))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![
                GocdCall::post("api/admin/materials/git/notify")
                    .version(2)
                    .body(body)
            ]
        );
        assert_ne!(result.is_error, Some(true));
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(shaped, docs_message_body());
    }

    #[tokio::test]
    async fn notify_hg_material_posts_the_body_to_the_documented_hg_path() {
        let body = json!({ "repository_url": "ssh://hg.example.com/hg/repos/funky-widgets" });
        let fake = FakeGocd::replies(docs_message_body(), None);
        let result = service(fake.clone())
            .notify_hg_material(notify_args(body.clone()))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![
                GocdCall::post("api/admin/materials/hg/notify")
                    .version(2)
                    .body(body)
            ]
        );
        assert_ne!(result.is_error, Some(true));
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(shaped, docs_message_body());
    }

    #[tokio::test]
    async fn notify_scm_material_posts_the_body_to_the_documented_scm_path() {
        let body = json!({ "scm_name": "material_name" });
        let fake = FakeGocd::replies(docs_message_body(), None);
        let result = service(fake.clone())
            .notify_scm_material(notify_args(body.clone()))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![
                GocdCall::post("api/admin/materials/scm/notify")
                    .version(2)
                    .body(body)
            ]
        );
        assert_ne!(result.is_error, Some(true));
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(shaped, docs_message_body());
    }

    #[tokio::test]
    async fn a_notify_reply_is_shaped_with_links_dropped_and_etag_injected() {
        // Shaping is uniform even though the docs' notify 202 carries neither:
        // a `_links` object is dropped and a sent ETag surfaces as `_etag`.
        let fake = FakeGocd::replies(
            json!({
                "_links": { "doc": { "href": "https://api.gocd.org/current/#notify-materials" } },
                "message": "The material is now scheduled for an update. Please check relevant pipeline(s) for status."
            }),
            Some("\"deadbeef\"".into()),
        );
        let result = service(fake)
            .notify_git_material(notify_args(json!({ "repository_url": "git://x/y.git" })))
            .await;

        assert_ne!(result.is_error, Some(true));
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(
            shaped,
            json!({
                "message": "The material is now scheduled for an update. Please check relevant pipeline(s) for status.",
                "_etag": "\"deadbeef\""
            })
        );
    }

    #[tokio::test]
    async fn an_unauthorized_notify_surfaces_the_token_hint() {
        let fake = FakeGocd::fails(GocdError::Http { status: 401 });
        let result = service(fake)
            .notify_scm_material(notify_args(json!({ "scm_name": "material_name" })))
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
        for expected in [
            "notify_git_material",
            "notify_hg_material",
            "notify_scm_material",
            "notify_svn_material",
        ] {
            assert!(names.contains(&expected.to_string()), "{expected} missing");
        }
    }
}
