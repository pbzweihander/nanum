use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
    /// S3 bucket name to use
    #[arg(env = "S3_BUCKET_NAME", long, short)]
    pub bucket: String,
}

#[derive(Subcommand)]
pub enum Command {
    /// List uploaded file IDs [alias: ls]
    #[command(alias = "ls")]
    List,
    /// Delete uploaded files [alias: rm]
    #[command(alias = "rm")]
    Delete { ids: Vec<String> },
}
