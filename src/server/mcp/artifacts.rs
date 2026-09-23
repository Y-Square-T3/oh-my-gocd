// Artifacts section: GET /go/files/:pipeline_name/:pipeline_counter/
// :stage_name/:stage_counter/:job_name.json — the plain JSON artifact-tree
// listing — and GET .../:job_name/*path_to_file — one artifact file, answered
// as lossy-decoded text with a metadata line. The section's remaining four
// operations (directory-zip download, create, create-multiple, append) stay
// out of scope by recorded decision.

use super::{OmgMcp, tool_error};
use crate::gocd::{GocdReply, artifacts};
use rmcp::model::ContentBlock;
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

#[derive(Debug, Deserialize, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub(crate) struct JobArtifactFile {
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
    /// The artifact-relative path of the file to read, e.g.
    /// "cruise-output/console.log".
    pub path_to_file: String,
}

/// The file tool's advertised answer: one metadata line naming the path, the
/// GoCD content-type (or `unknown`) and the byte size, then the content
/// verbatim.
fn artifact_answer(file: &JobArtifactFile, reply: &GocdReply) -> String {
    let content = reply.body.as_str().unwrap_or_default();
    let content_type = reply.content_type.as_deref().unwrap_or("unknown");
    format!(
        "[artifact] {} | content-type: {content_type} | {} bytes\n\n{content}",
        file.path_to_file,
        content.len()
    )
}

#[tool_router(router = artifacts_tool_router, vis = "pub(crate)")]
impl OmgMcp {
    #[tool(
        description = "List a job's artifacts as a plain JSON tree (GET /go/files/:pipeline_name/:pipeline_counter/:stage_name/:stage_counter/:job_name.json, API v1). Each node carries name, url, type plus folder `files`/leaf `path`/`created_time`/`size`; these objects contain no `_links`, so the uniform shaping returns the tree verbatim and the absolute `url` fields are kept as the pointer for a human to retrieve a file. For a job's console log prefer get_job_console_log, and for its code-review report get_job_code_review — this listing is for everything else. The tool answers the JSON listing only — file and directory downloads, uploads and appends are out of scope. Docs: https://api.gocd.org/current/#artifacts"
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

    #[tool(
        description = "Read one artifact file of a job as text (GET /go/files/:pipeline_name/:pipeline_counter/:stage_name/:stage_counter/:job_name/*path_to_file, API v1). Response bytes are lossy-decoded as UTF-8 and returned in full — a binary artifact comes back as text soup, never as a refusal — with no size cap; the transport's 10s client timeout is the only bound, so ask for small files. The answer is one metadata line, `[artifact] <path> | content-type: <ct> | <N> bytes` — N sizes the returned text — then the content verbatim. Use get_job_artifacts first to see the tree and each file's `url`. For the standard files prefer the derived tools: get_job_console_log for cruise-output/console.log and get_job_code_review for the code-review report. Directory-zip downloads, uploads and appends are out of scope. Docs: https://api.gocd.org/current/#get-artifact-file"
    )]
    async fn get_job_artifact_file(
        &self,
        Parameters(args): Parameters<JobArtifactFile>,
    ) -> CallToolResult {
        let call = artifacts::file(
            &args.pipeline_name,
            &args.pipeline_counter,
            &args.stage_name,
            &args.stage_counter,
            &args.job_name,
            &args.path_to_file,
        );
        match self.api.request(call).await {
            Ok(reply) => {
                CallToolResult::success(vec![ContentBlock::text(artifact_answer(&args, &reply))])
            }
            Err(err) => tool_error(&err),
        }
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
    fn the_artifacts_descriptions_point_at_the_derived_tools() {
        let service = service(FakeGocd::replies(json!({}), None));
        let description = |name: &str| {
            service
                .tool_router
                .list_all()
                .into_iter()
                .find(|t| t.name == name)
                .expect(name)
                .description
                .map(|d| d.to_string())
                .unwrap_or_default()
        };
        let file = description("get_job_artifact_file");
        assert!(file.contains("get_job_console_log"), "got: {file}");
        assert!(file.contains("get_job_code_review"), "got: {file}");
        let listing = description("get_job_artifacts");
        assert!(listing.contains("get_job_console_log"), "got: {listing}");
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

    fn file_args() -> super::JobArtifactFile {
        super::JobArtifactFile {
            pipeline_name: "pipeline1".into(),
            pipeline_counter: "1".into(),
            stage_name: "defaultStage".into(),
            stage_counter: "1".into(),
            job_name: "defaultJob".into(),
            path_to_file: "cruise-output/console.log".into(),
        }
    }

    #[tokio::test]
    async fn get_job_artifact_file_sends_a_raw_version_1_get_to_the_templated_path() {
        let fake = FakeGocd::raw_replies("", None);
        service(fake.clone())
            .get_job_artifact_file(Parameters(file_args()))
            .await;

        assert_eq!(
            fake.recorded(),
            vec![
                GocdCall::get(
                    "files/pipeline1/1/defaultStage/1/defaultJob/cruise-output/console.log"
                )
                .version(1)
                .raw()
            ]
        );
    }

    #[tokio::test]
    async fn a_leading_slash_on_the_path_does_not_double_up() {
        let fake = FakeGocd::raw_replies("", None);
        let mut args = file_args();
        args.path_to_file = "/console.log".into();
        service(fake.clone())
            .get_job_artifact_file(Parameters(args))
            .await;

        assert_eq!(
            fake.recorded()[0].path,
            "files/pipeline1/1/defaultStage/1/defaultJob/console.log"
        );
    }

    #[tokio::test]
    async fn get_job_artifact_file_answers_the_metadata_line_then_the_content() {
        let fake = FakeGocd::raw_replies("boom\n", Some("text/plain; charset=utf-8"));
        let result = service(fake)
            .get_job_artifact_file(Parameters(file_args()))
            .await;

        assert_eq!(result.is_error, Some(false));
        assert_eq!(
            first_text(&result),
            "[artifact] cruise-output/console.log | content-type: text/plain; charset=utf-8 | 5 bytes\n\nboom\n"
        );
    }

    #[tokio::test]
    async fn a_type_less_file_still_answers_its_line_then_the_content() {
        let fake = FakeGocd::raw_replies("", None);
        let result = service(fake)
            .get_job_artifact_file(Parameters(file_args()))
            .await;

        assert_eq!(
            first_text(&result),
            "[artifact] cruise-output/console.log | content-type: unknown | 0 bytes\n\n"
        );
    }

    #[tokio::test]
    async fn binary_soup_comes_back_as_text_never_as_a_refusal() {
        // Two raw bytes, 0xFF is not valid UTF-8: the production lossy mapper
        // turns them into U+FFFD + 'k' — 4 bytes of returned text, which is
        // what the line's size describes.
        let fake = FakeGocd::raw_bytes_replies(&[0xff, b'k'], Some("application/java-archive"));
        let result = service(fake)
            .get_job_artifact_file(Parameters(file_args()))
            .await;

        assert_eq!(result.is_error, Some(false));
        assert_eq!(
            first_text(&result),
            "[artifact] cruise-output/console.log | content-type: application/java-archive | 4 bytes\n\n\u{FFFD}k"
        );
    }

    #[tokio::test]
    async fn an_unauthorized_file_read_surfaces_the_token_hint() {
        let fake = FakeGocd::fails(GocdError::Http { status: 401 });
        let result = service(fake)
            .get_job_artifact_file(Parameters(file_args()))
            .await;

        assert_eq!(result.is_error, Some(true));
        let text = first_text(&result);
        assert!(text.contains("401"), "got: {text}");
        assert!(text.contains("token"), "got: {text}");
    }

    #[tokio::test]
    async fn a_not_found_file_read_surfaces_the_endpoint_hint() {
        let fake = FakeGocd::fails(GocdError::Http { status: 404 });
        let result = service(fake)
            .get_job_artifact_file(Parameters(file_args()))
            .await;

        assert_eq!(result.is_error, Some(true));
        let text = first_text(&result);
        assert!(text.contains("404"), "got: {text}");
        assert!(text.contains("endpoint"), "got: {text}");
    }

    #[test]
    fn the_file_tool_description_cites_the_endpoint_and_its_semantics() {
        let service = service(FakeGocd::replies(json!({}), None));
        let description = service
            .tool_router
            .list_all()
            .into_iter()
            .find(|t| t.name == "get_job_artifact_file")
            .expect("get_job_artifact_file registered")
            .description
            .map(|d| d.to_string())
            .unwrap_or_default();

        assert!(description.contains(
            "GET /go/files/:pipeline_name/:pipeline_counter/:stage_name/:stage_counter/:job_name/*path_to_file"
        ));
        assert!(description.contains("API v1"));
        assert!(description.contains("#get-artifact-file"));
        assert!(description.contains("lossy"), "got: {description}");
        assert!(description.contains("no size cap"), "got: {description}");
    }

    #[test]
    fn the_file_tool_is_merged_into_the_service_router_and_viewable_in_view_mode() {
        let names = service_in(FakeGocd::replies(json!({}), None), Mode::View).tool_names();
        assert!(names.contains(&"get_job_artifact_file".to_string()));
    }
}
