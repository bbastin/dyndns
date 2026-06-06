// SPDX-FileCopyrightText: 2023 Benedikt Bastin
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use std::net::Ipv6Addr;

#[derive(Debug, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum Provider {
    HetznerCloud { apitoken: SecretString },
}

#[derive(Debug, PartialEq, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum Record {
    A {
        name: String,
    },
    #[serde(rename = "AAAA")]
    Aaaa {
        name: String,
        suffix: Option<Ipv6Addr>,
    },
}

#[derive(Debug, Deserialize, Clone)]
pub struct Zone {
    pub host: String,
    pub id: String,
    pub provider: Provider,
    pub records: Vec<Record>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct User {
    pub name: String,
    pub password: SecretString,
    pub zones: Vec<Zone>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(transparent)]
pub struct Config {
    pub users: Vec<User>,
}

impl Config {
    pub fn get_user(&self, username: &str, password: &str) -> Result<&User, String> {
        let user_config = self.users.iter().find(|u| u.name == username);

        if user_config.is_none() {
            return Err(format!("Invalid user {}", username));
        }

        let user = user_config.unwrap();

        if user.password.expose_secret() != password {
            return Err(format!("Wrong password for user {}", user.name));
        }

        Ok(user)
    }
}

impl User {
    pub fn get_zone(&self, host: &str) -> Result<&Zone, String> {
        let zone = self.zones.iter().find(|z| z.host == host);

        match zone {
            None => Err(format!("No zone for host {host} and user {}", self.name)),
            Some(zone) => Ok(zone),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    const DATA: &str = r#"[
    {
        "name": "testuser",
        "password": "testpassword",
        "zones": [
            {
                "host": "sub.example.com",
                "id": "example.com",
                "provider": {
                    "type": "HetznerCloud",
                    "apitoken": "SECRETAPITOKEN"
                },
                "records": [
                    {
                        "type": "AAAA",
                        "name": "sub.example.com",
                        "suffix": "::a:b:c:d:e:f"
                    }
                ]
            }
        ]
    }
]"#;

    #[test]
    fn parse_config() {
        let c: Config = serde_json::from_str(DATA).unwrap();

        let u0 = &c.users[0];

        assert_eq!(u0.name, "testuser");
        assert_eq!(u0.password.expose_secret(), "testpassword");
        assert_eq!(u0.zones.len(), 1);

        let u0z0 = &u0.zones[0];

        assert_eq!(u0z0.id, "example.com");

        match &u0z0.provider {
            Provider::HetznerCloud { apitoken } => {
                assert_eq!(apitoken.expose_secret(), "SECRETAPITOKEN".to_string());
            }
        }

        assert_eq!(u0z0.records.len(), 1);

        let u0z0r0 = &u0z0.records[0];

        assert_eq!(
            *u0z0r0,
            Record::Aaaa {
                name: "sub.example.com".to_string(),
                suffix: Some(Ipv6Addr::from_str("::a:b:c:d:e:f").unwrap())
            }
        );
    }

    #[test]
    fn get_user() {
        let c: Config = serde_json::from_str(DATA).unwrap();

        assert!(c.get_user("wronguser", "wrongpass").is_err());
        assert!(c.get_user("testuser", "wrongpass").is_err());
        assert!(c.get_user("testuser", "testpassword").is_ok());
    }
}
