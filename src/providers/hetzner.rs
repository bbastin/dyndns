// SPDX-FileCopyrightText: 2023 Benedikt Bastin
// SPDX-License-Identifier: AGPL-3.0-or-later

#![deny(clippy::all)]

use std::{
    convert::Infallible,
    error::Error,
    fmt::{self},
};

use futures::executor::block_on;
use log::info;
use serde::{Deserialize, Serialize};

use crate::config::{DomainConfig, Zone};

#[derive(Deserialize)]
struct Zones {
    zones: Vec<Zone>,
}

#[derive(PartialEq, Eq, Deserialize, Serialize, strum_macros::Display)]
#[allow(clippy::min_ident_chars)]
pub enum RecordType {
    A,
    AAAA,
    NS,
    MX,
    CNAME,
    RP,
    TXT,
    SOA,
    HINFO,
    SRV,
    DANE,
    TLSA,
    DS,
    CAA,
}

#[derive(Deserialize, Serialize)]
pub struct Record {
    #[serde(rename = "type")]
    pub record_type: RecordType,
    pub id: String,
    // pub created: String,
    // pub modified: String,
    pub zone_id: String,
    pub name: String,
    pub value: String,
    pub ttl: Option<u64>,
}

impl fmt::Display for Record {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} {} {} {}",
            self.name,
            self.ttl.unwrap_or(0),
            self.record_type,
            self.value
        )
    }
}

#[derive(Deserialize)]
struct Records {
    records: Vec<Record>,
}

#[derive(Deserialize)]
struct ResponseRecord {
    record: Record,
}

#[derive(Default)]
pub struct HetznerProvider {
    client: reqwest::Client,
}

impl HetznerProvider {
    #[must_use]
    pub fn new() -> HetznerProvider {
        let p = HetznerProvider {
            client: reqwest::Client::new(),
        };
        info!("Created new Hetzner Provider");

        p
    }

    pub async fn get_zones(&self, apitoken: &str) -> Result<Vec<Zone>, Infallible> {
        let response = self
            .client
            .get("https://dns.hetzner.com/api/v1/zones")
            .header("Auth-API-Token", apitoken)
            .send()
            .await
            .unwrap();

        let zones = response.json::<Zones>().await.unwrap().zones;

        info!("Received {} zones", zones.len());

        Ok(zones)
    }

    // fn get_zone_id(&self, _domainname: &str) -> Result<Zone, i32> {
    //     Err(0)
    // }

    pub async fn get_records(
        &self,
        apitoken: &str,
        zone: &Zone,
    ) -> Result<Vec<Record>, Infallible> {
        let response = self
            .client
            .get("https://dns.hetzner.com/api/v1/records")
            .query(&[("zone_id", zone.id.as_str())])
            .header("Auth-API-Token", apitoken)
            .send()
            .await
            .unwrap();

        let records = response.json::<Records>().await.unwrap().records;

        info!("Received {} records", records.len());

        Ok(records)
    }

    pub async fn update_record(
        &self,
        apitoken: &str,
        record: &Record,
    ) -> Result<Record, Infallible> {
        let response = self
            .client
            .put(format!(
                "https://dns.hetzner.com/api/v1/records/{}",
                record.id
            ))
            .header("Auth-API-Token", apitoken)
            .json(record)
            .send()
            .await
            .unwrap();

        let new_record = response.json::<ResponseRecord>().await.unwrap().record;

        info!("Successfully updated record {}", new_record.id);

        Ok(new_record)
    }
}

impl super::Provider for HetznerProvider {
    fn update_ip(
        &self,
        domain_config: &DomainConfig,
        new_ip: std::net::IpAddr,
    ) -> Result<bool, Box<dyn Error>> {
        // Split domain into subdomain and zone (if applicable)
        let update_record_name = if let Some(subdomain) = domain_config
            .host
            .strip_suffix(domain_config.zone.name.as_str())
        {
            // Strip last remaining dot from subdomain
            subdomain.strip_suffix('.').unwrap_or(subdomain)
        } else {
            // Use the whole domain, which is denoted by @ in DNS
            "@"
        };

        // Determine type of record to update (A for IPv4 or AAAA for IPv6)
        let update_record_type = if new_ip.is_ipv4() {
            RecordType::A
        } else {
            RecordType::AAAA
        };

        info!(
            "Updating \"{}\" record of type {} in zone {} (ID: {})",
            domain_config.host, update_record_type, domain_config.zone.name, domain_config.zone.id
        );

        tokio::task::block_in_place(|| {
            block_on(async move {
                // Get all records of specified zone
                let records = self
                    .get_records(&domain_config.apitoken, &domain_config.zone)
                    .await
                    .unwrap();

                // Find the record with matching type and name
                let record = records
                    .into_iter()
                    .find(|r| r.name == update_record_name && r.record_type == update_record_type)
                    .unwrap_or_else(|| {
                        panic!(
                            "No matching record (name: {update_record_name}, type: {update_record_type})"
                        )
                    });

                // If the value is already correct, skip the update
                if record.value == new_ip.to_string() {
                    info!(
                        "Record \"{update_record_name}\" of type {update_record_type} in zone {} (ID: {}) does not need to be updated",
                        domain_config.zone.name,
                        domain_config.zone.id
                    );
                    return Ok(false);
                }

                // Create the updated record
                let new_record = Record {
                    value: new_ip.to_string(),
                    ..record
                };

                // Update the record
                let _ = self
                    .update_record(&domain_config.apitoken, &new_record)
                    .await;

                Ok(true)
            })
        })
    }
}
