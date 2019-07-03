mod input;
mod settings;

use settings::Settings;
use std::{
    fs::File,
    thread,
    time,
};

fn main() {
    let config_file = File::open("config.yml").expect("can not open config.yml");
    let settings: Settings =
        serde_yaml::from_reader(config_file).expect("can not read config.yml to settings");

    dbg!(&settings);

    settings
        .inputs
        .into_iter()
        .map(|input| input.run())
        .collect::<Result<(), String>>()
        .unwrap();

    loop {
        thread::sleep(time::Duration::from_millis(1000));
    }
}
