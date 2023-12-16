// SPDX-FileCopyrightText: 2023 Benedikt Bastin
//
// SPDX-License-Identifier: AGPL-3.0-or-later

#![deny(clippy::all)]

use std::{net::IpAddr, str::FromStr};

use log::{info, warn};
use rocket::{get, routes, State};

use crate::{
    config::Config,
    providers::{hetzner::HetznerProvider, Provider},
};

#[get("/update?<user>&<password>&<host>&<ip>&<ip6>")]
async fn update(
    user: &str,
    password: &str,
    host: &str,
    ip: &str,
    ip6: &str,
    config: &State<Config>,
) -> String {
    let user_config = config.users.iter().find(|u| u.name == user);

    if user_config.is_none() {
        warn!("Invalid user {}", user);
        return "Invalid user".to_string();
    }
    let user = user_config.unwrap();

    if user.password != password {
        warn!("Wrong password for user {}", user.name);
        return "Invalid user".to_string();
    }

    let domain_config = user.domains.iter().find(|d| d.host == host);

    if domain_config.is_none() {
        warn!("Invalid domain {} for user {}", host, user.name);
        return "Invalid domain".to_string();
    }
    let domain_config = domain_config.unwrap();

    // @TODO Determine correct ProviderType
    let p = HetznerProvider::new(&domain_config.apitoken);

    info!("Received IP addresses: IPv4 {}, IPv6: {}", ip, ip6);

    // @TODO
    // if !ip.is_empty() {
    // }

    if !ip6.is_empty() {
        let new_ip = IpAddr::from_str(ip6).unwrap();
        let updated = p.update_ip(host.to_string(), domain_config.zone.clone(), new_ip);

        return match updated {
            Ok(u) => match u {
                true => "Updated IPv6 successfully".to_string(),
                false => "IPv6 already set correctly".to_string(),
            },
            Err(e) => format!("Error: {}", e),
        };
    }

    "No valid IP adress found".to_string()
}

// #[launch]
pub fn rocket() -> rocket::Rocket<rocket::Build> {
    rocket::build().mount("/", routes![update])
}
