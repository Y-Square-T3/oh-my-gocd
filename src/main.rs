use clap::Parser;
use std::path::Path;

mod cli;
mod config;

use cli::{Cli, Command};

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Config(args) => {
            let xdg = std::env::var_os("XDG_CONFIG_HOME");
            let home = home::home_dir();
            let dir = config::resolve_config_dir(xdg.as_deref().map(Path::new), home.as_deref())?;
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
    }
}
