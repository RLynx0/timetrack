use clap::{Parser, Subcommand};

#[derive(Debug, Clone, Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Opt {
    #[command(subcommand)]
    command: SubCommand,
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

    /// Define a new trackable activity
    #[command()]
    New {
        /// The name of the new activity
        name: String,
    },

    /// Stop tracking time
    #[command()]
    End,

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
}
