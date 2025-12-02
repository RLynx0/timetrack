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
    /// Start tracking time for a specified activity
    ///
    /// This ends tracking of the previous activity
    #[command()]
    Start {
        /// Start tracking time for this activity
        #[clap(default_value = "Idle")]
        activity: String,
    },

    /// Stop tracking time
    #[command()]
    End,

    /// Define a new trackable activity
    #[command()]
    Add {
        /// The name of the new activity
        name: String,
    },

    /// Remove a specified trackable activity
    #[command()]
    Remove {
        /// The name of the activity to remove
        name: String,
    },

    /// List all trackable activities
    #[command()]
    List,

    /// Generate output file for a specified time frame
    #[command()]
    Generate {
        /// Print to stdout instead of saving to file
        #[clap(short, long)]
        stdout: bool,

        /// Save to custom filepath
        #[clap(short, long)]
        file_path: Option<String>,
    },

    /// Print the default configuration to stdout and exit
    #[command()]
    DumpDefaultConfig,
}
