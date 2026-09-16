pub mod user;

use anyhow::Context;
use serde_json::Value;
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use url::Url;

/// The GoCD API surface omg consumes. A trait so the MCP tools can be tested
/// with a fake instead of a live server.
pub trait GocdApi: Send + Sync + Debug {
    fn fetch_current_user(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Value, GocdError>> + Send + '_>>;
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
    fn fetch_current_user(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<Value, GocdError>> + Send + '_>> {
        Box::pin(async move {
            let url = self.api_url("api/current_user");
            let response = self
                .http
                .get(url.clone())
                .bearer_auth(&self.token)
                .header(reqwest::header::ACCEPT, "application/vnd.go.cd.v1+json")
                .send()
                .await
                .map_err(|err| GocdError::Transport(format!("{url}: {err}")))?;
            let status = response.status();
            if !status.is_success() {
                return Err(GocdError::Http {
                    status: status.as_u16(),
                });
            }
            response
                .json::<Value>()
                .await
                .map_err(|err| GocdError::Decode(err.to_string()))
        })
    }
}

/// Everything that can go wrong on the way to a GoCD answer.
#[derive(Debug)]
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
}
