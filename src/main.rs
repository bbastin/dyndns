// SPDX-FileCopyrightText: 2023 Benedikt Bastin
//
// SPDX-License-Identifier: AGPL-3.0-or-later

#![deny(clippy::all)]

use std::{env, error::Error, fs::File, io::BufReader, path::{Path, PathBuf}};

use log::{info};
use config::Config;
use simple_logger::SimpleLogger;
mod config;
mod ipv6prefix;
mod providers;
mod server;

/// Find first path for the configuration where a file is present
fn find_config() -> Option<PathBuf> {
    if Path::new("./config.json").is_file() {
        info!("loading config from ./config.json");
        return Some(PathBuf::from("./config.json"));
    }

    if Path::new("/etc/dyndns/config.json").is_file() {
        info!("loading config from /etc/dyndns/config.json");
        return Some(PathBuf::from("/etc/dyndns/config.json"));
    }
    
    let maybe_config_file_path = env::var("DYNDNS_CONFIG_FILE_PATH");
    if maybe_config_file_path.is_ok() {
        let config_file_path = maybe_config_file_path.unwrap();
        info!("found env config path DYNDNS_CONFIG_FILE_PATH=${config_file_path}");
        if Path::new(config_file_path.as_str()).is_file()  {
            info!("loading config from ${config_file_path}");
            return Some(PathBuf::from(config_file_path));
        }
    }

    None
}

fn load_config(path: &Path) -> Result<Config, Box<dyn Error>> {
    let reader = BufReader::new(File::open(path)?);

    Ok(serde_json::from_reader(reader)?)
}

#[tokio::main]
pub async fn main() -> Result<(), Box<rocket::Error>> {
    SimpleLogger::new()
        // .with_level(log::LevelFilter::Warn)
        .init()
        .unwrap();

    let path = match find_config() {
        Some(path) => path,
        None => panic!("Error finding config: No config found"),
    };

    let config = match load_config(&path) {
        Ok(config) => config,
        Err(error) => panic!("Error reading config: {error}"),
    };

    server::rocket().manage(config).launch().await?;

    Ok(())
}
