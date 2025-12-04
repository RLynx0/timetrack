use std::path::PathBuf;

pub use clap::{Parser, Subcommand};

use crate::BUILTIN_ACTIVITY_INTERN;

#[derive(Debug, Clone, Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Opt {
    #[command(subcommand)]
    pub command: TtrCommand,

    /// Specify custom config path
    #[clap(short, long)]
    pub config: Option<PathBuf>,
}

#[derive(Debug, Clone, Subcommand)]
pub enum TtrCommand {
    #[command()]
    Start(Start),
    #[command()]
    Idle(Idle),
    #[command()]
    End(End),
    #[command(subcommand)]
    Activity(ActivityCommand),
    #[command()]
    Generate(Generate),
}

/// Edit or list trackable activities
#[derive(Debug, Clone, Subcommand)]
pub enum ActivityCommand {
    #[command()]
    New(AddActivity),
    #[command()]
    Rm(RemoveActivity),
    #[command()]
    Ls(ListActivities),
}

/// Start tracking time for a specified activity
///
/// This ends tracking of the previous activity
#[derive(Debug, Clone, Parser)]
pub struct Start {
    /// Start tracking time for this activity
    #[clap(default_value = BUILTIN_ACTIVITY_INTERN)]
    pub activity: String,

    /// Set the attendance type of this entry
    ///
    /// Subsequent entries will keep using this attendance type by default
    /// The default attendance type is defined by your config
    #[clap(short, long, verbatim_doc_comment)]
    pub attendance: Option<String>,

    /// Set a custom description for this entry
    #[clap(short, long)]
    pub description: Option<String>,

    /// Pollute the terminal with output
    #[clap(short, long)]
    pub verbose: bool,
}

/// Start tracking time for the builtin `Idle` activity
///
/// This is an alias for `start Idle`
#[derive(Debug, Clone, Parser)]
pub struct Idle {
    /// Set the attendance type of this entry
    ///
    /// Subsequent entries will keep using this attendance type by default
    /// The default attendance type is defined by your config
    #[clap(short, long, verbatim_doc_comment)]
    pub attendance: Option<String>,

    /// Set a custom description for this entry
    #[clap(short, long)]
    pub description: Option<String>,

    /// Pollute the terminal with output
    #[clap(short, long)]
    pub verbose: bool,
}

/// Stop tracking time
#[derive(Debug, Clone, Parser)]
pub struct End {
    /// Pollute the terminal with output
    #[clap(short, long)]
    pub verbose: bool,
}

/// Define a new trackable activity
#[derive(Debug, Clone, Parser)]
pub struct AddActivity {
    /// The name of the new activity
    name: String,
}

/// Remove a specified trackable activity
///
/// Entries using this activity will still be valid
/// However, you won't be able to create new ones with it
#[derive(Debug, Clone, Parser)]
#[clap(verbatim_doc_comment)]
pub struct RemoveActivity {
    /// The name of the activity to remove
    name: String,

    /// Allow removing activity hierarchies
    #[clap(short, long)]
    recursive: bool,
}

/// List all trackable activities
///
/// You can think of the activity hierarchy like your filesystem
/// In this context, this command is very similar to the unix ls command
#[derive(Debug, Clone, Parser)]
#[clap(verbatim_doc_comment)]
pub struct ListActivities {
    /// List contents of a given activity category
    name: Option<String>,

    /// Show contents of activity categories
    #[clap(short, long)]
    recursive: bool,
}

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
