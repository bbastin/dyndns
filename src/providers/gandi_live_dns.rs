// SPDX-FileCopyrightText: 2026 Benedikt Bastin
// SPDX-License-Identifier: AGPL-3.0-or-later

#![deny(clippy::all)]

use log::{debug, info};
use secrecy::{ExposeSecret, SecretString};
use std::error::Error;
use std::net::{Ipv4Addr, Ipv6Addr};

// Provider to update DNS records using Gandi's live DNS API
//
// This provider replaces records by zone_id, name and type 
//
// [`Gandi Live DNS api documentation`]: https://api.gandi.net/docs/livedns/#patch-v5-livedns-domains-fqdn-records-rrset_name-rrset_type
pub struct GandiLiveDnsProvider {}

impl GandiLiveDnsProvider {
    pub async fn update_ipv4(
        addr: Ipv4Addr,
        zone_id: &str,
        apitoken: &SecretString,
        name: &str,
        datestr: &str,
    ) -> Result<(), Box<dyn Error>> {
        Self::update_rrset("A", &addr.to_string(), zone_id, apitoken, name, datestr).await
    }

    pub async fn update_ipv6(
        addr: Ipv6Addr,
        zone_id: &str,
        apitoken: &SecretString,
        name: &str,
        datestr: &str,
    ) -> Result<(), Box<dyn Error>> {
        Self::update_rrset("AAAA", &addr.to_string(), zone_id, apitoken, name, datestr)
            .await
    }

    async fn update_rrset(
        record_type: &str,
        value: &str,
        zone_id: &str,
        apitoken: &SecretString,
        name: &str,
        datestr: &str,
    ) -> Result<(), Box<dyn Error>> {
        let client = reqwest::Client::new();

        let (_, body, url) = Self::build_query(record_type, value, zone_id, name, datestr);

        debug!("Request {url} => {value} (body: \"{body}\")");

        let res = client
            .put(url)
            .header("Content-Type", "application/json")
            .header(
                "Authorization",
                format!("Bearer {}", apitoken.expose_secret()),
            )
            .body(body)
            .send()
            .await?;

        debug!("Provider response {res:?}");

        if res.status() == 201 {
            info!("Record {record_type} for {name} in zone {zone_id} has been updated to {value}");
            return Ok(());
        }

        let body = res.text().await?;

        info!( "Failed to update record {record_type} for {name} in zone {zone_id} to {value}: {body}");
        
        Err(body.into())
    }

    fn build_query(record_type: &str, value: &str, zone_id: &str, name: &str, datestr: &str) -> (String, String, String) {
        let comment = format!("dyndns updated {datestr}");

        let body = format!(r#"{{"rrset_values":["{value}"]}}"#);

        let url = format!("https://api.gandi.net/v5/livedns/domains/{zone_id}/records/{name}/{record_type}");
        (comment, body, url)
    }
}



#[cfg(test)]
mod tests{
    use super::*;

    #[test]
    fn test_build_query_rrset_a(){
        let (comment, body, url) = GandiLiveDnsProvider::build_query("A", "10.0.0.1", "example.com", "example", "2026-10-31" );
        assert_eq!(
            "dyndns updated 2026-10-31",
            comment
        );

        assert_eq!(
            "https://api.gandi.net/v5/livedns/domains/example.com/records/example/A",
            url
        );
        
        assert_eq!(
            r#"{"rrset_values":["10.0.0.1"]}"#,
            body
        );
    }

}
