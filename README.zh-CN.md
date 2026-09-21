[![CI](https://github.com/Y-Square-T3/oh-my-gocd/actions/workflows/ci.yml/badge.svg)](https://github.com/Y-Square-T3/oh-my-gocd/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/oh-my-gocd.svg)](https://crates.io/crates/oh-my-gocd)
[![Crates.io Downloads](https://img.shields.io/crates/d/oh-my-gocd.svg)](https://crates.io/crates/oh-my-gocd)
[![Homebrew](https://img.shields.io/github/v/release/Y-Square-T3/oh-my-gocd?label=homebrew&logo=homebrew)](https://github.com/Y-Square-T3/homebrew-oh-my-gocd)

# oh-my-gocd

[English](README.md) | [简体中文](README.zh-CN.md)

## 安装

### Linux（x86-64 / ARM64）

```sh
curl --proto '=https' --tlsv1.2 -fsSL https://github.com/Y-Square-T3/oh-my-gocd/releases/latest/download/install.sh | sh
```

无需安装 Rust 工具链。安装程序会校验发布文件、将 `omg` 安装到 `~/.local/bin`，并在需要时将该目录加入 shell 的 `PATH`。如果安装程序修改了 shell 配置，请重启终端。预编译程序需要兼容的、基于 glibc 的发行版；Alpine/musl 或 glibc 版本较旧的系统请使用下方的 Cargo 安装方式。

### Windows（x64）

在 PowerShell 中运行：

```powershell
irm https://github.com/Y-Square-T3/oh-my-gocd/releases/latest/download/install.ps1 | iex
```

无需安装 Rust 工具链或 Visual Studio C++ Build Tools。安装程序会校验发布文件、将 `omg` 安装到 `%LOCALAPPDATA%\Programs\omg`，并加入用户 `PATH`。请重启已经打开的终端和 MCP 客户端。如果使用 WSL，请在 WSL 终端中运行 Linux 安装命令。

### Homebrew（macOS）

```sh
brew install Y-Square-T3/homebrew-oh-my-gocd/omg
```

### Cargo（所有平台）

先通过 [rustup](https://rust-lang.org/tools/install/) 安装最新稳定版 Rust 工具链。在 Windows 上，请按提示安装 Visual Studio C++ Build Tools。在 Linux 上，需要 C 编译器和链接器（例如 Debian/Ubuntu 的 `build-essential`）。

```sh
cargo install oh-my-gocd
omg --version
```

如果找不到 `omg` 命令，请确认 Cargo 的可执行文件目录已加入 `PATH`，然后重启终端：Linux/macOS 为 `~/.cargo/bin`，Windows 为 `%USERPROFILE%\.cargo\bin`。运行前，也可以在[最新发布版本](https://github.com/Y-Square-T3/oh-my-gocd/releases/latest)页面下载安装包并检查安装脚本。升级时重新运行所选的安装命令即可。

## 配置

配置文件位于 `~/.config/omg/omg.jsonc`；如果设置了 `XDG_CONFIG_HOME`，则使用 `$XDG_CONFIG_HOME/omg/omg.jsonc`：

```sh
omg config --token 'gocd-bearer-token' --endpoint 'https://gocd.example.com/go'

# 避免将密钥留在 shell 历史记录中：'-' 从标准输入读取一行
security find-generic-password -w gocd-token | omg config -T -

# 设置 MCP 服务器使用令牌时允许执行的操作（参见下方“安全模式”）
omg config --mode operate

omg config --list
```

## MCP 服务器

omg 可以通过标准输入/输出（stdio）提供 [Model Context Protocol](https://modelcontextprotocol.io) 服务：

```sh
omg server --mcp
```

服务器使用已保存的 `server.endpoint` 和 `server.token` 进行认证，缺少任意一项都会导致启动失败。它通过标准输入/输出传输 JSON-RPC 消息，因此应由 MCP 客户端启动，而不是手动交互运行。

### 安全模式

每个工具都属于三个权限层级之一，已保存的 `mcp.mode` 决定 MCP 会话能使用哪些层级。每种模式都包含低于它的层级：

| 模式 | 开放的操作 |
| --- | --- |
| `view`（默认） | 仅读取：纯 GET 工具，包括管理类读取操作。 |
| `operate` | 额外开放日常写操作：调度和暂停流水线、运行和取消阶段及作业、评论、物料通知、备份调度、制品存储和配置更新、Agent 更新。 |
| `full` | 额外开放高风险操作：所有删除和批量删除、令牌撤销、用户及授权配置写入、维护模式、强制终止 Agent 任务。 |

```sh
omg config --mode view      # Agent 仅可读取（默认模式）
omg config --mode operate   # 日常流水线操作
omg config --mode full      # 开放全部操作，包括删除
```

- 未设置时使用 `view`，默认仅开放读取操作。**升级注意**：早期版本会开放全部工具，因此需要写入 GoCD 的工作流必须显式设置 `omg config --mode operate`（或 `full`）。
- 模式在服务器启动时读取；修改后需要重启 MCP 客户端会话才能生效。运行 `omg config --list` 可查看已保存的值。

工具如何分级、如何限制隐藏工具的调用，以及设计原因，参见 [ADR 0001](docs/adr/0001-mcp-security-mode-ladder.md)。完整工具列表见 [docs/mcp-tools.md](docs/mcp-tools.md)。

### Codex

安装 `omg` 并通过 `omg config` 保存 GoCD 地址和令牌后（参见[配置](#配置)），使用 Codex CLI 注册 stdio 服务器：

```sh
codex mcp add omg -- omg server --mcp
codex mcp list
```

也可以将以下配置添加到 `~/.codex/config.toml`（对于受信任的项目，也可使用 `.codex/config.toml`）：

```toml
[mcp_servers.omg]
command = "omg"
args = ["server", "--mcp"]
```

请确保 Codex 进程的 `PATH` 中包含 `omg`，或者将 `command` 设置为已安装程序的绝对路径。Codex 会启动服务器，omg 会读取已保存的配置；无需在 Codex 配置中填写 GoCD 令牌。

启动新的 Codex 会话，运行 `/mcp` 检查连接。然后可以尝试：“使用 omg 列出我的 GoCD 流水线。”可用工具由配置的[安全模式](#安全模式)决定；如需日常写操作，请启用 `operate`，并在修改模式后重启会话。

更多配置选项参见 [Codex MCP 文档](https://developers.openai.com/codex/mcp/)。

### opencode

将以下配置添加到项目的 `opencode.json` 中（如需全局生效，则添加到 `~/.config/opencode/opencode.json`）：

```json
{
  "$schema": "https://opencode.ai/config.json",
  "mcp": {
    "omg": {
      "type": "local",
      "command": ["omg", "server", "--mcp"],
      "enabled": true
    }
  }
}
```

重启 opencode 以加载配置。
