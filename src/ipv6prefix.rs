// SPDX-FileCopyrightText: 2026 Benedikt Bastin
//
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::error::Error;
use std::net::Ipv6Addr;
use std::str::FromStr;

#[derive(Debug, PartialEq)]
pub struct Ipv6Prefix {
    pub prefix: Ipv6Addr,
    pub length: u8,
}

impl Ipv6Prefix {
    pub fn from_str(s: &str) -> Result<Self, Box<dyn Error>> {
        let mut split = s.split("/");

        let prefix = Ipv6Addr::from_str(split.next().expect("prefix error"))?;
        let length = u8::from_str(split.next().expect("prefix error"))?;

        Ok(Self { prefix, length })
    }

    pub fn combine(&self, addr: Ipv6Addr) -> Ipv6Addr {
        let mut p = self.prefix.octets();
        let m = (u128::MAX << (128 - self.length)).to_be_bytes();
        let a = addr.octets();

        for i in 0..p.len() {
            p[i] = (p[i] & m[i]) | (a[i] & !m[i]);
        }

        Ipv6Addr::from_octets(p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_str() {
        assert_eq!(
            Ipv6Prefix::from_str("::1/128").unwrap(),
            Ipv6Prefix {
                prefix: Ipv6Addr::from_bits(1),
                length: 128
            }
        );
        assert_eq!(
            Ipv6Prefix::from_str("::1/64").unwrap(),
            Ipv6Prefix {
                prefix: Ipv6Addr::from_bits(1),
                length: 64
            }
        );
        assert_eq!(
            Ipv6Prefix::from_str("::1/0").unwrap(),
            Ipv6Prefix {
                prefix: Ipv6Addr::from_bits(1),
                length: 0
            }
        );
        assert_eq!(
            Ipv6Prefix::from_str("2001:db8::/32").unwrap(),
            Ipv6Prefix {
                prefix: Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 0),
                length: 32
            }
        );
    }

    #[test]
    fn combine() {
        let test_prefix = Ipv6Prefix::from_str("2001:db8::/32").unwrap();

        assert_eq!(
            test_prefix.combine(Ipv6Addr::from_bits(1)),
            Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 1)
        );
        assert_eq!(
            test_prefix.combine(Ipv6Addr::new(0x1, 0x1, 0, 0, 0, 0, 0, 0)),
            Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 0)
        );
        assert_eq!(
            test_prefix.combine(Ipv6Addr::new(0x1, 0x10, 0, 0, 0, 0, 0, 0)),
            Ipv6Addr::new(0x2001, 0xdb8, 0, 0, 0, 0, 0, 0)
        );
    }
}
