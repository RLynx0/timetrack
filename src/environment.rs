use std::{convert::Infallible, env, path::PathBuf};

use clap::Args;
use color_eyre::eyre::Result;

use crate::printable::{TableCharOptions, TablePrintOptions, TableWidth};

#[derive(Debug, Clone, Args)]
pub struct Environment {
    /// Config and activity definitions directory; overrides the XDG config location
    #[arg(long, global = true, env = "TIMETRACK_HOME", value_name = "DIR")]
    pub config_home: Option<PathBuf>,

    /// Activity entries directory; overrides the XDG data location
    #[arg(long, global = true, env = "TIMETRACK_DATA_HOME", value_name = "DIR")]
    pub data_home: Option<PathBuf>,

    /// Maximum table width in terminal columns
    ///
    /// If unset, tables fit the terminal when stdout is a TTY; redirected output
    /// is unbounded. Applies to redirected tables too when explicitly set.
    /// Cells may be truncated. If too small to fit minimum columns, the table
    /// keeps its natural width.
    #[arg(long, global = true, env = "MAX_TABLE_WIDTH")]
    pub max_table_width: Option<usize>,

    /// Force colored output, taking precedence over --no-color
    #[arg(long, global = true, env = "FORCE_COLOR", value_parser = parse_force_color)]
    pub force_color: bool,

    /// Disable colored output
    #[arg(
        long, global = true, env = "NO_COLOR",
        value_parser = parse_no_color, default_value = "0"
    )]
    pub no_color: bool,

    /// Program used by edit to open a timetrack file
    #[arg(short, long, global = true, env = "EDITOR", default_value = "vi")]
    pub editor: String,
}

// Match supports-color's environment semantics, including empty values.
fn parse_force_color(value: &str) -> std::result::Result<bool, Infallible> {
    Ok(value != "false" && value.parse::<usize>().unwrap_or(1) > 0)
}

fn parse_no_color(value: &str) -> std::result::Result<bool, Infallible> {
    Ok(value != "0")
}

impl Environment {
    pub fn config_home(&self) -> Result<PathBuf> {
        if let Some(path) = &self.config_home {
            return Ok(path.clone());
        }
        let base = env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .or_else(|_| env::var("HOME").map(|home| PathBuf::from(home).join(".config")))?;
        Ok(base.join("timetrack"))
    }

    pub fn data_home(&self) -> Result<PathBuf> {
        if let Some(path) = &self.data_home {
            return Ok(path.clone());
        }
        let base = env::var("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|_| env::var("HOME").map(|home| PathBuf::from(home).join(".local/share")))?;
        Ok(base.join("timetrack"))
    }

    pub fn table_options(&self) -> TablePrintOptions {
        TablePrintOptions {
            chars: TableCharOptions::rounded(),
            width: self
                .max_table_width
                .map(TableWidth::Fixed)
                .unwrap_or_default(),
        }
    }

    pub fn apply_color(&self) {
        if self.force_color {
            owo_colors::set_override(true);
        } else if self.no_color {
            owo_colors::set_override(false);
        }
    }
}
