// SPDX-FileCopyrightText: 2023 Benedikt Bastin
//
// SPDX-License-Identifier: AGPL-3.0-or-later

#![deny(clippy::all)]

use std::{
    net::{Ipv4Addr, Ipv6Addr},
    str::FromStr,
};

use log::{error, info, warn};
use rocket::{get, http::Status, routes, State};

use crate::{
    config::Config,
    providers::{hetzner::HetznerProvider, update_ipv4, update_ipv6},
};

#[get("/update?<user>&<password>&<host>&<ip>&<ip6>")]
async fn update(
    user: &str,
    password: &str,
    host: &str,
    ip: Option<&str>,
    ip6: Option<&str>,
    config: &State<Config>,
) -> (Status, String) {
    let user_config = config.users.iter().find(|u| u.name == user);

    if user_config.is_none() {
        warn!("Invalid user {}", user);
        return (Status::Unauthorized, "Invalid user".to_string());
    }
    let user = user_config.unwrap();

    if user.password != password {
        warn!("Wrong password for user {}", user.name);
        return (Status::Unauthorized, "Invalid user".to_string());
    }

    let domain_config = user.domains.iter().find(|d| d.host == host);

    if domain_config.is_none() {
        warn!("Invalid domain {} for user {}", host, user.name);
        return (Status::BadRequest, "Invalid domain".to_string());
    }
    let domain_config = domain_config.unwrap();

    // @TODO Determine correct ProviderType
    let p = HetznerProvider::new(&domain_config.apitoken);

    info!(
        "Received IP addresses: IPv4 {}, IPv6: {}",
        ip.unwrap_or("<empty>"),
        ip6.unwrap_or("<empty>")
    );

    let parsed_ipv4 = match ip.is_some_and(|s| !s.is_empty()) {
        true => match Ipv4Addr::from_str(ip.unwrap()) {
            Ok(i) => Some(i),
            Err(_) => return (Status::BadRequest, "Invalid IPv4 address".to_string()),
        },
        false => None,
    };

    let parsed_ipv6 = match ip6.is_some_and(|s| !s.is_empty()) {
        true => match Ipv6Addr::from_str(ip6.unwrap()) {
            Ok(i) => Some(i),
            Err(_) => return (Status::BadRequest, "Invalid IPv6 address".to_string()),
        },
        false => None,
    };

    let mut status_code = Status::Ok;
    let mut response: String = Default::default();

    if parsed_ipv4.is_some() {
        let res = update_ipv4(&p, &parsed_ipv4.unwrap(), domain_config);

        match res {
            Ok(s) => {
                response += &format!("{}\n", s).to_string();
                info!("{}", s);
            }
            Err(e) => {
                response += &format!("Error updating IPv4 address: {}\n", e).to_string();
                status_code = Status::InternalServerError;
                error!("{}", e);
            }
        }
    }

    if ip6.is_some_and(|s| !s.is_empty()) {
        let res = update_ipv6(&p, &parsed_ipv6.unwrap(), domain_config);

        match res {
            Ok(s) => {
                response += &format!("{}\n", s).to_string();
                info!("{}", s);
            }
            Err(e) => {
                response += &format!("Error updating IPv6 address: {}\n", e).to_string();
                status_code = Status::InternalServerError;
                error!("{}", e);
            }
        }
    }

    if response.is_empty() {
        return (Status::Ok, "No IP address specified".to_string());
    }

    (status_code, response)
}

// #[launch]
pub fn rocket() -> rocket::Rocket<rocket::Build> {
    rocket::build().mount("/", routes![update])
}
