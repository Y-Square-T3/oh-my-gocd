pub mod command;

use serde::Deserialize;
use std::path::{Path, PathBuf};

/// The settings omg understands. Unknown keys in the file are ignored here
/// but preserved on write.
#[derive(Debug, Default, PartialEq, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub server: Server,
}

#[derive(Debug, Default, PartialEq, Deserialize)]
pub struct Server {
    pub endpoint: Option<String>,
    pub token: Option<String>,
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

/// Set `server.<key>` to a string value, preserving everything else in the
/// file. A missing file, or one that does not parse as JSON (or whose root,
/// or `server` section, is not an object) is replaced. A file that exists but
/// cannot be read is an error: omg must not clobber state it couldn't parse.
pub fn set_server_value(dir: &Path, key: &str, value: &str) -> anyhow::Result<()> {
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
    let server = root
        .as_object_mut()
        .unwrap()
        .entry("server")
        .or_insert_with(|| serde_json::json!({}));
    if !server.is_object() {
        *server = serde_json::json!({});
    }
    server
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
}
