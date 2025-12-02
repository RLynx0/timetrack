#![allow(unused)] // TODO: Remove this when more things are implemented

use std::{
    fs,
    path::{Path, PathBuf},
    process::exit,
    str::FromStr,
};

use clap::Parser;
use rev_lines::RawRevLines;

use crate::{config::Config, entry::ActivityEntry, opt::Opt};

mod config;
mod entry;
mod files;
mod format_string;
mod opt;

fn main() {
    let opt = Opt::parse();

    let config = match load_or_create_config(opt.config) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to load or create config: {e}");
            exit(1)
        }
    };

    println!("{config:?}")

    // let config_result = toml::from_str::<Config>(&config_str);
    // println!("{config_result:#?}");
}

fn load_or_create_config(custom_path: Option<PathBuf>) -> anyhow::Result<Config> {
    let config_path = match custom_path {
        None => files::default_config_path()?,
        Some(p) => p,
    };
    if fs::exists(&config_path)? {
        let config_str = fs::read_to_string(config_path)?;
        Ok(toml::from_str(&config_str)?)
    } else {
        let config = make_guided_config();
        let config_str = toml::to_string(&config)?;
        if let Some(p) = config_path.parent() {
            fs::create_dir_all(p)?;
        }
        fs::write(&config_path, config_str)?;
        println!("Saved generated configuration to {config_path:?}");
        Ok(config)
    }
}

fn make_guided_config() -> Config {
    let default = toml::from_str::<Config>(include_str!("../default_config.toml"))
        .expect("Default config must be valid");

    default
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
