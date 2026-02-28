use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Args {
    /// The directory to initialize a db cluster in
    pub directory: PathBuf,
    /// Run postgres in an existing db cluster
    #[arg(long)]
    pub use_existing_dir: bool,
    /// Automatically configure auto-explain
    #[arg(long)]
    pub auto_explain: bool,
    /// Localhost port to listen on
    #[arg(short, long)]
    pub port: Option<u16>,
    /// Control if the directory is deleted when tmp-postgres ends
    #[arg(long = "remove")]
    pub should_remove: Option<bool>,
    /// Start a psql session
    #[arg(long)]
    pub psql: bool,
    /// Silences stdout/stderr forwarding from initdb and postgres
    #[arg(long)]
    pub silent: bool,
    /// Optional command to run when tmp-postgres is ready to receive
    /// connections, when specified tmp-postgres will tear down after the
    /// command exits
    #[arg(trailing_var_arg = true)]
    pub around: Vec<String>,
}
