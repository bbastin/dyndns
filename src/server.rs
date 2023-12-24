// SPDX-FileCopyrightText: 2023 Benedikt Bastin
//
// SPDX-License-Identifier: AGPL-3.0-or-later

#![deny(clippy::all)]

use std::{
    fmt::Write,
    net::{Ipv4Addr, Ipv6Addr},
    str::FromStr,
};

use log::{error, info, warn};
use rocket::{get, http::Status, routes, State};

use crate::{
    config::{self, Config, DomainConfig, Providers, User},
    providers::{update_ipv4, update_ipv6, Provider},
};

fn get_user<'user_config_lifetime>(
    config: &'user_config_lifetime Config,
    username: &str,
    password: &str,
) -> Result<&'user_config_lifetime User, ()> {
    let user_config = config.users.iter().find(|u| u.name == username);

    if user_config.is_none() {
        warn!("Invalid user {}", username);
        return Err(());
    }
    let user = user_config.unwrap();

    if user.password != password {
        warn!("Wrong password for user {}", user.name);
        return Err(());
    }

    Ok(user)
}

fn get_domain_config<'user_config_lifetime>(
    user: &'user_config_lifetime User,
    host: &str,
) -> Result<&'user_config_lifetime DomainConfig, ()> {
    let domain_config = user.domains.iter().find(|d| d.host == host);

    if domain_config.is_none() {
        warn!("Invalid domain {host} for user {}", user.name);
        return Err(());
    }
    Ok(domain_config.unwrap())
}

#[get("/update?<user>&<password>&<host>&<ip>&<ip6>")]
fn update(
    user: &str,
    password: &str,
    host: &str,
    ip: Option<&str>,
    ip6: Option<&str>,
    config: &State<Config>,
    providers: &State<Providers>,
) -> (Status, String) {
    let user = get_user(config, user, password);
    if user.is_err() {
        return (Status::Unauthorized, "Invalid user".to_string());
    }
    let user = user.unwrap();

    let domain_config = get_domain_config(user, host);
    if domain_config.is_err() {
        return (Status::BadRequest, "Invalid domain".to_string());
    }
    let domain_config = domain_config.unwrap();

    let p: &dyn Provider = match domain_config.provider {
        config::ProviderType::HetznerProvider => providers.hetzner_provider.as_ref().unwrap(),
        config::ProviderType::MockProvider => providers.mock_provider.as_ref().unwrap(),
    };

    info!(
        "Received IP addresses: IPv4 {}, IPv6: {}",
        ip.unwrap_or("<empty>"),
        ip6.unwrap_or("<empty>")
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

    let mut status_code = Status::Ok;
    let mut response: String = String::default();

    if parsed_ipv4.is_some() {
        let res = update_ipv4(p, &parsed_ipv4.unwrap(), domain_config);

        match res {
            Ok(s) => {
                let _ = writeln!(response, "{s}");
                info!("{s}");
            }
            Err(e) => {
                let _ = writeln!(response, "Error updating IPv4 address: {e}");
                status_code = Status::InternalServerError;
                error!("{e}");
            }
        }
    }

    if ip6.is_some_and(|s| !s.is_empty()) {
        let res = update_ipv6(p, &parsed_ipv6.unwrap(), domain_config);

        match res {
            Ok(s) => {
                let _ = writeln!(response, "{s}");
                info!("{s}");
            }
            Err(e) => {
                let _ = writeln!(response, "Error updating IPv6 address: {e}");
                status_code = Status::InternalServerError;
                error!("{e}");
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

#[cfg(test)]
mod tests {
    use crate::{
        config::{DomainConfig, User},
        providers::MockProvider,
    };

    use super::*;
    use rocket::local::blocking::Client;

    fn construct(mock_provider: Option<MockProvider>) -> Client {
        Client::tracked(
            rocket()
                .manage(Config {
                    users: vec![User {
                        name: "test".to_string(),
                        password: "testpassword".to_string(),
                        domains: vec![DomainConfig {
                            provider: crate::config::ProviderType::MockProvider,
                            apitoken: "testtoken".to_string(),
                            host: "example.com".to_string(),
                            zone: crate::config::Zone {
                                id: "testzoneid".to_string(),
                                name: "testzone".to_string(),
                            },
                        }],
                    }],
                })
                .manage(Providers {
                    hetzner_provider: None,
                    mock_provider: mock_provider,
                }),
        )
        .expect("valid rocket instance")
    }

    mod uri_checks {
        use super::*;

        #[test]
        fn test() {
            let client = construct(Some(MockProvider::default()));
            let response = client.get("/").dispatch();
            assert_eq!(response.status(), Status::NotFound);
        }

        #[test]
        fn missing_auth_query_params() {
            let client = construct(Some(MockProvider::default()));
            let response = client.get("/update").dispatch();
            assert_eq!(response.status(), Status::UnprocessableEntity);
        }

        #[test]
        fn unauthorized() {
            let client = construct(Some(MockProvider::default()));
            let response = client
                .get("/update?user=wronguser&password=wrongpassword&host=example.com")
                .dispatch();
            assert_eq!(response.status(), Status::Unauthorized);
        }

        #[test]
        fn empty_update() {
            let client = construct(Some(MockProvider::default()));
            let response = client
                .get("/update?user=test&password=testpassword&host=example.com")
                .dispatch();
            assert_eq!(response.status(), Status::Ok);
        }
    }

    mod mock_updates {
        use super::*;

        #[test]
        fn update_ipv4_only() {
            let mut mock = MockProvider::default();
            mock.expect_update_ip().once().returning(|_, _| Ok(true));

            let client = construct(Some(mock));
            let response = client
                .get("/update?user=test&password=testpassword&host=example.com&ip=192.0.2.0")
                .dispatch();
            assert_eq!(response.status(), Status::Ok);
            assert_eq!(
                response.into_string().unwrap(),
                "Updated IPv4 successfully\n"
            );
        }

        #[test]
        fn update_ipv6_only() {
            let mut mock = MockProvider::default();
            mock.expect_update_ip().once().returning(|_, _| Ok(true));

            let client = construct(Some(mock));
            let response = client
                .get("/update?user=test&password=testpassword&host=example.com&ip6=2001:0db8:85a3:0000:0000:8a2e:0370:7334")
                .dispatch();
            assert_eq!(response.status(), Status::Ok);
            assert_eq!(
                response.into_string().unwrap(),
                "Updated IPv6 successfully\n"
            );
        }

        #[test]
        fn update_ipv4_ipv6() {
            let mut mock = MockProvider::default();
            mock.expect_update_ip().times(2).returning(|_, _| Ok(true));

            let client = construct(Some(mock));
            let response = client
                .get("/update?user=test&password=testpassword&host=example.com&ip=192.0.2.0&ip6=2001:0db8:85a3:0000:0000:8a2e:0370:7334")
                .dispatch();
            assert_eq!(response.status(), Status::Ok);
            assert_eq!(
                response.into_string().unwrap(),
                "Updated IPv4 successfully\nUpdated IPv6 successfully\n"
            );
        }

        #[test]
        fn update_ipv4_only_twice() {
            let mut mock = MockProvider::default();
            mock.expect_update_ip().once().returning(|_, _| Ok(true));
            // The second time, the IP address will already be set correctly
            mock.expect_update_ip().once().returning(|_, _| Ok(false));

            let client = construct(Some(mock));
            let response = client
                .get("/update?user=test&password=testpassword&host=example.com&ip=192.0.2.0")
                .dispatch();
            assert_eq!(response.status(), Status::Ok);
            assert_eq!(
                response.into_string().unwrap(),
                "Updated IPv4 successfully\n"
            );

            let response = client
                .get("/update?user=test&password=testpassword&host=example.com&ip=192.0.2.0")
                .dispatch();
            assert_eq!(response.status(), Status::Ok);
            assert_eq!(
                response.into_string().unwrap(),
                "IPv4 already set correctly\n"
            );
        }
    }
}
