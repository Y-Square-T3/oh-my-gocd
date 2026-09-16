use clap::Parser;
use std::path::Path;

mod cli;
mod config;
mod gocd;
mod server;

use cli::{Cli, Command};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let xdg = std::env::var_os("XDG_CONFIG_HOME");
    let home = home::home_dir();
    let dir = config::resolve_config_dir(xdg.as_deref().map(Path::new), home.as_deref())?;
    match cli.command {
        Command::Config(args) => {
            let stdin = std::io::stdin();
            let stdout = std::io::stdout();
            let mut ctx = config::command::Ctx {
                dir,
                stdin: &mut stdin.lock(),
                stdout: &mut stdout.lock(),
            };
            config::command::run(&args, &mut ctx)?;
            Ok(())
        }
        Command::Server(args) => server::command::run(&args, &dir).await,
    }
}
