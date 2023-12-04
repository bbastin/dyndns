// SPDX-FileCopyrightText: Copyright (C) 2023 Benedikt Bastin
// SPDX-License-Identifier: AGPL-3.0-or-later

#![deny(clippy::all)]

use std::{error::Error, fmt::Display, net::IpAddr};

use crate::config::Zone;

pub mod hetzner;

#[derive(Debug)]
pub struct RecordNotFoundError {}

impl Error for RecordNotFoundError {}

impl Display for RecordNotFoundError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Record not found")
    }
}

pub trait Provider {
    fn update_ip(&self, domain: String, zone: Zone, new_ip: IpAddr)
        -> Result<bool, Box<dyn Error>>;
}
