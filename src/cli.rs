use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    #[command(alias("fmt"))]
    Format {
        #[command(subcommand)]
        command: FormatCommand,
    },
    Lobby {
        #[command(subcommand)]
        command: LobbyCommand,
    },
}

#[derive(Subcommand)]
pub enum FormatCommand {
    #[command(alias("sb"))]
    ScanBreakpoint,
}

#[derive(Subcommand)]
pub enum LobbyCommand {
    #[command(alias("i"))]
    Info {
        #[arg(short, long)]
        dir: Option<PathBuf>,
    },
    #[command(alias("r"))]
    Route {
        #[arg(short, long)]
        dir: Option<PathBuf>,
        #[arg(short, long)]
        num: Option<u32>,
        #[arg(long)]
        show_arc: bool,
    },
    #[command(alias("gi"))]
    GenerateInput {
        string: String,
        csv: PathBuf,
        template: PathBuf,
    },
}
