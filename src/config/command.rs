use crate::cli::ConfigArgs;
use crate::config::{self, Mode, config_file_path};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

/// Everything the config command touches outside its own logic: injected per test.
pub struct Ctx<'a> {
    pub dir: PathBuf,
    pub stdin: &'a mut dyn BufRead,
    pub stdout: &'a mut dyn Write,
}

/// A literal `-` means: read one line from stdin, trimming the line ending.
fn resolve_input(raw: &str, stdin: &mut dyn BufRead) -> anyhow::Result<String> {
    if raw != "-" {
        return Ok(raw.to_owned());
    }
    let mut line = String::new();
    if stdin.read_line(&mut line)? == 0 {
        anyhow::bail!("expected one line of input on stdin");
    }
    if line.ends_with('\n') {
        line.pop();
    }
    if line.ends_with('\r') {
        line.pop();
    }
    Ok(line)
}

/// Validate a GoCD endpoint: must be an absolute http(s) URL. Returns the
/// URL normalized (via `url`) with one trailing '/' stripped.
fn normalize_endpoint(raw: &str) -> anyhow::Result<String> {
    if raw.trim().is_empty() {
        anyhow::bail!("endpoint must not be empty");
    }
    let parsed = match url::Url::parse(raw.trim()) {
        Ok(u) => u,
        Err(_) => {
            anyhow::bail!(
                "invalid endpoint {raw:?}: it must be an absolute URL, e.g. https://{raw}"
            )
        }
    };
    if !matches!(parsed.scheme(), "http" | "https") {
        anyhow::bail!(
            "endpoint scheme must be http or https, got {:?}",
            parsed.scheme()
        );
    }
    let normalized = parsed.as_str();
    Ok(normalized
        .strip_suffix('/')
        .unwrap_or(normalized)
        .to_owned())
}

/// A string setting omg can save, together with the file section that owns it.
enum Setting {
    Server(&'static str),
    Mcp(&'static str),
}

impl Setting {
    fn key(&self) -> &str {
        match self {
            Setting::Server(key) | Setting::Mcp(key) => key,
        }
    }

    fn write(&self, dir: &Path, value: &str) -> anyhow::Result<()> {
        match self {
            Setting::Server(key) => config::set_server_value(dir, key, value),
            Setting::Mcp(key) => config::set_mcp_value(dir, key, value),
        }
    }
}

pub fn run(args: &ConfigArgs, ctx: &mut Ctx) -> anyhow::Result<()> {
    let path = config_file_path(&ctx.dir);
    if args.list {
        let cfg = config::Config::load(&ctx.dir)?;
        writeln!(ctx.stdout, "config file: {}", path.display())?;
        writeln!(
            ctx.stdout,
            "endpoint: {}",
            cfg.server.endpoint.as_deref().unwrap_or("<unset>")
        )?;
        writeln!(
            ctx.stdout,
            "token: {}",
            cfg.server.token.as_deref().unwrap_or("<unset>")
        )?;
        match cfg.mcp.mode {
            Some(mode) => writeln!(ctx.stdout, "mode: {mode}")?,
            None => writeln!(ctx.stdout, "mode: <unset> (default: {})", Mode::default())?,
        }
        return Ok(());
    }

    // Read and validate every input first: a rejection must leave the file untouched.
    let mut writes: Vec<(Setting, String)> = Vec::new();
    if let Some(raw) = &args.token {
        let token = resolve_input(raw, &mut *ctx.stdin)?;
        if token.trim().is_empty() {
            anyhow::bail!("token must not be empty");
        }
        writes.push((Setting::Server("token"), token));
    }
    if let Some(raw) = &args.endpoint {
        let input = resolve_input(raw, &mut *ctx.stdin)?;
        writes.push((Setting::Server("endpoint"), normalize_endpoint(&input)?));
    }
    if let Some(raw) = &args.mode {
        let mode: Mode = raw.parse()?;
        writes.push((Setting::Mcp("mode"), mode.to_string()));
    }

    for (setting, value) in &writes {
        setting.write(&ctx.dir, value)?;
        writeln!(ctx.stdout, "{} saved to {}", setting.key(), path.display())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Cursor;
    use std::path::Path;

    fn args(token: Option<&str>, endpoint: Option<&str>, list: bool) -> ConfigArgs {
        ConfigArgs {
            token: token.map(str::to_owned),
            endpoint: endpoint.map(str::to_owned),
            mode: None,
            list,
        }
    }

    fn mode_args(mode: Option<&str>) -> ConfigArgs {
        ConfigArgs {
            mode: mode.map(str::to_owned),
            ..args(None, None, false)
        }
    }

    fn invoke(dir: &Path, stdin: &str, a: &ConfigArgs) -> anyhow::Result<Vec<u8>> {
        let mut out = Vec::new();
        let mut ctx = Ctx {
            dir: dir.to_path_buf(),
            stdin: &mut Cursor::new(stdin.as_bytes()),
            stdout: &mut out,
        };
        run(a, &mut ctx)?;
        Ok(out)
    }

    fn read_file(dir: &Path) -> String {
        std::fs::read_to_string(config_file_path(dir)).unwrap()
    }

    fn saved(dir: &Path) -> serde_json::Value {
        serde_json::from_str(&read_file(dir)).unwrap()
    }

    #[test]
    fn setting_token_creates_config_file_with_nested_server_section() {
        let tmp = tempfile::TempDir::new().unwrap();
        let out = invoke(tmp.path(), "", &args(Some("t0k3n"), None, false)).unwrap();

        let saved = saved(tmp.path());
        assert_eq!(saved, json!({ "server": { "token": "t0k3n" } }));
        assert_eq!(
            String::from_utf8(out).unwrap(),
            format!(
                "token saved to {}\n",
                config_file_path(tmp.path()).display()
            )
        );
    }

    #[test]
    fn token_dash_reads_one_line_from_stdin_and_trims_newline() {
        let tmp = tempfile::TempDir::new().unwrap();
        invoke(tmp.path(), "s3cr3t\n", &args(Some("-"), None, false)).unwrap();

        let saved = saved(tmp.path());
        assert_eq!(saved, json!({ "server": { "token": "s3cr3t" } }));
    }

    #[test]
    fn empty_token_is_rejected_and_nothing_is_written() {
        let tmp = tempfile::TempDir::new().unwrap();
        let err = invoke(tmp.path(), "", &args(Some(""), None, false)).unwrap_err();
        assert!(err.to_string().contains("token must not be empty"));
        assert!(!config_file_path(tmp.path()).exists());
    }

    #[test]
    fn whitespace_only_token_is_rejected() {
        let tmp = tempfile::TempDir::new().unwrap();
        let err = invoke(tmp.path(), "", &args(Some("   "), None, false)).unwrap_err();
        assert!(err.to_string().contains("token must not be empty"));
        assert!(!config_file_path(tmp.path()).exists());
    }

    #[test]
    fn token_dash_with_empty_stdin_is_rejected() {
        let tmp = tempfile::TempDir::new().unwrap();
        let err = invoke(tmp.path(), "", &args(Some("-"), None, false)).unwrap_err();
        assert!(err.to_string().contains("stdin"));
        assert!(!config_file_path(tmp.path()).exists());
    }

    #[test]
    fn token_with_surrounding_whitespace_passes_validation_and_is_stored_verbatim() {
        let tmp = tempfile::TempDir::new().unwrap();
        invoke(tmp.path(), "", &args(Some(" padded "), None, false)).unwrap();

        let saved = saved(tmp.path());
        assert_eq!(saved, json!({ "server": { "token": " padded " } }));
    }

    #[test]
    fn setting_endpoint_writes_server_endpoint_and_reports_it() {
        let tmp = tempfile::TempDir::new().unwrap();
        let out = invoke(
            tmp.path(),
            "",
            &args(None, Some("https://gocd.example.com"), false),
        )
        .unwrap();

        let saved = saved(tmp.path());
        assert_eq!(
            saved,
            json!({ "server": { "endpoint": "https://gocd.example.com" } })
        );
        assert_eq!(
            String::from_utf8(out).unwrap(),
            format!(
                "endpoint saved to {}\n",
                config_file_path(tmp.path()).display()
            )
        );
    }

    #[test]
    fn endpoint_strips_trailing_slash() {
        let tmp = tempfile::TempDir::new().unwrap();
        invoke(
            tmp.path(),
            "",
            &args(None, Some("https://gocd.example.com/"), false),
        )
        .unwrap();

        let saved = saved(tmp.path());
        assert_eq!(
            saved["server"]["endpoint"],
            json!("https://gocd.example.com")
        );
    }

    #[test]
    fn endpoint_keeps_non_root_path() {
        let tmp = tempfile::TempDir::new().unwrap();
        invoke(
            tmp.path(),
            "",
            &args(None, Some("https://gocd.example.com/go/"), false),
        )
        .unwrap();

        let saved = saved(tmp.path());
        assert_eq!(
            saved["server"]["endpoint"],
            json!("https://gocd.example.com/go")
        );
    }

    #[test]
    fn endpoint_without_scheme_is_rejected_with_hint() {
        let tmp = tempfile::TempDir::new().unwrap();
        let err = invoke(tmp.path(), "", &args(None, Some("gocd.example.com"), false)).unwrap_err();
        assert!(err.to_string().contains("https://"), "got: {err}");
        assert!(!config_file_path(tmp.path()).exists());
    }

    #[test]
    fn endpoint_with_non_http_scheme_is_rejected() {
        let tmp = tempfile::TempDir::new().unwrap();
        let err = invoke(
            tmp.path(),
            "",
            &args(None, Some("ftp://gocd.example.com"), false),
        )
        .unwrap_err();
        assert!(err.to_string().contains("http"), "got: {err}");
        assert!(!config_file_path(tmp.path()).exists());
    }

    #[test]
    fn http_scheme_is_accepted() {
        let tmp = tempfile::TempDir::new().unwrap();
        invoke(
            tmp.path(),
            "",
            &args(None, Some("http://localhost:8153"), false),
        )
        .unwrap();

        let saved = saved(tmp.path());
        assert_eq!(saved["server"]["endpoint"], json!("http://localhost:8153"));
    }

    #[test]
    fn endpoint_dash_reads_one_line_from_stdin() {
        let tmp = tempfile::TempDir::new().unwrap();
        invoke(
            tmp.path(),
            "https://ci.example.com\n",
            &args(None, Some("-"), false),
        )
        .unwrap();

        let saved = saved(tmp.path());
        assert_eq!(saved["server"]["endpoint"], json!("https://ci.example.com"));
    }

    #[test]
    fn empty_endpoint_is_rejected_and_nothing_is_written() {
        let tmp = tempfile::TempDir::new().unwrap();
        let err = invoke(tmp.path(), "", &args(None, Some(""), false)).unwrap_err();
        assert!(err.to_string().contains("endpoint must not be empty"));
        assert!(!config_file_path(tmp.path()).exists());
    }

    #[test]
    fn setting_both_keys_prints_one_confirmation_line_each() {
        let tmp = tempfile::TempDir::new().unwrap();
        let out = invoke(
            tmp.path(),
            "",
            &args(Some("abc"), Some("https://gocd.example.com"), false),
        )
        .unwrap();

        let out = String::from_utf8(out).unwrap();
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("token saved to "));
        assert!(lines[1].starts_with("endpoint saved to "));
        let saved = saved(tmp.path());
        assert_eq!(
            saved,
            json!({ "server": { "token": "abc", "endpoint": "https://gocd.example.com" } })
        );
    }

    #[test]
    fn a_failed_endpoint_validation_writes_nothing_even_with_valid_token() {
        let tmp = tempfile::TempDir::new().unwrap();
        let err = invoke(
            tmp.path(),
            "",
            &args(Some("good"), Some("not a url"), false),
        )
        .unwrap_err();
        assert!(err.to_string().contains("endpoint"));
        assert!(!config_file_path(tmp.path()).exists());
    }

    #[test]
    fn setting_token_preserves_unrelated_keys_in_the_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            config_file_path(tmp.path()),
            r#"{"server": {"token": "old", "custom": 42, "endpoint": "https://keep.me"}, "theme": "dark"}"#,
        )
        .unwrap();

        invoke(tmp.path(), "", &args(Some("new"), None, false)).unwrap();

        let saved = saved(tmp.path());
        assert_eq!(
            saved,
            json!({
                "server": { "token": "new", "custom": 42, "endpoint": "https://keep.me" },
                "theme": "dark",
            })
        );
    }

    #[test]
    fn setting_token_replaces_unparseable_file() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(config_file_path(tmp.path()), "{ not json ,, }").unwrap();

        invoke(tmp.path(), "", &args(Some("new"), None, false)).unwrap();

        let saved = saved(tmp.path());
        assert_eq!(saved, json!({ "server": { "token": "new" } }));
    }

    #[test]
    fn setting_token_replaces_json_document_that_is_not_an_object() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(config_file_path(tmp.path()), "[1, 2, 3]").unwrap();

        invoke(tmp.path(), "", &args(Some("new"), None, false)).unwrap();

        let saved = saved(tmp.path());
        assert_eq!(saved, json!({ "server": { "token": "new" } }));
    }

    #[test]
    fn setting_token_replaces_server_section_that_is_not_an_object() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            config_file_path(tmp.path()),
            r#"{"server": "junk", "theme": "dark"}"#,
        )
        .unwrap();

        invoke(tmp.path(), "", &args(Some("new"), None, false)).unwrap();

        let saved = saved(tmp.path());
        assert_eq!(
            saved,
            json!({ "server": { "token": "new" }, "theme": "dark" })
        );
    }

    #[test]
    fn setting_token_propagates_unreadable_file_instead_of_replacing_it() {
        let tmp = tempfile::TempDir::new().unwrap();
        // A directory in place of the config file: read fails with something
        // other than NotFound, so silently replacing would destroy state we
        // were never allowed to read.
        std::fs::create_dir(config_file_path(tmp.path())).unwrap();

        let err = invoke(tmp.path(), "", &args(Some("new"), None, false)).unwrap_err();
        assert!(err.to_string().contains("cannot read"), "got: {err}");
    }

    #[test]
    fn writes_back_pretty_json_with_trailing_newline() {
        let tmp = tempfile::TempDir::new().unwrap();
        invoke(tmp.path(), "", &args(Some("tok"), None, false)).unwrap();

        let raw = read_file(tmp.path());
        assert!(raw.ends_with("\n"));
        assert!(raw.contains("\n  \"server\""));
        assert!(raw.contains("\n    \"token\""));
    }

    #[test]
    fn list_without_config_file_shows_path_and_unset_placeholders() {
        let tmp = tempfile::TempDir::new().unwrap();
        let out =
            String::from_utf8(invoke(tmp.path(), "", &args(None, None, true)).unwrap()).unwrap();

        assert_eq!(
            out,
            format!(
                "config file: {}\nendpoint: <unset>\ntoken: <unset>\nmode: <unset> (default: view)\n",
                config_file_path(tmp.path()).display()
            )
        );
    }

    #[test]
    fn list_prints_saved_values_in_plaintext() {
        let tmp = tempfile::TempDir::new().unwrap();
        invoke(
            tmp.path(),
            "",
            &args(Some("s3cr3t"), Some("https://gocd.example.com"), false),
        )
        .unwrap();

        let out =
            String::from_utf8(invoke(tmp.path(), "", &args(None, None, true)).unwrap()).unwrap();
        assert_eq!(
            out,
            format!(
                "config file: {}\nendpoint: https://gocd.example.com\ntoken: s3cr3t\nmode: <unset> (default: view)\n",
                config_file_path(tmp.path()).display()
            )
        );
    }

    #[test]
    fn list_shows_unset_for_keys_that_were_never_set() {
        let tmp = tempfile::TempDir::new().unwrap();
        invoke(tmp.path(), "", &args(Some("onlytoken"), None, false)).unwrap();

        let out =
            String::from_utf8(invoke(tmp.path(), "", &args(None, None, true)).unwrap()).unwrap();
        assert!(out.contains("endpoint: <unset>\n"));
        assert!(out.contains("token: onlytoken\n"));
    }

    #[test]
    fn setting_mode_stores_it_under_the_mcp_section() {
        let tmp = tempfile::TempDir::new().unwrap();
        let out = String::from_utf8(invoke(tmp.path(), "", &mode_args(Some("operate"))).unwrap())
            .unwrap();

        assert_eq!(saved(tmp.path()), json!({ "mcp": { "mode": "operate" } }));
        assert!(out.starts_with("mode saved to "), "got: {out}");
    }

    #[test]
    fn setting_mode_canonicalizes_and_is_accepted_in_all_three_values() {
        for mode in ["view", "operate", "full"] {
            let tmp = tempfile::TempDir::new().unwrap();
            invoke(tmp.path(), "", &mode_args(Some(mode))).unwrap();
            assert_eq!(saved(tmp.path())["mcp"]["mode"], json!(mode));
        }
    }

    #[test]
    fn unknown_mode_is_rejected_naming_the_allowed_values_and_writes_nothing() {
        let tmp = tempfile::TempDir::new().unwrap();
        let err = invoke(tmp.path(), "", &mode_args(Some("veiw"))).unwrap_err();
        let msg = err.to_string();
        for expected in ["veiw", "view", "operate", "full"] {
            assert!(
                msg.contains(expected),
                "error should mention {expected}: {msg}"
            );
        }
        assert!(!config_file_path(tmp.path()).exists());
    }

    #[test]
    fn mode_alias_view_only_is_rejected() {
        let tmp = tempfile::TempDir::new().unwrap();
        let err = invoke(tmp.path(), "", &mode_args(Some("view-only"))).unwrap_err();
        assert!(err.to_string().contains("view"), "got: {err}");
        assert!(!config_file_path(tmp.path()).exists());
    }

    #[test]
    fn setting_mode_and_token_together_writes_both_sections() {
        let tmp = tempfile::TempDir::new().unwrap();
        let out = String::from_utf8(
            invoke(
                tmp.path(),
                "",
                &ConfigArgs {
                    token: Some("abc".into()),
                    mode: Some("full".into()),
                    ..args(None, None, false)
                },
            )
            .unwrap(),
        )
        .unwrap();

        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with("token saved to "));
        assert!(lines[1].starts_with("mode saved to "));
        assert_eq!(
            saved(tmp.path()),
            json!({ "server": { "token": "abc" }, "mcp": { "mode": "full" } })
        );
    }

    #[test]
    fn list_shows_unset_mode_with_its_effective_default() {
        let tmp = tempfile::TempDir::new().unwrap();
        let out =
            String::from_utf8(invoke(tmp.path(), "", &args(None, None, true)).unwrap()).unwrap();

        assert_eq!(
            out,
            format!(
                "config file: {}\nendpoint: <unset>\ntoken: <unset>\nmode: <unset> (default: view)\n",
                config_file_path(tmp.path()).display()
            )
        );
    }

    #[test]
    fn list_shows_the_saved_mode_verbatim() {
        let tmp = tempfile::TempDir::new().unwrap();
        invoke(tmp.path(), "", &mode_args(Some("operate"))).unwrap();

        let out =
            String::from_utf8(invoke(tmp.path(), "", &args(None, None, true)).unwrap()).unwrap();
        assert!(out.contains("mode: operate\n"), "got: {out}");
        assert!(!out.contains("<unset> (default"), "got: {out}");
    }

    #[test]
    fn list_on_corrupt_file_errors_and_names_the_path() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(config_file_path(tmp.path()), "{ broken").unwrap();

        let err = invoke(tmp.path(), "", &args(None, None, true)).unwrap_err();
        assert!(
            err.to_string()
                .contains(&config_file_path(tmp.path()).display().to_string()),
            "error should name the file: {err}"
        );
    }

    #[test]
    fn list_ignores_unknown_keys() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            config_file_path(tmp.path()),
            r#"{"server": {"token": "tk", "custom": 1}, "theme": "dark"}"#,
        )
        .unwrap();

        let out =
            String::from_utf8(invoke(tmp.path(), "", &args(None, None, true)).unwrap()).unwrap();
        assert!(out.contains("token: tk\n"));
        assert!(!out.contains("custom"));
    }

    #[test]
    fn list_with_non_string_token_errors() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(config_file_path(tmp.path()), r#"{"server": {"token": 42}}"#).unwrap();

        invoke(tmp.path(), "", &args(None, None, true)).unwrap_err();
    }
}
