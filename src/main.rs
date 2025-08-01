use clap::{Parser, Subcommand};
use eyre::Result;
use log::{error, info};
use std::path;
mod commands;
mod serialization;
mod utils;

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
    #[command(subcommand)]
    Dialogs(DialogsCommand),
    Login,
    Logout,
}

#[derive(Debug, Subcommand)]
enum FoldersCommand {
    Backup { dst_file_path: path::PathBuf },
    Restore { src_file_path: path::PathBuf },
    Clear,
}

#[derive(Debug, Subcommand)]
enum DialogsCommand {
    Assign { rules_file_path: path::PathBuf },
}

fn handle_folders_command(
    tokio_rt: &tokio::runtime::Runtime,
    session_file: &path::Path,
    folders_cmd: FoldersCommand,
) -> Result<()> {
    match folders_cmd {
        FoldersCommand::Backup { dst_file_path } => tokio_rt.block_on(
            commands::handle_folders_backup_command(session_file, &dst_file_path),
        )?,
        FoldersCommand::Restore { src_file_path } => tokio_rt.block_on(
            commands::handle_folders_restore_command(session_file, &src_file_path),
        )?,
        FoldersCommand::Clear => {
            tokio_rt.block_on(commands::handle_folders_clear_command(session_file))?
        }
    }
    Ok(())
}

fn handle_dialogs_command(
    tokio_rt: &tokio::runtime::Runtime,
    session_file: &path::Path,
    dialogs_cmd: DialogsCommand,
) -> Result<()> {
    match dialogs_cmd {
        DialogsCommand::Assign { rules_file_path } => tokio_rt.block_on(
            commands::handle_dialogs_assign_command(session_file, &rules_file_path),
        )?,
    }
    Ok(())
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
        Command::Folders(folders_cmd) => {
            handle_folders_command(&tokio_rt, &params.session_file, folders_cmd)?;
        }
        Command::Dialogs(dialogs_cmd) => {
            handle_dialogs_command(&tokio_rt, &params.session_file, dialogs_cmd)?;
        }
    }
    Ok(())
}

fn main() {
    simple_logger::init_with_level(log::Level::Info).unwrap();

    match do_main() {
        Ok(_) => {
            info!("Success")
        }
        Err(e) => {
            error!("Error {e}");
            std::process::exit(1);
        }
    }
}
