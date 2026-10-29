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
    /// If unset, tables fit the terminal when stdout is a TTY, unbounded otherwise.
    /// Applies to redirected tables too when explicitly set.
    /// Cells may be truncated. Below minimum column widths, the table may exceed the limit.
    #[arg(long, global = true, env = "MAX_TABLE_WIDTH", verbatim_doc_comment)]
    pub max_table_width: Option<usize>,

    /// Center tables in the provided width
    ///
    /// Uses max_table_width or the terminal width for positioning.
    /// If false, tables align at the left side of the output.
    /// Applies to redirected tables too when max_table_width is set.
    #[arg(long, global = true, env = "CENTER_TABLES", verbatim_doc_comment)]
    pub center_tables: bool,

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
        Self::xdg_dir(self.config_home.as_ref(), "XDG_CONFIG_HOME", ".config")
    }

    pub fn data_home(&self) -> Result<PathBuf> {
        Self::xdg_dir(self.data_home.as_ref(), "XDG_DATA_HOME", ".local/share")
    }

    fn xdg_dir(override_path: Option<&PathBuf>, variable: &str, fallback: &str) -> Result<PathBuf> {
        if let Some(path) = override_path {
            return Ok(path.clone());
        }
        let base = env::var(variable)
            .map(PathBuf::from)
            .or_else(|_| env::var("HOME").map(|home| PathBuf::from(home).join(fallback)))?;
        Ok(base.join("timetrack"))
    }

    pub fn table_options(&self) -> TablePrintOptions {
        TablePrintOptions {
            chars: TableCharOptions::rounded(),
            width: self
                .max_table_width
                .map(TableWidth::Fixed)
                .unwrap_or_default(),
            center: self.center_tables,
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
