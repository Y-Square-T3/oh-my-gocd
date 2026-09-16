[![CI](https://github.com/Y-Square-T3/oh-my-gocd/actions/workflows/ci.yml/badge.svg)](https://github.com/Y-Square-T3/oh-my-gocd/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/oh-my-gocd.svg)](https://crates.io/crates/oh-my-gocd)
[![Crates.io Downloads](https://img.shields.io/crates/d/oh-my-gocd.svg)](https://crates.io/crates/oh-my-gocd)
[![Homebrew](https://img.shields.io/github/v/release/Y-Square-T3/oh-my-gocd?label=homebrew&logo=homebrew)](https://github.com/Y-Square-T3/homebrew-oh-my-gocd)

# oh-my-gocd

## Installation

### Homebrew (macOS)

```sh
brew install Y-Square-T3/homebrew-oh-my-gocd/omg
```

### Cargo (all platforms)

```sh
cargo install oh-my-gocd
```

## Configuration

omg stores its settings in `omg.jsonc`, resolved XDG-style on every platform:

- `$XDG_CONFIG_HOME/omg/omg.jsonc` if `XDG_CONFIG_HOME` is set, otherwise
- `~/.config/omg/omg.jsonc` (on Windows: `%USERPROFILE%\.config\omg\omg.jsonc`)

```sh
# Save credentials (both flags at once, or either alone)
omg config --token 'gocd-bearer-token' --endpoint 'https://gocd.example.com/go'

# Keep secrets out of your shell history — '-' reads one line from stdin
security find-generic-password -w gocd-token | omg config -T -

# Show what is saved (plaintext, as stored)
omg config --list
```

The file is JSON:

```jsonc
{
  "server": {
    "endpoint": "https://gocd.example.com/go",
    "token": "gocd-bearer-token"
  }
}
```

Notes:

- `--token` must be non-empty. `--endpoint` must be an absolute `http(s)` URL; a
  trailing `/` is stripped on save.
- `omg config` rewrites the file on every set: your other JSON keys are kept,
  but comments are dropped. Remove a value by editing the file directly.
- `omg config` with no flags prints help and exits with code 2;
  `--list` cannot be combined with `--token`/`--endpoint`.
