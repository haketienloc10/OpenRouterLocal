use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "openrouter-local",
    version,
    about = "Local OpenRouter-compatible gateway"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Run the HTTP gateway server
    Serve,
    /// Start the HTTP gateway in the background
    Start,
    /// Stop the background HTTP gateway process
    Stop,
    /// Restart the background HTTP gateway process
    Restart,
    /// Show server logs
    Logs {
        /// Follow log output
        #[arg(short, long)]
        follow: bool,
        /// Number of trailing lines to show
        #[arg(short = 'n', long, default_value_t = 200)]
        lines: usize,
    },
}
