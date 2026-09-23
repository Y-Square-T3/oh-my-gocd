// Artifacts section: GET /go/files/:pipeline_name/:pipeline_counter/
// :stage_name/:stage_counter/:job_name.json — the plain JSON artifact-tree
// listing only. The section's other five documented operations (file and
// directory downloads, create, create-multiple, append) stay out of scope by
// recorded decision: their payloads the JSON-only seam cannot carry.

use super::OmgMcp;
use crate::gocd::artifacts;
use rmcp::schemars::JsonSchema;
use rmcp::{handler::server::wrapper::Parameters, model::CallToolResult, tool, tool_router};
use serde::Deserialize;

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub(crate) struct JobArtifactsPath {
    /// The pipeline name, e.g. "pipeline1".
    pub pipeline_name: String,
    /// The pipeline counter, e.g. "1".
    pub pipeline_counter: String,
    /// The stage name, e.g. "defaultStage".
    pub stage_name: String,
    /// The stage counter, e.g. "1".
    pub stage_counter: String,
    /// The job name, e.g. "defaultJob".
    pub job_name: String,
}

#[tool_router(router = artifacts_tool_router, vis = "pub(crate)")]
impl OmgMcp {
    #[tool(
        description = "List a job's artifacts as a plain JSON tree (GET /go/files/:pipeline_name/:pipeline_counter/:stage_name/:stage_counter/:job_name.json, API v1). Each node carries name, url, type plus folder `files`/leaf `path`/`created_time`/`size`; these objects contain no `_links`, so the uniform shaping returns the tree verbatim and the absolute `url` fields are kept as the pointer for a human to retrieve a file. The tool answers the JSON listing only — file and directory downloads, uploads and appends are out of scope. Docs: https://api.gocd.org/current/#artifacts"
    )]
    async fn get_job_artifacts(
        &self,
        Parameters(args): Parameters<JobArtifactsPath>,
    ) -> CallToolResult {
        self.request_shaped(artifacts::listing(
            &args.pipeline_name,
            &args.pipeline_counter,
            &args.stage_name,
            &args.stage_counter,
            &args.job_name,
        ))
        .await
    }
}

#[cfg(test)]
mod tests {
    use crate::config::Mode;
    use crate::gocd::{GocdCall, GocdError};
    use crate::server::mcp::fake::{FakeGocd, first_text, service, service_in};
    use rmcp::handler::server::wrapper::Parameters;
    use serde_json::{Value, json};

    fn listing_args() -> super::JobArtifactsPath {
        super::JobArtifactsPath {
            pipeline_name: "pipeline1".into(),
            pipeline_counter: "1".into(),
            stage_name: "defaultStage".into(),
            stage_counter: "1".into(),
            job_name: "defaultJob".into(),
        }
    }

    // Tree taken from the GoCD API docs' JSON-listing example; these objects
    // carry no `_links`, and the absolute `url` fields are kept on purpose.
    fn docs_artifact_tree() -> Value {
        json!({
            "name": "defaultJob",
            "url": "http://ci.example.com/go/files/pipeline1/1/defaultStage/1/defaultJob",
            "type": "folder",
            "path": "",
            "created_time": null,
            "size": null,
            "files": [
                {
                    "name": "logs",
                    "url": "http://ci.example.com/go/files/pipeline1/1/defaultStage/1/defaultJob/logs",
                    "type": "folder",
                    "path": "logs",
                    "created_time": null,
                    "size": null,
                    "files": [
                        {
                            "name": "unit-test-output7227182830818981820.txt",
                            "url": "http://ci.example.com/go/files/pipeline1/1/defaultStage/1/defaultJob/logs/unit-test-output7227182830818981820.txt",
                            "type": "file",
                            "path": "logs/unit-test-output7227182830818981820.txt",
                            "created_time": "12 Dec 2016 07:44:57 +8000",
                            "size": "506 Bytes"
                        }
                    ]
                },
                {
                    "name": "test-results.xml",
                    "url": "http://ci.example.com/go/files/pipeline1/1/defaultStage/1/defaultJob/test-results.xml",
                    "type": "file",
                    "path": "test-results.xml",
                    "created_time": "12 Dec 2016 07:44:57 +8000",
                    "size": "1.17 KB"
                }
            ]
        })
    }

    #[tokio::test]
    async fn get_job_artifacts_sends_a_version_1_get_to_the_json_listing_path() {
        let fake = FakeGocd::replies(json!({}), None);
        service(fake.clone())
            .get_job_artifacts(Parameters(listing_args()))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![GocdCall::get("files/pipeline1/1/defaultStage/1/defaultJob.json").version(1)]
        );
    }

    #[tokio::test]
    async fn get_job_artifacts_yields_the_tree_verbatim_with_urls_kept_and_no_etag() {
        let fake = FakeGocd::replies(docs_artifact_tree(), None);
        let result = service(fake)
            .get_job_artifacts(Parameters(listing_args()))
            .await;

        assert_ne!(result.is_error, Some(true));
        let shaped: Value = serde_json::from_str(&first_text(&result)).unwrap();
        assert_eq!(shaped, docs_artifact_tree());
        assert!(shaped.get("_etag").is_none());
    }

    #[tokio::test]
    async fn an_unauthorized_read_surfaces_the_token_hint() {
        let fake = FakeGocd::fails(GocdError::Http { status: 401 });
        let result = service(fake)
            .get_job_artifacts(Parameters(listing_args()))
            .await;

        assert_eq!(result.is_error, Some(true));
        let text = first_text(&result);
        assert!(text.contains("401"), "got: {text}");
        assert!(text.contains("token"), "got: {text}");
    }

    #[tokio::test]
    async fn a_not_found_read_surfaces_the_endpoint_and_token_hint() {
        let fake = FakeGocd::fails(GocdError::Http { status: 404 });
        let result = service(fake)
            .get_job_artifacts(Parameters(listing_args()))
            .await;

        assert_eq!(result.is_error, Some(true));
        let text = first_text(&result);
        assert!(text.contains("404"), "got: {text}");
        assert!(text.contains("endpoint"), "got: {text}");
    }

    #[test]
    fn the_tool_description_cites_the_documented_endpoint() {
        let service = service(FakeGocd::replies(json!({}), None));
        let description = service
            .tool_router
            .list_all()
            .into_iter()
            .find(|t| t.name == "get_job_artifacts")
            .expect("get_job_artifacts registered")
            .description
            .map(|d| d.to_string())
            .unwrap_or_default();

        assert!(description.contains(
            "GET /go/files/:pipeline_name/:pipeline_counter/:stage_name/:stage_counter/:job_name.json"
        ));
        assert!(description.contains("API v1"));
        assert!(description.contains("#artifacts"));
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
        assert!(names.contains(&"get_job_artifacts".to_string()));
    }

    #[test]
    fn view_mode_exposes_the_artifacts_listing() {
        let names = service_in(FakeGocd::replies(json!({}), None), Mode::View).tool_names();
        assert!(names.contains(&"get_job_artifacts".to_string()));
    }
}
