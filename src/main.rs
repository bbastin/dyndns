// SPDX-FileCopyrightText: 2023 Benedikt Bastin
//
// SPDX-License-Identifier: AGPL-3.0-or-later

#![deny(clippy::all)]

use std::{error::Error, fs::File, io::BufReader};

use config::Config;
use simple_logger::SimpleLogger;
mod config;
pub mod providers;
mod server;

// @TODO this should read in a config file containing multiple users
// For now, this will not work, but all other functions expect a Vec of users,
// therefore this will be packed up into an array of exactly one user.
fn load_config() -> Result<config::User, Box<dyn Error>> {
    let reader = BufReader::new(File::open("config.json")?);

    Ok(serde_json::from_reader(reader)?)
}

#[tokio::main]
pub async fn main() -> Result<(), rocket::Error> {
    SimpleLogger::new().init().unwrap();

    let config = match load_config() {
        Ok(user) => Config { users: vec![user] },
        Err(error) => panic!("Error reading config: {}", error),
    };

    server::rocket().manage(config).launch().await?;

    Ok(())
}
