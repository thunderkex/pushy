use clap::{Parser, Subcommand};

#[derive(Parser, Debug)]
#[command(
    name = "pushy",
    version,
    about = "Simple, zero-config file uploader with automatic fallback",
    subcommand_precedence_over_arg = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    #[arg(value_name = "FILE")]
    pub files: Vec<String>,

    #[arg(short = 'p', long = "provider")]
    pub provider: Option<String>,

    #[arg(short = 'e', long = "expire")]
    pub expire: Option<String>,

    #[arg(long = "name")]
    pub name: Option<String>,

    #[arg(short = 'q', long = "quiet")]
    pub quiet: bool,

    /// Upload to all available providers simultaneously
    #[arg(short = 'a', long = "all")]
    pub all: bool,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    Setup,
    Providers,
    Config,
}
