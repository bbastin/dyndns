// SPDX-FileCopyrightText: 2026 Benedikt Bastin
// SPDX-License-Identifier: AGPL-3.0-or-later

#![deny(clippy::all)]

use log::{debug, info};
use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use std::error::Error;
use std::net::{Ipv4Addr, Ipv6Addr};

pub struct HetznerCloudProvider {}

#[derive(Deserialize)]
struct HetznerApiErrorResponse {
    _code: String,
    message: String,
    _details: Option<String>,
}

impl HetznerCloudProvider {
    pub async fn update_ipv4(
        addr: Ipv4Addr,
        zone_id: &str,
        apitoken: &SecretString,
        name: &str,
        datestr: &str,
    ) -> Result<(), Box<dyn Error>> {
        return Self::update_rrset("A", &addr.to_string(), zone_id, apitoken, name, datestr).await;
    }

    pub async fn update_ipv6(
        addr: Ipv6Addr,
        zone_id: &str,
        apitoken: &SecretString,
        name: &str,
        datestr: &str,
    ) -> Result<(), Box<dyn Error>> {
        return Self::update_rrset("AAAA", &addr.to_string(), zone_id, apitoken, name, datestr)
            .await;
    }

    pub async fn update_rrset(
        record_type: &str,
        value: &str,
        zone_id: &str,
        apitoken: &SecretString,
        name: &str,
        datestr: &str,
    ) -> Result<(), Box<dyn Error>> {
        let client = reqwest::Client::new();

        let comment = format!("dyndns updated {datestr}");

        let body = format!(r#"{{"records": [{{"value": "{value}","comment": "{comment}"}}]}}"#);

        let url = format!("https://api.hetzner.cloud/v1/zones/{zone_id}/rrsets/{name}/{record_type}/actions/set_records");

        debug!("Request {url} => {value} (label: \"{comment}\")");

        let res = client
            .post(url)
            .header("Content-Type", "application/json")
            .header(
                "Authorization",
                format!("Bearer {}", apitoken.expose_secret()),
            )
            .body(body)
            .send()
            .await?;

        debug!("Response {res:?}");

        if res.status() == 201 {
            info!("Request successfull");
            return Ok(());
        }

        let body = res.text().await?;
        let error: HetznerApiErrorResponse = serde_json::from_str(&body).unwrap();

        Err(error.message.into())
    }
}
