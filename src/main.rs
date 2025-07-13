use clap::{Parser, Subcommand};
use eyre::Result;
use std::path;

#[derive(Debug, Parser)]
#[command(version, about = "Tool for command-line interaction with Telegram")]
struct CliParams {
    #[command(subcommand)]
    command: Command,
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
    println!("Running with params {params:?}");
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
