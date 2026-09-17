[![CI](https://github.com/Y-Square-T3/oh-my-gocd/actions/workflows/ci.yml/badge.svg)](https://github.com/Y-Square-T3/oh-my-gocd/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/oh-my-gocd.svg)](https://crates.io/crates/oh-my-gocd)
[![Crates.io Downloads](https://img.shields.io/crates/d/oh-my-gocd.svg)](https://crates.io/crates/oh-my-gocd)
[![Homebrew](https://img.shields.io/github/v/release/Y-Square-T3/oh-my-gocd?label=homebrew&logo=homebrew)](https://github.com/Y-Square-T3/homebrew-oh-my-gocd)

# oh-my-gocd

[English](README.md) | [简体中文](README.zh-CN.md)

## 安装

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

如果找不到 `omg` 命令，请确认 Cargo 的可执行文件目录已加入 `PATH`，然后重启终端：Linux/macOS 为 `~/.cargo/bin`，Windows 为 `%USERPROFILE%\.cargo\bin`。

### Linux（x86-64）

从[最新发布版本](https://github.com/Y-Square-T3/oh-my-gocd/releases/latest)下载预编译程序，无需安装 Rust。Linux 安装包的目标平台为 `x86_64-unknown-linux-gnu`，需要兼容的、基于 glibc 的发行版；它不是原生 ARM64 或 Alpine/musl 构建。对于其他受支持的目标平台，或系统 glibc 版本过旧的情况，请使用 Cargo 编译安装。

在已安装 `curl` 和 `tar` 的 POSIX 兼容 shell 中运行：

```sh
omg_tmp_dir=$(mktemp -d)
curl -fL https://github.com/Y-Square-T3/oh-my-gocd/releases/latest/download/omg-linux-amd64.tar.gz \
  -o "$omg_tmp_dir/omg-linux-amd64.tar.gz" &&
tar -xzf "$omg_tmp_dir/omg-linux-amd64.tar.gz" -C "$omg_tmp_dir" &&
mkdir -p "$HOME/.local/bin" &&
install -m 755 "$omg_tmp_dir/omg" "$HOME/.local/bin/omg"

export PATH="$HOME/.local/bin:$PATH"
omg --version
```

要在新终端中继续使用 `omg`，请将 `export PATH="$HOME/.local/bin:$PATH"` 添加到 shell 启动文件中（例如 Bash 的 `~/.bashrc` 或 Zsh 的 `~/.zshrc`）。升级时，重复下载和安装步骤即可。

### Windows（x64）

在 PowerShell 中下载并解压预编译程序。此安装方式无需安装 Rust 或 C++ Build Tools：

```powershell
$omgInstallDir = "$env:LOCALAPPDATA\Programs\omg"
$omgArchive = "$env:TEMP\omg-windows-amd64.zip"
Invoke-WebRequest -Uri "https://github.com/Y-Square-T3/oh-my-gocd/releases/latest/download/omg-windows-amd64.zip" -OutFile $omgArchive -ErrorAction Stop
Expand-Archive -Path $omgArchive -DestinationPath $omgInstallDir -Force -ErrorAction Stop

$env:Path = "$omgInstallDir;$env:Path"
omg --version
```

以上 `PATH` 修改仅对当前 PowerShell 会话生效。要永久生效，请打开**编辑账户的环境变量**（Edit environment variables for your account），在**用户变量**（User variables）中编辑 **Path**，添加 `%LOCALAPPDATA%\Programs\omg`。重启终端及 MCP 客户端，使其读取新的路径。

升级时，请先停止正在运行的 `omg` 进程（包括 MCP 服务器），再重复下载和解压步骤。如果使用 WSL，请在 WSL 终端中按照 Linux 安装指南操作。

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
