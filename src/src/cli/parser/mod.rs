use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug)]
pub struct Args {
    pub directory: PathBuf,
    #[arg(long)]
    pub use_existing_dir: bool,
    #[arg(short, long)]
    pub port: Option<u16>,
    #[arg(long = "remove")]
    pub should_remove: Option<bool>,
    #[arg(long)]
    pub psql: bool,
    #[arg(long)]
    pub silent: bool,
    #[arg(trailing_var_arg = true)]
    pub around: Vec<String>,
}
