use clap::{Parser, Subcommand};
use eyre::Result;
use std::path;
mod commands;

#[derive(Debug, Parser)]
#[command(version, about = "Tool for command-line interaction with Telegram")]
struct CliParams {
    #[command(subcommand)]
    command: Command,
    #[arg(long)]
    session_file: path::PathBuf,
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(subcommand)]
    Folders(FoldersCommand),
    Login,
    Logout,
}

#[derive(Debug, Subcommand)]
enum FoldersCommand {
    Backup { dst_file_path: path::PathBuf },
    Restore { src_file_path: path::PathBuf },
}

fn do_main() -> Result<()> {
    let params = CliParams::parse();
    let tokio_rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    match params.command {
        Command::Login => {
            tokio_rt.block_on(commands::handle_login_command(&params.session_file))?;
        }
        Command::Logout => {
            tokio_rt.block_on(commands::handle_logout_command(&params.session_file))?;
        }
        _ => {
            todo!();
        }
    }
    Ok(())
}

fn main() {
    match do_main() {
        Ok(_) => {
            println!("Success")
        }
        Err(e) => {
            println!("Error {e}");
            std::process::exit(1);
        }
    }
}
