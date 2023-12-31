// SPDX-FileCopyrightText: 2023 Benedikt Bastin
//
// SPDX-License-Identifier: AGPL-3.0-or-later

#![deny(clippy::all)]

use std::{
    error::Error,
    fmt::Display,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
};

use mockall::automock;

use crate::config::DomainConfig;

pub mod hetzner;

#[derive(Debug)]
pub struct RecordNotFoundError;

impl Error for RecordNotFoundError {}

impl Display for RecordNotFoundError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Record not found")
    }
}

#[automock]
pub trait Provider {
    fn update_ip(
        &self,
        domain_config: &DomainConfig,
        new_ip: IpAddr,
    ) -> Result<bool, Box<dyn Error>>;
}

pub fn update_ipv4(
    provider: &dyn Provider,
    new_ip: &Ipv4Addr,
    domain_config: &DomainConfig,
) -> Result<String, String> {
    let updated = provider.update_ip(domain_config, IpAddr::V4(*new_ip));

    match updated {
        Ok(update_status) => {
            if update_status {
                Ok("Updated IPv4 successfully".to_string())
            } else {
                Ok("IPv4 already set correctly".to_string())
            }
        }
        Err(error) => Err(format!("Error: {error}")),
    }
}

pub fn update_ipv6(
    provider: &dyn Provider,
    new_ip: &Ipv6Addr,
    domain_config: &DomainConfig,
) -> Result<String, String> {
    let updated = provider.update_ip(domain_config, IpAddr::V6(*new_ip));

    match updated {
        Ok(update_status) => {
            if update_status {
                Ok("Updated IPv6 successfully".to_string())
            } else {
                Ok("IPv6 already set correctly".to_string())
            }
        }
        Err(error) => Err(format!("Error: {error}")),
    }
}
