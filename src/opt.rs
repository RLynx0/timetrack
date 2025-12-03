use std::path::PathBuf;

pub use clap::{Parser, Subcommand};

#[derive(Debug, Clone, Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Opt {
    #[command(subcommand)]
    pub command: SubCommand,

    /// Specify custom config path
    #[clap(short, long)]
    pub config: Option<PathBuf>,
}

#[derive(Debug, Clone, Subcommand)]
pub enum SubCommand {
    #[command()]
    Start(Start),
    #[command()]
    End(End),
    #[command()]
    New(New),
    #[command()]
    Remove(Remove),
    #[command()]
    List(List),
    #[command()]
    Generate(Generate),
}

/// Start tracking time for a specified activity
///
/// This ends tracking of the previous activity
#[derive(Debug, Clone, Parser)]
pub struct Start {
    /// Start tracking time for this activity
    #[clap(default_value = "Idle")]
    pub activity: String,
}

/// Stop tracking time
#[derive(Debug, Clone, Parser)]
pub struct End;

/// Define a new trackable activity
#[derive(Debug, Clone, Parser)]
pub struct New {
    /// The name of the new activity
    name: String,
}

/// Remove a specified trackable activity
///
/// Entries using this activity will still be valid,
/// but you won't be able to create new ones with it.
#[derive(Debug, Clone, Parser)]
pub struct Remove {
    /// The name of the activity to remove
    name: String,

    /// Allow removing activity hierarchies
    #[clap(short, long)]
    recursive: bool,
}

/// List all trackable activities
#[derive(Debug, Clone, Parser)]
pub struct List;

/// Generate output file for a specified time frame
#[derive(Debug, Clone, Parser)]
pub struct Generate {
    /// Print to stdout instead of saving to file
    #[clap(short, long)]
    stdout: bool,

    /// Save to custom filepath
    #[clap(short, long)]
    file_path: Option<String>,
}
