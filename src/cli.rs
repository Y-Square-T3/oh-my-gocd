use clap::{Args, Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(name = "omg", version, about = "Oh, my GoCD!")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Manage omg configuration
    #[command(arg_required_else_help = true)]
    Config(ConfigArgs),

    /// Run omg as a server
    #[command(arg_required_else_help = true)]
    Server(ServerArgs),
}

#[derive(Args, Debug)]
pub struct ServerArgs {
    /// Serve the Model Context Protocol over stdio.
    #[arg(long)]
    pub mcp: bool,
}

#[derive(Args, Debug)]
pub struct ConfigArgs {
    /// GoCD bearer token. Pass '-' to read one line from stdin.
    #[arg(short = 'T', long, value_name = "TOKEN")]
    pub token: Option<String>,

    /// GoCD server endpoint URL. Pass '-' to read one line from stdin.
    #[arg(short = 'E', long, value_name = "ENDPOINT")]
    pub endpoint: Option<String>,

    /// Print the saved configuration.
    #[arg(long, conflicts_with_all = ["token", "endpoint"])]
    pub list: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::error::ErrorKind;

    fn parse(args: &[&str]) -> Result<Cli, clap::Error> {
        Cli::try_parse_from(args)
    }

    fn config_args(args: &[&str]) -> ConfigArgs {
        match parse(args).unwrap().command {
            Command::Config(a) => a,
            Command::Server(_) => panic!("expected config"),
        }
    }

    #[test]
    fn bare_config_exits_with_help_code() {
        let err = parse(&["omg", "config"]).unwrap_err();
        assert_eq!(
            err.kind(),
            ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
        );
        assert_eq!(err.exit_code(), 2);
    }

    #[test]
    fn config_accepts_short_flags() {
        let args = config_args(&[
            "omg",
            "config",
            "-T",
            "abc",
            "-E",
            "https://gocd.example.com",
        ]);
        assert_eq!(args.token.as_deref(), Some("abc"));
        assert_eq!(args.endpoint.as_deref(), Some("https://gocd.example.com"));
        assert!(!args.list);
    }

    #[test]
    fn config_accepts_list_flag() {
        let args = config_args(&["omg", "config", "--list"]);
        assert!(args.list);
        assert_eq!(args.token, None);
        assert_eq!(args.endpoint, None);
    }

    #[test]
    fn config_list_conflicts_with_token() {
        let err = parse(&["omg", "config", "--list", "-T", "abc"]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::ArgumentConflict);
        assert_eq!(err.exit_code(), 2);
    }

    #[test]
    fn config_list_conflicts_with_endpoint() {
        let err = parse(&["omg", "config", "--list", "-E", "https://x.dev"]).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::ArgumentConflict);
    }

    #[test]
    fn server_accepts_mcp_flag() {
        match parse(&["omg", "server", "--mcp"]).unwrap().command {
            Command::Server(a) => assert!(a.mcp),
            Command::Config(_) => panic!("expected server"),
        }
    }

    #[test]
    fn bare_server_exits_with_help_code() {
        let err = parse(&["omg", "server"]).unwrap_err();
        assert_eq!(
            err.kind(),
            ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand
        );
        assert_eq!(err.exit_code(), 2);
    }
}
