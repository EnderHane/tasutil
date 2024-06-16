use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(version, author, about)]
pub(crate) struct Cli {
    pub(crate) path: Option<PathBuf>,

    #[arg(short, long, help = "Recursively scan files in every folders")]
    pub(crate) recursive: bool,

    // #[arg(id = "warp", long, short, help = "Enable feature for warping lobbies")]
    // pub(crate) warp_feature: bool,
}

impl Cli {
    pub(crate) fn get() -> Self {
        Self::parse()
    }
}
