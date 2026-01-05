use clap::{Parser, Subcommand};

/// A linux profiling utility that generates interactive flamegraphs
#[derive(Parser)]
#[command(name = "flg")]
#[command(version = "0.2.0")]
#[command(about = "A linux profiling utility that generates interactive flamegraphs")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Generate flamegraph(s) from perf data file(s)
    Gen {
        /// Output HTML file path
        #[arg(short, long, default_value = "flamegraph.html")]
        output: String,

        /// Input perf data file(s)
        #[arg(required = true)]
        files: Vec<String>,
    },
}
