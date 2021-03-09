
use std::convert::{From, TryFrom, TryInto};
use std::net::{Ipv4Addr, Ipv6Addr};
use pnet::packet::ipv6;

type Error = Box<dyn std::error::Error>;
type Result<T> = std::result::Result<T, Error>;

pub trait Teredo {
    fn is_teredo(&self) -> bool;
}

impl Teredo for [u8; 16] {
    fn is_teredo(&self) -> bool {
        self[0] == 0x20 &&
        self[1] == 0x01 &&
        self[2] == 0x00 &&
        self[3] == 0x00
    }
}

impl Teredo for Ipv6Addr {
    fn is_teredo(&self) -> bool {
        self.octets().is_teredo()
    }
}

impl Teredo for ipv6::Ipv6 {
    fn is_teredo(&self) -> bool {
        self.version == 6 &&
        self.source.is_teredo() &&
        self.destination.is_teredo()
    }
}

impl<'a> Teredo for ipv6::Ipv6Packet<'a> {
    fn is_teredo(&self) -> bool {
        self.get_version() == 6 &&
        self.get_source().is_teredo() &&
        self.get_destination().is_teredo()
    }
}

/// RFC 4380
/// Represents a Teredo endpoint.
///
/// The Teredo addresses are composed of 5 components:
///
/// +-------------+-------------+-------+------+-------------+
/// | Prefix      | Server IPv4 | Flags | Port | Client IPv4 |
/// +-------------+-------------+-------+------+-------------+
///
/// - Prefix: the 32-bit Teredo service prefix.
/// - Server IPv4: the IPv4 address of a Teredo server.
/// - Flags: a set of 16 bits that document type of address and NAT.
/// - Port: the obfuscated "mapped UDP port" of the Teredo service at
///   the client.
/// - Client IPv4: the obfuscated "mapped IPv4 address" of the client.
///
/// In this format, both the "mapped UDP port" and "mapped IPv4 address"
/// of the client are obfuscated.  Each bit in the address and port
/// number is reversed; this can be done by an exclusive OR of the 16-bit
/// port number with the hexadecimal value 0xFFFF, and an exclusive OR of
/// the 32-bit address with the hexadecimal value 0xFFFFFFFF.
///
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct TeredoEndpoint {
    pub prefix: u32,
    pub teredo_server_ipv4: Ipv4Addr,
    pub teredo_client_ipv4: Ipv4Addr,
    pub flags: u16,
    pub udp_port: u16,
}

impl TryFrom<[u8; 16]> for TeredoEndpoint
{
    type Error = Error;

    fn try_from(value: [u8; 16]) -> Result<Self> {
        if !value.is_teredo() {
            Err("Not a teredo address")?
        }

        Ok(TeredoEndpoint {
            prefix: u32::from_be_bytes(value[0..4].try_into().unwrap()),
            teredo_server_ipv4: u32::from_be_bytes(value[4..8].try_into().unwrap()).into(),
            flags: u16::from_be_bytes(value[8..10].try_into().unwrap()),
            udp_port: u16::from_be_bytes(value[10..12].try_into().unwrap()) ^ 0xFFFF,
            teredo_client_ipv4: (u32::from_be_bytes(value[12..16].try_into().unwrap()) ^ 0xFFFF_FFFF).try_into().unwrap()
        })
    }
}

impl TryFrom<Ipv6Addr> for TeredoEndpoint
{
    type Error = Error;

    fn try_from(value: Ipv6Addr) -> Result<Self> {
        if !value.octets().is_teredo() {
            Err("Not a teredo address")?
        }

        Ok(value.octets().try_into()?)
    }
}

pub trait TeredoHeader {
    fn get_teredo_endpoints(&self) -> Result<(TeredoEndpoint, TeredoEndpoint)>;
}

impl TeredoHeader for ipv6::Ipv6 {
    fn get_teredo_endpoints(&self) -> Result<(TeredoEndpoint, TeredoEndpoint)> {
        if !self.is_teredo() {
            Err("Not a teredo packet")?
        }

        Ok(
            (self.source.try_into()?,
            self.destination.try_into()?)
        )
    }
}

impl<'a> TeredoHeader for ipv6::Ipv6Packet<'a> {
    fn get_teredo_endpoints(&self) -> Result<(TeredoEndpoint, TeredoEndpoint)> {
        if !self.is_teredo() {
            Err("Not a teredo packet")?
        }

        Ok(
            (self.get_source().try_into()?,
            self.get_destination().try_into()?)
        )
    }
}

#[cfg(test)]
mod test{
    use std::str::FromStr;
    use super::{TeredoEndpoint, Teredo, Ipv6Addr, Ipv4Addr, TryInto};

    #[test]
    fn is_teredo_address() {
        let ipv6 = Ipv6Addr::from_str("2001:0:338c:24f4:43b:30e3:d2f3:c93d").unwrap();
        let ipv6_not_teredo = Ipv6Addr::from_str("2019:0:338c:24f4:43b:30e3:d2f3:c93d").unwrap();

        assert_eq!(ipv6.is_teredo(), true);
        assert_eq!(ipv6_not_teredo.is_teredo(), false);
    }

    #[test]
    fn from_ipv6_endpoint() {
        let ipv6 = Ipv6Addr::from_str("2001:0:338c:24f4:43b:30e3:d2f3:c93d").unwrap();
        let ep_teredo: TeredoEndpoint = ipv6.try_into().unwrap();

        assert_eq!(ep_teredo.prefix, 0x20010000);
        assert_eq!(ep_teredo.teredo_client_ipv4, Ipv4Addr::from_str("45.12.54.194").unwrap());
        assert_eq!(ep_teredo.teredo_server_ipv4, Ipv4Addr::from_str("51.140.36.244").unwrap());
        assert_eq!(ep_teredo.udp_port, 53020);
    }
}