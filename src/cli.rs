use clap::ValueEnum;
pub use clap::{Parser, Subcommand};

use crate::{activity_range::ActivityRange, trackable::BUILTIN_ACTIVITY_IDLE_NAME};

#[derive(Debug, Clone, Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: TtrCommand,
}

#[derive(Debug, Clone, Subcommand)]
pub enum TtrCommand {
    #[command()]
    Start(Start),
    #[command()]
    End(End),
    #[command()]
    Show(Show),
    #[command()]
    Edit(Edit),
    #[command()]
    Generate(Generate),
    #[command(subcommand)]
    Activity(ActivityCommand),

    // Convenience Commands
    /// Easily generate a timetrack configuration file
    #[command()]
    MakeConfig,
    #[command()]
    ListAttendanceTypes(ListAttendanceTypes),
}

/// Edit or list trackable activities
#[derive(Debug, Clone, Subcommand)]
pub enum ActivityCommand {
    #[command()]
    Set(SetActivity),
    #[command()]
    Rm(RemoveActivity),
    #[command()]
    Mv(MoveActivity),
    #[command()]
    Ls(ListActivities),
}

/// Start tracking time for a specified activity
///
/// This ends tracking of the previous activity
#[derive(Debug, Clone, Parser)]
pub struct Start {
    /// Start tracking time for this activity
    #[clap(default_value = BUILTIN_ACTIVITY_IDLE_NAME)]
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

/// Stop tracking time
#[derive(Debug, Clone, Parser)]
pub struct End {
    /// Pollute the terminal with output
    #[clap(short, long)]
    pub verbose: bool,
}

/// Show tracked activity or activities within a range
#[derive(Debug, Clone, Parser)]
pub struct Show {
    /// Specify what you want to see
    #[clap(default_value = "entries")]
    pub mode: Vec<ShowMode>,

    /// Select results by count, relative time, date bucket, or range
    ///
    /// Counts:
    /// - 0       Show only the currently active activity (default)
    /// - <n>     Show the last <n> completed results plus the active result, if any
    ///
    /// Relative periods (include the current bucket):
    /// - <n>h | <n>hours    The last <n> hours plus the current hour
    /// - <n>d | <n>days     The last <n> days plus the current day
    /// - <n>w | <n>weeks    The last <n> weeks plus the current week
    /// - <n>m | <n>months   The last <n> months plus the current month
    /// - hour, day, week, month  Aliases for 0h, 0d, 0w, 0m
    ///
    /// Absolute buckets:
    /// - YYYY-MM | YYYY-MM-DD        Select that month or day
    /// - YYYY..YYYY                  Select that year (a bare number is a count)
    ///
    /// Ranges use inclusive calendar buckets. A relative endpoint is measured from now,
    /// except in Bucket..Nunit, where the right endpoint is measured forward from Bucket.
    /// A relative endpoint before an absolute bucket (Nunit..Bucket) is not supported.
    /// Counts may only be used alone.
    ///
    /// Examples:
    /// - 3m                         Three completed months and the current month
    /// - 2026-02                    The February 2026 bucket
    /// - 2025-02-03..2026-01-16     From February 3, 2025 through January 16, 2026
    /// - 2026-01-01..7d             From January 1 through the bucket seven days later
    /// - 3m..1m                     From the bucket three months ago through one month ago
    /// - ..3d                       Everything through the bucket three days ago
    /// - 3d..                       From the bucket three days ago through now
    /// - ..                         Everything recorded through now
    #[clap(verbatim_doc_comment, short, long, default_value = "0")]
    pub range: ActivityRange,

    /// Print machine readable values instead of a formatted table
    #[clap(short, long)]
    pub machine_readable: bool,
}
#[derive(Debug, Clone, ValueEnum, PartialEq, Eq, Hash)]
pub enum ShowMode {
    /// Show individual activity entries
    Entries,
    /// Show a summary of tracked activities
    Collapsed,
    /// Show daily time and attendance, derived from selected activities
    Attendance,
    /// Show the total tracked time, derived from selected activities
    Time,
}

/// Open a timetrack work file in an editor
#[derive(Debug, Clone, Parser)]
pub struct Edit {
    /// Specify the timetrack file to open
    #[clap(default_value = "entries")]
    pub target: EditTarget,

    /// Specify a program to open the file with
    #[clap(short, long, env = "EDITOR", default_value = "vi")]
    pub editor: String,
}
#[derive(Debug, Clone, ValueEnum)]
pub enum EditTarget {
    /// Open the entry log file
    Entries,
    /// Open the config file
    Config,
    /// Open the activity definition file
    Activities,
}

/// Generate output file for a specified time frame
#[derive(Debug, Clone, Parser)]
pub struct Generate {
    /// Print to stdout instead of saving to file
    #[clap(short, long)]
    pub stdout: bool,

    /// Save to custom filepath
    #[clap(short, long)]
    pub file_path: Option<String>,
}

/// Define a new trackable activity
///
/// Missing categories in the activity path are created automatically.
#[derive(Debug, Clone, Parser)]
pub struct SetActivity {
    /// The name of the trackable activity
    pub name: String,

    /// The wbs to use for this activity
    pub wbs: String,

    /// The default description for this activity
    #[clap(short, long)]
    pub description: Option<String>,

    /// Allow overwriting an existing activity, but not a category
    #[clap(short, long)]
    pub force: bool,
}

/// Remove a specified trackable activity
///
/// Existing entries using this activity will still be valid
/// However, you won't be able to create new ones with it
#[derive(Debug, Clone, Parser)]
#[clap(verbatim_doc_comment)]
pub struct RemoveActivity {
    /// The name of the activity to remove
    pub name: String,

    /// Allow removing activity hierarchies
    #[clap(short, long)]
    pub recursive: bool,
}

/// Move or rename a trackable activity or category
///
/// Existing entries using this activity will retain the old name
#[derive(Debug, Clone, Parser)]
#[clap(verbatim_doc_comment)]
pub struct MoveActivity {
    /// The path of the activity or category to move
    pub from: String,

    /// The new path, or an existing category to move into
    pub to: String,
}

/// List all trackable activities
///
/// You can think of the activity hierarchy like your filesystem
/// In this context, this command is very similar to the unix ls command
#[derive(Debug, Clone, Parser)]
#[clap(verbatim_doc_comment)]
pub struct ListActivities {
    /// List contents of a given activity category
    pub name: Option<String>,

    /// Show contents of activity categories
    #[clap(short, long)]
    pub recursive: bool,

    /// Collapse activity categories with only one entry
    ///
    /// Does nothing when --recursive is passed.
    #[clap(short, long)]
    pub collapsed: bool,

    /// Print machine readable values instead of a formatted table
    #[clap(short, long)]
    pub machine_readable: bool,
}

/// Print out configured attendance types
#[derive(Debug, Clone, Parser)]
pub struct ListAttendanceTypes {
    /// Print machine readable values instead of a formatted list
    #[clap(short, long)]
    pub machine_readable: bool,
}
