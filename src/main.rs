// SPDX-FileCopyrightText: 2023 Benedikt Bastin
//
// SPDX-License-Identifier: AGPL-3.0-or-later

#![deny(clippy::all)]

use std::{
    error::Error,
    fs::File,
    io::BufReader,
    path::{Path, PathBuf},
};

use config::{Config, Providers};
use providers::hetzner::HetznerProvider;
use simple_logger::SimpleLogger;
mod config;
pub mod providers;
mod server;

/// Find first path for the configuration where a file is present
fn find_config() -> Option<PathBuf> {
    if Path::new("./config.json").is_file() {
        return Some(PathBuf::from("./config.json"));
    }

    if Path::new("/etc/dyndns/config.json").is_file() {
        return Some(PathBuf::from("/etc/dyndns/config.json"));
    }

    None
}

// @TODO this should read in a config file containing multiple users
// For now, this will not work, but all other functions expect a Vec of users,
// therefore this will be packed up into an array of exactly one user.
fn load_config(path: &Path) -> Result<config::User, Box<dyn Error>> {
    let reader = BufReader::new(File::open(path)?);

    Ok(serde_json::from_reader(reader)?)
}

#[tokio::main]
pub async fn main() -> Result<(), rocket::Error> {
    SimpleLogger::new()
        .with_level(log::LevelFilter::Warn)
        .init()
        .unwrap();

    let path = match find_config() {
        Some(path) => path,
        None => panic!("Error finding config: No config found"),
    };

    let config = match load_config(&path) {
        Ok(user) => Config { users: vec![user] },
        Err(error) => panic!("Error reading config: {error}"),
    };

    let providers = Providers {
        hetzner_provider: Some(HetznerProvider::new()),
        mock_provider: None,
    };

    server::rocket()
        .manage(config)
        .manage(providers)
        .launch()
        .await?;

    Ok(())
}
