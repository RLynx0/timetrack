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

/// Show latest tracked activity or activities
#[derive(Debug, Clone, Parser)]
pub struct Show {
    /// Specify what you want to see
    #[clap(default_value = "entries")]
    pub mode: ShowMode,

    /// Specify how many activities should be shown
    ///
    /// - <n>                Show the last <n> tracked activities
    /// - <n>h | <n>hours    Show activities in the last <n> hours
    /// - <n>d | <n>days     Show activities in the last <n> days
    /// - <n>w | <n>weeks    Show activities in the last <n> weeks
    /// - <n>m | <n>months   Show activities in the last <n> months
    /// - 0                  Show the currently tracked activity
    /// - hour               Show activities from the current hour
    /// - day                Show activities from the current day
    /// - week               Show activities from the current week
    /// - month              Show activities from the current month
    #[clap(verbatim_doc_comment, short, long, default_value = "0")]
    pub last: ActivityRange,

    /// Print machine readable values instead of a formatted table
    #[clap(short, long)]
    pub machine_readable: bool,
}
#[derive(Debug, Clone, ValueEnum)]
pub enum ShowMode {
    /// Show individual activity entries
    Entries,
    /// Show a summary of tracked activities
    Collapsed,
    /// Show the total tracked time
    Time,
}

/// Open the activity log in an editor
///
/// Set the EDITOR environment variable to use a specific program
#[derive(Debug, Clone, Parser)]
pub struct Edit;

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

/// Define a new trackable activity
#[derive(Debug, Clone, Parser)]
pub struct SetActivity {
    /// The name of the trackable activity
    pub name: String,

    /// The wbs to use for this activity
    pub wbs: String,

    /// The default description for this activity
    #[clap(short, long)]
    pub description: Option<String>,

    /// Allow overwriting existing activities
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

/// Rename a specified trackable activity
///
/// Existing entries using this activity will retain the old name
#[derive(Debug, Clone, Parser)]
#[clap(verbatim_doc_comment)]
pub struct MoveActivity {
    /// The name of the activity to rename
    pub from: String,

    /// The new name of the activity
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
