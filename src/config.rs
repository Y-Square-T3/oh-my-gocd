pub mod command;

use serde::Deserialize;
use std::fmt;
use std::path::{Path, PathBuf};

/// The settings omg understands. Unknown keys in the file are ignored here
/// but preserved on write.
#[derive(Debug, Default, PartialEq, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub server: Server,
    #[serde(default)]
    pub mcp: Mcp,
}

#[derive(Debug, Default, PartialEq, Deserialize)]
pub struct Server {
    pub endpoint: Option<String>,
    pub token: Option<String>,
}

/// Settings that govern the omg MCP server itself, not the GoCD connection.
#[derive(Debug, Default, PartialEq, Deserialize)]
pub struct Mcp {
    pub mode: Option<Mode>,
}

/// The MCP exposure ladder: each mode includes every tier below it. Unset is
/// `View` — the safe default the server falls back to.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    #[default]
    View,
    Operate,
    Full,
}

impl Mode {
    /// The canonical spelling — the only accepted form on disk and in the CLI.
    pub fn as_str(self) -> &'static str {
        match self {
            Mode::View => "view",
            Mode::Operate => "operate",
            Mode::Full => "full",
        }
    }

    fn parse(raw: &str) -> Option<Mode> {
        match raw {
            "view" => Some(Mode::View),
            "operate" => Some(Mode::Operate),
            "full" => Some(Mode::Full),
            _ => None,
        }
    }
}

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl std::str::FromStr for Mode {
    type Err = anyhow::Error;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        Mode::parse(raw).ok_or_else(|| {
            anyhow::anyhow!("unknown mode {raw:?}: expected one of view, operate, full")
        })
    }
}

// Hand-rolled so any bad stored value — unknown spelling or non-string —
// reports omg's wording naming the key, the offending value and the three
// canonical values. Nothing else (no aliases, no case-insensitivity) is
// accepted.
impl<'de> Deserialize<'de> for Mode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_json::Value::deserialize(deserializer)?;
        match value.as_str().map(Mode::parse) {
            Some(Some(mode)) => Ok(mode),
            _ => Err(serde::de::Error::custom(format!(
                "invalid mcp.mode {value}: expected one of view, operate, full"
            ))),
        }
    }
}

impl Config {
    /// The single read entry point for omg's configuration. Future layers
    /// (e.g. OMG_TOKEN / OMG_ENDPOINT env overrides) slot in here.
    ///
    /// A missing file yields the default config; an unreadable or invalid
    /// one is an error naming the path.
    pub fn load(dir: &Path) -> anyhow::Result<Config> {
        use anyhow::Context;
        let path = config_file_path(dir);
        let text = match std::fs::read_to_string(&path) {
            Ok(text) => text,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Config::default());
            }
            Err(err) => return Err(err).with_context(|| format!("cannot read {}", path.display())),
        };
        let value: serde_json::Value = serde_json::from_str(&text)
            .map_err(|err| anyhow::anyhow!("{} is not valid JSON: {err}", path.display()))?;
        serde_json::from_value(value)
            .map_err(|err| anyhow::anyhow!("invalid configuration in {}: {err}", path.display()))
    }
}

/// Resolve the omg config directory, XDG-style on every platform.
///
/// `xdg_config_home` and `home_dir` are injected so the rule is testable;
/// callers feed them from the environment.
pub fn resolve_config_dir(
    xdg_config_home: Option<&Path>,
    home_dir: Option<&Path>,
) -> anyhow::Result<PathBuf> {
    let from_env = xdg_config_home
        .filter(|p| !p.as_os_str().is_empty())
        .map(|p| p.join("omg"));
    let from_home = home_dir.map(|p| p.join(".config").join("omg"));
    from_env.or(from_home).ok_or_else(|| {
        anyhow::anyhow!("could not resolve config directory: set $XDG_CONFIG_HOME or $HOME")
    })
}

/// Path to the config file inside a resolved config directory.
pub fn config_file_path(dir: &Path) -> PathBuf {
    dir.join("omg.jsonc")
}

/// Set `server.<key>` to a string value. See [`set_section_value`].
pub fn set_server_value(dir: &Path, key: &str, value: &str) -> anyhow::Result<()> {
    set_section_value(dir, "server", key, value)
}

/// Set `mcp.<key>` to a string value. See [`set_section_value`].
pub fn set_mcp_value(dir: &Path, key: &str, value: &str) -> anyhow::Result<()> {
    set_section_value(dir, "mcp", key, value)
}

/// Set `<section>.<key>` to a string value, preserving everything else in the
/// file. A missing file, or one that does not parse as JSON (or whose root,
/// or the section, is not an object) is replaced. A file that exists but
/// cannot be read is an error: omg must not clobber state it couldn't parse.
fn set_section_value(dir: &Path, section: &str, key: &str, value: &str) -> anyhow::Result<()> {
    use anyhow::Context;
    let path = config_file_path(dir);
    let mut root: serde_json::Value = match std::fs::read_to_string(&path) {
        Ok(text) => serde_json::from_str(&text).unwrap_or_else(|_| serde_json::json!({})),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => serde_json::json!({}),
        Err(err) => {
            return Err(err).with_context(|| format!("cannot read {}", path.display()));
        }
    };
    if !root.is_object() {
        root = serde_json::json!({});
    }
    let section_object = root
        .as_object_mut()
        .unwrap()
        .entry(section)
        .or_insert_with(|| serde_json::json!({}));
    if !section_object.is_object() {
        *section_object = serde_json::json!({});
    }
    section_object
        .as_object_mut()
        .unwrap()
        .insert(key.to_owned(), serde_json::Value::String(value.to_owned()));

    std::fs::create_dir_all(dir)?;
    let rendered = serde_json::to_string_pretty(&root)?;
    std::fs::write(&path, format!("{rendered}\n"))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn xdg_config_home_wins_over_home() {
        let dir = resolve_config_dir(Some(Path::new("/xdg")), Some(Path::new("/home/u"))).unwrap();
        assert_eq!(dir, PathBuf::from("/xdg/omg"));
    }

    #[test]
    fn falls_back_to_home_dot_config() {
        let dir = resolve_config_dir(None, Some(Path::new("/home/u"))).unwrap();
        assert_eq!(dir, PathBuf::from("/home/u/.config/omg"));
    }

    #[test]
    fn empty_xdg_var_is_treated_as_unset() {
        let dir = resolve_config_dir(Some(Path::new("")), Some(Path::new("/home/u"))).unwrap();
        assert_eq!(dir, PathBuf::from("/home/u/.config/omg"));
    }

    #[test]
    fn no_xdg_and_no_home_is_an_error() {
        let err = resolve_config_dir(None, None).unwrap_err();
        assert!(err.to_string().contains("config directory"));
    }

    #[test]
    fn config_file_is_named_omg_jsonc() {
        assert_eq!(
            config_file_path(Path::new("/cfg/omg")),
            PathBuf::from("/cfg/omg/omg.jsonc")
        );
    }

    #[test]
    fn setting_the_mode_writes_the_mcp_section_and_keeps_everything_else() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            config_file_path(tmp.path()),
            r#"{"server": {"token": "tk"}, "theme": "dark"}"#,
        )
        .unwrap();

        set_mcp_value(tmp.path(), "mode", "operate").unwrap();

        let saved: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(config_file_path(tmp.path())).unwrap())
                .unwrap();
        assert_eq!(
            saved,
            json!({
                "server": { "token": "tk" },
                "mcp": { "mode": "operate" },
                "theme": "dark",
            })
        );
    }

    #[test]
    fn setting_the_mode_creates_the_file_when_absent() {
        let tmp = tempfile::TempDir::new().unwrap();
        set_mcp_value(tmp.path(), "mode", "full").unwrap();

        let saved: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(config_file_path(tmp.path())).unwrap())
                .unwrap();
        assert_eq!(saved, json!({ "mcp": { "mode": "full" } }));
    }

    #[test]
    fn an_absent_mode_loads_as_none() {
        let tmp = tempfile::TempDir::new().unwrap();
        assert_eq!(Config::load(tmp.path()).unwrap().mcp.mode, None);
    }

    #[test]
    fn the_mcp_section_mode_parses_into_a_typed_value() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(
            config_file_path(tmp.path()),
            r#"{"mcp": {"mode": "operate"}}"#,
        )
        .unwrap();
        assert_eq!(
            Config::load(tmp.path()).unwrap().mcp.mode,
            Some(Mode::Operate)
        );
    }

    #[test]
    fn a_non_string_mode_is_rejected_naming_the_key_and_the_allowed_ones() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(config_file_path(tmp.path()), r#"{"mcp": {"mode": 42}}"#).unwrap();
        let err = Config::load(tmp.path()).unwrap_err();
        let msg = err.to_string();
        for expected in ["mcp.mode", "42", "view", "operate", "full"] {
            assert!(
                msg.contains(expected),
                "error should mention {expected}: {msg}"
            );
        }
    }

    #[test]
    fn an_unknown_mode_is_rejected_naming_the_value_and_the_allowed_ones() {
        let tmp = tempfile::TempDir::new().unwrap();
        std::fs::write(config_file_path(tmp.path()), r#"{"mcp": {"mode": "veiw"}}"#).unwrap();
        let err = Config::load(tmp.path()).unwrap_err();
        let msg = err.to_string();
        for expected in ["veiw", "view", "operate", "full"] {
            assert!(
                msg.contains(expected),
                "error should mention {expected}: {msg}"
            );
        }
    }
}
