use std::{
    fs,
    io::{Write, stdin, stdout},
};

use color_eyre::eyre::Result;

use crate::{cli, config::Config, environment::Environment, files, print_smart_list};

pub fn list_attendance_types(
    list_opts: &cli::ListAttendanceTypes,
    env: &Environment,
) -> Result<()> {
    let config = Config::load(env)?;
    let mut list = config.attendance_types.into_iter().collect::<Vec<_>>();
    list.sort_by(|(_, va), (_, vb)| va.cmp(vb));
    if list_opts.machine_readable {
        for (number, hint) in list {
            println!("{number}\t{hint}")
        }
    } else {
        print_smart_list!(list);
    }
    Ok(())
}

pub fn make_guided_config(env: &Environment) -> Result<()> {
    let config_path = files::get_main_config_path(env)?;
    let default = toml::from_str::<Config>(include_str!("../assets/default_config.toml"))
        .expect("Default config must be valid");

    println!("Let me guide you through creating a configuration file!");
    let employee_name = get_input_string("Your Name")?;
    let employee_number = get_input_string("Your employee id")?;
    let cost_center = get_input_string("Your cost center")?;
    let performance_type = get_input_string("Your performance type")?;
    let accounting_cycle = get_input_string("Your accounting cycle")?;

    println!("\nOkay, that's it!");
    let default_attendance = &default.default_attendance;
    println!("\nI have set your default attendance type to {default_attendance}.",);
    println!("This is probably what you want, but in case it isn't -");
    println!("You can always manually edit the generated config file.");
    println!("There are a few more options there that were also set automatically!");

    let config = Config {
        employee_name,
        employee_number,
        cost_center,
        performance_type,
        accounting_cycle,
        ..default
    };

    let config_str = toml::to_string(&config)?;
    if let Some(p) = config_path.parent() {
        fs::create_dir_all(p)?;
    }
    fs::write(&config_path, config_str)?;
    println!("Your config has been saved to {config_path:?}.");
    Ok(())
}

fn get_input_string(query: &str) -> Result<String> {
    let mut input = String::new();
    while input.trim().is_empty() {
        print!("{query}: ");
        stdout().flush()?;
        stdin().read_line(&mut input)?;
    }
    Ok(input.trim().into())
}
