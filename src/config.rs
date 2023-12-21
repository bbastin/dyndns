// SPDX-FileCopyrightText: 2023 Benedikt Bastin
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use serde::Deserialize;

use crate::providers::{hetzner::HetznerProvider, MockProvider};

#[derive(PartialEq, Eq, Deserialize, strum_macros::Display, Clone, Copy)]
pub enum ProviderType {
    HetznerProvider,
    MockProvider,
}

#[derive(Deserialize, Clone)]
pub struct Zone {
    pub id: String,
    pub name: String,
}

#[derive(Deserialize, Clone)]
pub struct DomainConfig {
    pub provider: ProviderType,
    pub apitoken: String,
    pub host: String,
    pub zone: Zone,
}

#[derive(Deserialize, Clone)]
pub struct User {
    pub name: String,
    pub password: String,
    pub domains: Vec<DomainConfig>,
}

#[derive(Deserialize, Clone)]
pub struct Config {
    pub users: Vec<User>,
}

pub struct Providers {
    pub hetzner_provider: Option<HetznerProvider>,
    pub mock_provider: Option<MockProvider>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_user() {
        let data = r#"{
            "name": "testtest",
            "password": "testpassword",
            "domains": [{
                "provider": "HetznerProvider",
                "apitoken": "testapitoken",
                "host": "test.example.com",
                "zone": {
                    "name": "example.com",
                    "id": "idexamplecom"
                }
            }]
        }"#;

        let u: User = serde_json::from_str(data).unwrap();

        assert_eq!(u.name, "testtest");
        assert_eq!(u.password, "testpassword");
    }

    // #[test]
    // fn parse_config() {
    //     let data = r#"[{
    //         "name": "testtest",
    //         "password": "testpassword",
    //         "domains": [{
    //             "provider": "HetznerProvider",
    //             "apitoken": "testapitoken",
    //             "host": "test.example.com",
    //             "zone": {
    //                 "name": "example.com",
    //                 "id": "idexamplecom"
    //             }
    //         }]
    //     }]"#;

    //     let u: Config = serde_json::from_str(data).unwrap();

    //     assert_eq!(u.users.len(), 1);

    //     assert_eq!(u.users[0].name, "testtest");
    //     assert_eq!(u.users[0].password, "testpassword");
    // }
}
