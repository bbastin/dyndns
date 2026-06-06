// SPDX-FileCopyrightText: 2023 Benedikt Bastin
//
// SPDX-License-Identifier: AGPL-3.0-or-later

#![deny(clippy::all)]

use std::{
    net::{Ipv4Addr, Ipv6Addr},
    str::FromStr,
};

use chrono::{DateTime, Local};
use log::{info, warn};
use rocket::{get, http::Status, routes, State};

use crate::config::{Config, Provider, Record};
use crate::ipv6prefix::Ipv6Prefix;

use crate::providers::hetzner_cloud::HetznerCloudProvider;

#[get("/update?<user>&<password>&<host>&<ip>&<ip6>&<ip6lanprefix>")]
async fn update(
    user: &str,
    password: &str,
    host: &str,
    ip: Option<&str>,
    ip6: Option<&str>,
    ip6lanprefix: Option<&str>,
    config: &State<Config>,
    // providers: &State<Providers>,
) -> (Status, String) {
    let user = match config.get_user(user, password) {
        Err(msg) => {
            warn!("get_user: {msg}");
            return (Status::Unauthorized, "Invalid user".to_string());
        }
        Ok(user) => user,
    };

    let zone = match user.get_zone(host) {
        Err(msg) => {
            warn!("get_zone: {msg}");
            return (Status::BadRequest, "Invalid domain".to_string());
        }
        Ok(zone) => zone,
    };

    info!("Zone {host}: {:?}", zone);

    info!(
        "Received IP addresses: IPv4 {}, IPv6: {}, IPv6 Prefix: {}",
        ip.unwrap_or("<empty>"),
        ip6.unwrap_or("<empty>"),
        ip6lanprefix.unwrap_or("<empty>"),
    );

    let parsed_ipv4 = if ip.is_some_and(|s| !s.is_empty()) {
        match Ipv4Addr::from_str(ip.unwrap()) {
            Ok(i) => Some(i),
            Err(_) => return (Status::BadRequest, "Invalid IPv4 address".to_string()),
        }
    } else {
        None
    };

    let parsed_ipv6 = if ip6.is_some_and(|s| !s.is_empty()) {
        match Ipv6Addr::from_str(ip6.unwrap()) {
            Ok(i) => Some(i),
            Err(_) => return (Status::BadRequest, "Invalid IPv6 address".to_string()),
        }
    } else {
        None
    };

    let parsed_ipv6lanprefix = if ip6lanprefix.is_some_and(|s| !s.is_empty()) {
        match Ipv6Prefix::from_str(ip6lanprefix.unwrap()) {
            Ok(i) => Some(i),
            Err(_) => return (Status::BadRequest, "Invalid IPv6 prefix".to_string()),
        }
    } else {
        None
    };

    let date: DateTime<Local> = Local::now();
    let datestr = date.to_rfc3339();

    for record in &zone.records {
        match record {
            Record::Aaaa { name, suffix } => {
                let new_address = match suffix {
                    Some(s) => parsed_ipv6lanprefix.as_ref().map(|p| p.combine(*s)),
                    None => parsed_ipv6,
                };

                if let Some(addr) = new_address {
                    match &zone.provider {
                        Provider::HetznerCloud { apitoken } => {
                            let res = HetznerCloudProvider::update_ipv6(
                                addr, &zone.id, apitoken, name, &datestr,
                            )
                            .await;

                            if res.is_err() {
                                return (Status::BadRequest, "Error".to_string());
                            }
                        }
                    };
                };
            }
            Record::A { name } => {
                if let Some(addr) = parsed_ipv4 {
                    match &zone.provider {
                        Provider::HetznerCloud { apitoken } => {
                            let res = HetznerCloudProvider::update_ipv4(
                                addr, &zone.id, apitoken, name, &datestr,
                            )
                            .await;

                            if res.is_err() {
                                return (Status::BadRequest, "Error".to_string());
                            }
                        }
                    };
                }
            }
        }
    }

    (Status::Ok, "Ok".to_string())
}

// #[launch]
pub fn rocket() -> rocket::Rocket<rocket::Build> {
    rocket::build().mount("/", routes![update])
}
