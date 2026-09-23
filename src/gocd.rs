pub mod access_tokens;
pub mod agents;
pub mod artifact_store;
pub mod artifacts;
pub mod artifacts_config;
pub mod authorization_config;
pub mod backup_config;
pub mod backups;
pub mod current_user;
pub mod dashboard;
pub mod encryption;
pub mod jobs;
pub mod maintenance_mode;
pub mod notify_materials;
pub mod pipeline_instances;
pub mod pipelines;
pub mod plugin_info;
pub mod server_health;
pub mod server_health_messages;
pub mod stage_instances;
pub mod stages;
pub mod users;
pub mod version;

use anyhow::Context;
use serde_json::{Value, json};
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use url::Url;

/// The GoCD API surface omg consumes: one generic transport call so adding an
/// API section never multiplies trait methods or fake stubs. A trait so the
/// MCP tools can be tested with a recording fake instead of a live server.
pub trait GocdApi: Send + Sync + Debug {
    fn request(
        &self,
        call: GocdCall,
    ) -> Pin<Box<dyn Future<Output = Result<GocdReply, GocdError>> + Send + '_>>;
}

/// The HTTP verbs GoCD documents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GocdVerb {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

/// One GoCD HTTP operation as documented in the API reference: the
/// context-relative path, the documented accept version, and the optional
/// query pairs, JSON body, If-Match ETag and X-GoCD-Confirm header the
/// endpoint defines.
#[derive(Debug, Clone, PartialEq)]
pub struct GocdCall {
    pub verb: GocdVerb,
    pub path: String,
    pub version: u32,
    pub query: Vec<(String, String)>,
    pub body: Option<Value>,
    pub if_match: Option<String>,
    pub confirm: bool,
    pub raw: bool,
}

impl GocdCall {
    pub fn get(path: &str) -> Self {
        Self::new(GocdVerb::Get, path)
    }

    pub fn post(path: &str) -> Self {
        Self::new(GocdVerb::Post, path)
    }

    pub fn put(path: &str) -> Self {
        Self::new(GocdVerb::Put, path)
    }

    pub fn patch(path: &str) -> Self {
        Self::new(GocdVerb::Patch, path)
    }

    pub fn delete(path: &str) -> Self {
        Self::new(GocdVerb::Delete, path)
    }

    fn new(verb: GocdVerb, path: &str) -> Self {
        Self {
            verb,
            path: path.to_owned(),
            version: 1,
            query: Vec::new(),
            body: None,
            if_match: None,
            confirm: false,
            raw: false,
        }
    }

    /// Pin the `application/vnd.go.cd.vN+json` accept version.
    pub fn version(mut self, version: u32) -> Self {
        self.version = version;
        self
    }

    pub fn query(mut self, pairs: Vec<(String, String)>) -> Self {
        self.query = pairs;
        self
    }

    pub fn body(mut self, body: Value) -> Self {
        self.body = Some(body);
        self
    }

    /// Send the ETag back as `If-Match`, GoCD's write guard.
    pub fn etag(mut self, etag: String) -> Self {
        self.if_match = Some(etag);
        self
    }

    /// Send `X-GoCD-Confirm: true`, the confirmation header GoCD documents
    /// for POSTs it shows without a request body.
    pub fn confirm(mut self) -> Self {
        self.confirm = true;
        self
    }

    /// Ask the transport for the raw-bytes response arm: the reply body is
    /// the lossy-UTF-8 text of the response instead of a JSON parse, for
    /// routes like `/go/files/**` that answer with file content.
    pub fn raw(mut self) -> Self {
        self.raw = true;
        self
    }
}

/// GoCD's answer: the body (parsed JSON, or the raw text of a `.raw()` call)
/// plus the ETag header when one was sent and the Content-Type header when a
/// raw reply needs it to describe the bytes it carries.
#[derive(Debug, Clone, PartialEq)]
pub struct GocdReply {
    pub body: Value,
    pub etag: Option<String>,
    pub content_type: Option<String>,
}

impl GocdReply {
    /// The success body as every omg tool advertises it: all `_links` objects
    /// recursively removed, and `_etag` injected only when GoCD sent an ETag
    /// header. Everything else is kept verbatim.
    pub fn shaped(&self) -> Value {
        let mut body = self.body.clone();
        strip_links(&mut body);
        if let (Some(etag), Value::Object(map)) = (&self.etag, &mut body) {
            map.entry("_etag").or_insert_with(|| json!(etag));
        }
        body
    }
}

fn strip_links(value: &mut Value) {
    match value {
        Value::Object(map) => {
            map.remove("_links");
            for child in map.values_mut() {
                strip_links(child);
            }
        }
        Value::Array(items) => {
            for child in items {
                strip_links(child);
            }
        }
        _ => {}
    }
}

/// Turn a JSON response body into the uniform reply every tool advertises:
/// empty answers `Value::Null`, anything else is parsed, and non-JSON is the
/// Decode error. The raw arm lives in [`raw_reply`].
fn map_reply(text: String, etag: Option<String>) -> Result<GocdReply, GocdError> {
    let body = if text.trim().is_empty() {
        Value::Null
    } else {
        serde_json::from_str(&text).map_err(|err| GocdError::Decode(err.to_string()))?
    };
    Ok(GocdReply {
        body,
        etag,
        content_type: None,
    })
}

/// The `.raw()` arm, shared by the transport and the test fake: the response
/// bytes become the lossy-UTF-8 text body — never a parse, never a refusal —
/// with the ETag and Content-Type headers carried along for the tool to
/// describe them.
pub(crate) fn raw_reply(
    bytes: Vec<u8>,
    etag: Option<String>,
    content_type: Option<String>,
) -> GocdReply {
    GocdReply {
        body: Value::String(String::from_utf8_lossy(&bytes).into_owned()),
        etag,
        content_type,
    }
}

/// The live implementation: reqwest against a GoCD server.
#[derive(Debug)]
pub struct HttpGocd {
    http: reqwest::Client,
    api_base: Url,
    token: String,
}

impl HttpGocd {
    pub fn new(endpoint: &str, token: &str) -> anyhow::Result<Self> {
        let http = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .context("cannot create HTTP client")?;
        Ok(Self {
            http,
            api_base: resolve_api_base(endpoint)?,
            token: token.to_owned(),
        })
    }

    /// Absolute URL of an API path, relative to the server's context path.
    fn api_url(&self, path: &str) -> Url {
        self.api_base
            .join(path)
            .unwrap_or_else(|err| panic!("bad API path {path:?}: {err}"))
    }
}

impl GocdApi for HttpGocd {
    fn request(
        &self,
        call: GocdCall,
    ) -> Pin<Box<dyn Future<Output = Result<GocdReply, GocdError>> + Send + '_>> {
        Box::pin(async move {
            let url = self.api_url(&call.path);
            let method = match call.verb {
                GocdVerb::Get => reqwest::Method::GET,
                GocdVerb::Post => reqwest::Method::POST,
                GocdVerb::Put => reqwest::Method::PUT,
                GocdVerb::Patch => reqwest::Method::PATCH,
                GocdVerb::Delete => reqwest::Method::DELETE,
            };
            let mut request = self
                .http
                .request(method, url.clone())
                .bearer_auth(&self.token)
                .header(
                    reqwest::header::ACCEPT,
                    format!("application/vnd.go.cd.v{}+json", call.version),
                )
                .query(&call.query);
            if let Some(body) = &call.body {
                request = request.json(body);
            }
            if let Some(etag) = &call.if_match {
                request = request.header(reqwest::header::IF_MATCH, etag);
            }
            if call.confirm {
                request = request.header("x-gocd-confirm", "true");
            }
            let response = request
                .send()
                .await
                .map_err(|err| GocdError::Transport(format!("{url}: {err}")))?;
            let status = response.status();
            if !status.is_success() {
                return Err(GocdError::Http {
                    status: status.as_u16(),
                });
            }
            let etag = response
                .headers()
                .get(reqwest::header::ETAG)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned);
            let content_type = response
                .headers()
                .get(reqwest::header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned);
            if call.raw {
                let bytes = response
                    .bytes()
                    .await
                    .map_err(|err| GocdError::Decode(err.to_string()))?;
                return Ok(raw_reply(bytes.to_vec(), etag, content_type));
            }
            let text = response
                .text()
                .await
                .map_err(|err| GocdError::Decode(err.to_string()))?;
            map_reply(text, etag)
        })
    }
}

/// Everything that can go wrong on the way to a GoCD answer.
#[derive(Debug, Clone)]
pub enum GocdError {
    /// GoCD answered with an unexpected status.
    Http { status: u16 },
    /// The request never produced a response.
    Transport(String),
    /// The response body was not the JSON we expected.
    Decode(String),
}

impl GocdError {
    /// A human sentence for the MCP tool-error channel: what failed and
    /// which omg setting most plausibly explains it.
    pub fn tool_message(&self) -> String {
        match self {
            GocdError::Http { status: 401 } => {
                "GoCD rejected the token (401) — check `omg config --token`".to_string()
            }
            GocdError::Http { status: 403 } => {
                "GoCD refused this token (403) — it may lack API access".to_string()
            }
            GocdError::Http { status: 404 } => concat!(
                "GoCD answered 404 — check `omg config --endpoint`; ",
                "GoCD also answers 404 for an unknown token"
            )
            .to_string(),
            GocdError::Http { status: 412 } => concat!(
                "GoCD rejected the write (412) — the resource changed since that ETag; ",
                "re-read it and retry with the fresh `_etag`"
            )
            .to_string(),
            GocdError::Http { status } => format!("GoCD responded with HTTP {status}"),
            GocdError::Transport(reason) => {
                format!("could not reach the GoCD server: {reason}")
            }
            GocdError::Decode(reason) => {
                format!("GoCD returned a non-JSON response: {reason}")
            }
        }
    }
}

/// The one place GoCD's context path is resolved. Every API call joins its
/// own `api/...` path onto this base.
///
/// A bare host (no path) gets GoCD's default `/go` context path; an explicit
/// path is honored as-is. The result always ends with `/` so relative joins
/// land inside the base.
pub fn resolve_api_base(endpoint: &str) -> anyhow::Result<Url> {
    let mut base =
        Url::parse(endpoint).with_context(|| format!("invalid endpoint URL: {endpoint}"))?;
    let mut path = base.path().trim_end_matches('/').to_string();
    if path.is_empty() {
        path = "/go".to_string();
    }
    path.push('/');
    base.set_path(&path);
    Ok(base)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_host_gets_the_go_context_path() {
        assert_eq!(
            resolve_api_base("https://gocd.example.com")
                .unwrap()
                .as_str(),
            "https://gocd.example.com/go/"
        );
    }

    #[test]
    fn root_path_gets_the_go_context_path() {
        assert_eq!(
            resolve_api_base("https://gocd.example.com/")
                .unwrap()
                .as_str(),
            "https://gocd.example.com/go/"
        );
    }

    #[test]
    fn explicit_go_path_is_honored() {
        assert_eq!(
            resolve_api_base("https://gocd.example.com/go")
                .unwrap()
                .as_str(),
            "https://gocd.example.com/go/"
        );
    }

    #[test]
    fn custom_context_path_is_honored() {
        assert_eq!(
            resolve_api_base("https://ci.example.com/ci")
                .unwrap()
                .as_str(),
            "https://ci.example.com/ci/"
        );
    }

    #[test]
    fn api_paths_join_cleanly_onto_the_base() {
        let base = resolve_api_base("https://gocd.example.com").unwrap();
        assert_eq!(
            base.join("api/current_user").unwrap().as_str(),
            "https://gocd.example.com/go/api/current_user"
        );
    }

    #[test]
    fn every_api_call_shares_one_resolved_base() {
        let gocd = HttpGocd::new("https://gocd.example.com", "t0k").unwrap();
        assert_eq!(
            gocd.api_url("api/current_user").as_str(),
            "https://gocd.example.com/go/api/current_user"
        );
        assert_eq!(
            gocd.api_url("api/pipelines").as_str(),
            "https://gocd.example.com/go/api/pipelines"
        );
    }

    #[test]
    fn unauthorized_points_at_the_token_setting() {
        let msg = GocdError::Http { status: 401 }.tool_message();
        assert!(msg.contains("401"), "got: {msg}");
        assert!(msg.contains("token"), "got: {msg}");
    }

    #[test]
    fn not_found_points_at_the_endpoint_and_the_token() {
        // GoCD answers 404 both for a wrong path and for an unknown token.
        let msg = GocdError::Http { status: 404 }.tool_message();
        assert!(msg.contains("endpoint"), "got: {msg}");
        assert!(msg.contains("token"), "got: {msg}");
    }

    #[test]
    fn transport_error_names_the_server_as_unreachable() {
        let msg = GocdError::Transport("connection refused".into()).tool_message();
        assert!(msg.contains("reach"), "got: {msg}");
        assert!(msg.contains("connection refused"), "got: {msg}");
    }

    #[test]
    fn decode_error_reports_a_non_json_response() {
        let msg = GocdError::Decode("expected value".into()).tool_message();
        assert!(msg.contains("JSON"), "got: {msg}");
    }

    #[test]
    fn precondition_failure_tells_the_caller_to_reread() {
        let msg = GocdError::Http { status: 412 }.tool_message();
        assert!(msg.contains("412"), "got: {msg}");
        assert!(msg.contains("re-read"), "got: {msg}");
        assert!(msg.contains("_etag"), "got: {msg}");
    }

    #[test]
    fn shaping_strips_links_recursively_and_keeps_everything_else() {
        let reply = GocdReply {
            body: json!({
                "_links": { "self": { "href": "x" } },
                "name": "store",
                "nested": [
                    { "_links": {}, "keep": 1 },
                    { "deeper": { "_links": { "doc": { "href": "y" } }, "v": 2 } }
                ],
                "_embedded": { "config_entries": [{ "_links": {}, "id": 7 }] }
            }),
            etag: None,
            content_type: None,
        };
        assert_eq!(
            reply.shaped(),
            json!({
                "name": "store",
                "nested": [ { "keep": 1 }, { "deeper": { "v": 2 } } ],
                "_embedded": { "config_entries": [{ "id": 7 }] }
            })
        );
    }

    #[test]
    fn shaping_surfaces_the_etag_header_only_when_go_cd_sent_one() {
        let with_etag = GocdReply {
            body: json!({ "name": "store" }),
            etag: Some("\"abc\"".into()),
            content_type: None,
        };
        assert_eq!(
            with_etag.shaped(),
            json!({ "name": "store", "_etag": "\"abc\"" })
        );

        let without_etag = GocdReply {
            body: json!({ "name": "store" }),
            etag: None,
            content_type: None,
        };
        assert_eq!(without_etag.shaped(), json!({ "name": "store" }));
    }

    #[test]
    fn call_builders_default_to_sending_nothing_optional() {
        let call = GocdCall::get("api/current_user").version(1);
        assert_eq!(call.verb, GocdVerb::Get);
        assert_eq!(call.path, "api/current_user");
        assert_eq!(call.version, 1);
        assert!(call.query.is_empty());
        assert_eq!(call.body, None);
        assert_eq!(call.if_match, None);
        assert!(!call.confirm);
        assert!(!call.raw);
    }

    #[test]
    fn the_confirm_builder_pins_the_bodyless_write_confirmation() {
        let call = GocdCall::post("api/pipelines/pipeline1/unpause").confirm();
        assert!(call.confirm);
    }

    #[test]
    fn the_raw_builder_asks_for_the_bytes_response_arm() {
        let call =
            GocdCall::get("files/pipeline1/1/defaultStage/1/defaultJob/cruise-output/console.log")
                .raw();
        assert!(call.raw);
    }

    #[test]
    fn a_raw_reply_keeps_the_text_verbatim_and_carries_the_content_type() {
        let reply = raw_reply(
            "boom\n".as_bytes().to_vec(),
            None,
            Some("text/plain; charset=utf-8".into()),
        );
        assert_eq!(reply.body, json!("boom\n"));
        assert_eq!(
            reply.content_type.as_deref(),
            Some("text/plain; charset=utf-8")
        );
    }

    #[test]
    fn raw_bytes_that_are_not_valid_utf8_come_back_lossy_never_refused() {
        let reply = raw_reply(
            vec![0xff, b'k'],
            None,
            Some("application/java-archive".into()),
        );
        assert_eq!(reply.body, json!("\u{FFFD}k"));
    }

    #[test]
    fn a_raw_reply_never_parses_json_and_keeps_an_empty_body_as_empty_text() {
        let soup = raw_reply(b"{not json".to_vec(), None, None);
        assert_eq!(soup.body, json!("{not json"));
        let empty = raw_reply(vec![], None, None);
        assert_eq!(empty.body, json!(""));
    }

    #[test]
    fn a_plain_reply_still_parses_json_refuses_soup_and_nulls_empty() {
        let parsed = map_reply("{\"a\":1}".into(), None).unwrap();
        assert_eq!(parsed.body, json!({ "a": 1 }));
        assert!(matches!(
            map_reply("not json".into(), None),
            Err(GocdError::Decode(_))
        ));
        let empty = map_reply("  ".into(), None).unwrap();
        assert_eq!(empty.body, Value::Null);
    }

    #[test]
    fn shaping_leaves_a_raw_text_reply_untouched_and_unstamped() {
        let reply = GocdReply {
            body: json!("log line"),
            etag: Some("\"abc\"".into()),
            content_type: Some("text/plain".into()),
        };
        assert_eq!(reply.shaped(), json!("log line"));
    }
}
