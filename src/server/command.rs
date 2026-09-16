use crate::cli::ServerArgs;
use crate::config::Config;
use crate::gocd::HttpGocd;
use crate::server::mcp::OmgMcp;
use rmcp::{ServiceExt, transport::stdio};
use std::path::Path;
use std::sync::Arc;

/// Entry point for `omg server`: validates the chosen mode, fails fast on
/// unusable config, then serves MCP over stdio until the client disconnects.
pub async fn run(args: &ServerArgs, dir: &Path) -> anyhow::Result<()> {
    if !args.mcp {
        anyhow::bail!("no server mode selected — pass --mcp");
    }
    let config = Config::load(dir)?;
    let (endpoint, token) = resolve_server_settings(&config)?;
    let api = Arc::new(HttpGocd::new(&endpoint, &token)?);
    let service = OmgMcp::new(api)
        .serve(stdio())
        .await
        .map_err(|err| anyhow::anyhow!("cannot start MCP server: {err}"))?;
    service
        .waiting()
        .await
        .map_err(|err| anyhow::anyhow!("MCP server stopped: {err}"))?;
    Ok(())
}

/// The GoCD connection settings the MCP server needs, or a startup error
/// naming every missing key and the command that sets it.
pub fn resolve_server_settings(config: &Config) -> anyhow::Result<(String, String)> {
    let endpoint = nonblank(config.server.endpoint.as_deref());
    let token = nonblank(config.server.token.as_deref());
    let mut missing = Vec::new();
    if endpoint.is_none() {
        missing.push("server.endpoint (set it with `omg config --endpoint`)");
    }
    if token.is_none() {
        missing.push("server.token (set it with `omg config --token`)");
    }
    if !missing.is_empty() {
        anyhow::bail!("cannot start MCP server: missing {}", missing.join(" and "));
    }
    Ok((endpoint.unwrap(), token.unwrap()))
}

fn nonblank(value: Option<&str>) -> Option<String> {
    let trimmed = value.map(str::trim).filter(|s| !s.is_empty())?;
    Some(trimmed.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Server;

    fn config(endpoint: Option<&str>, token: Option<&str>) -> Config {
        Config {
            server: Server {
                endpoint: endpoint.map(str::to_owned),
                token: token.map(str::to_owned),
            },
        }
    }

    #[test]
    fn complete_config_yields_endpoint_and_token() {
        let (endpoint, token) =
            resolve_server_settings(&config(Some("https://g.example.com"), Some("t0k"))).unwrap();
        assert_eq!(endpoint, "https://g.example.com");
        assert_eq!(token, "t0k");
    }

    #[test]
    fn missing_both_names_both_keys_and_points_at_config_command() {
        let err = resolve_server_settings(&config(None, None)).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("endpoint"), "got: {msg}");
        assert!(msg.contains("token"), "got: {msg}");
        assert!(msg.contains("omg config"), "got: {msg}");
    }

    #[test]
    fn missing_token_names_only_token() {
        let err =
            resolve_server_settings(&config(Some("https://g.example.com"), None)).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("token"), "got: {msg}");
        assert!(!msg.contains("endpoint"), "got: {msg}");
    }

    #[test]
    fn blank_token_counts_as_missing() {
        let err = resolve_server_settings(&config(Some("https://g.example.com"), Some("   ")))
            .unwrap_err();
        assert!(err.to_string().contains("token"), "got: {err}");
    }

    #[tokio::test]
    async fn server_without_mcp_mode_is_refused_before_touching_config() {
        let tmp = tempfile::TempDir::new().unwrap();
        let err = run(&ServerArgs { mcp: false }, tmp.path())
            .await
            .unwrap_err();
        assert!(err.to_string().contains("--mcp"), "got: {err}");
    }
}
