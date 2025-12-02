#![allow(unused)] // TODO: Remove this when more things are implemented

use std::{
    env, fs,
    path::{Path, PathBuf},
    process::exit,
    str::FromStr,
};

use clap::Parser;
use rev_lines::RawRevLines;

use crate::{config::Config, entry::ActivityEntry, opt::Opt};

mod config;
mod entry;
mod format_string;
mod opt;

fn get_xdg_config_home() -> anyhow::Result<PathBuf> {
    match env::var("XDG_CONFIG_HOME") {
        Ok(path) => Ok(PathBuf::from(path)),
        Err(_) => Ok(PathBuf::from_iter([
            env::var("HOME")?,
            String::from(".config"),
        ])),
    }
}

fn get_xdg_data_home() -> anyhow::Result<PathBuf> {
    match env::var("XDG_DATA_HOME") {
        Ok(path) => Ok(PathBuf::from(path)),
        Err(_) => Ok(PathBuf::from_iter([
            env::var("HOME")?,
            String::from(".local"),
            String::from("share"),
        ])),
    }
}

fn default_config_path() -> anyhow::Result<PathBuf> {
    let mut config_home = get_xdg_config_home()?;
    config_home.push("timetracker");
    config_home.push("config.toml");
    Ok(config_home)
}

fn get_last_state_entry(path: &Path) -> anyhow::Result<Option<ActivityEntry>> {
    let file = fs::File::open(path)?;
    let mut rev_lines = RawRevLines::new(file);
    match rev_lines.next() {
        Some(res) => {
            let entry = &String::from_utf8(res?)?;
            Ok(Some(ActivityEntry::from_str(entry)?))
        }
        None => Ok(None),
    }
}

fn main() {
    let opt = Opt::parse();
    if let opt::SubCommand::DumpDefaultConfig = opt.command {
        println!("{}", include_str!("../default_config.toml"));
        exit(0)
    }

    println!(
        "{:#?}",
        get_last_state_entry(&PathBuf::from("./state_sample"))
    );

    let config_path = opt.config.unwrap_or_else(|| default_config_path().unwrap());
    let config_str = match fs::read_to_string(&config_path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!(
                "Failed to read config: {e}\n\n\
                Make sure {config_path:?} exists before running the program!\n\
                You can generate a reference config with the dump-default-config option.\n"
            );
            if let Some(conf_dir) = config_path.parent() {
                eprintln!("  $ mkdir -p {conf_dir:?}");
            }
            eprintln!("  $ timetracker dump-default-config > {config_path:?}");
            exit(1)
        }
    };

    // let config_result = toml::from_str::<Config>(&config_str);
    // println!("{config_result:#?}");
}
