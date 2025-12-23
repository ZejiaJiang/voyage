//! Packet Parsing Module
//!
//! This module provides IP packet parsing functionality for both IPv4 and IPv6,
//! as well as TCP and UDP header parsing.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

use crate::error::VoyageError;
use crate::nat::NatKey;

/// Minimum IPv4 header length
pub const IPV4_MIN_HEADER_LEN: usize = 20;
/// Minimum IPv6 header length
pub const IPV6_HEADER_LEN: usize = 40;
/// TCP header minimum length
pub const TCP_MIN_HEADER_LEN: usize = 20;
/// UDP header length
pub const UDP_HEADER_LEN: usize = 8;

/// Protocol numbers
pub const PROTO_TCP: u8 = 6;
pub const PROTO_UDP: u8 = 17;
pub const PROTO_ICMP: u8 = 1;
pub const PROTO_ICMPV6: u8 = 58;

/// IP version
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpVersion {
    V4,
    V6,
}

/// Transport protocol type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportProtocol {
    Tcp,
    Udp,
    Icmp,
    Other(u8),
}

impl TransportProtocol {
    /// Create from protocol number
    pub fn from_proto(proto: u8) -> Self {
        match proto {
            PROTO_TCP => TransportProtocol::Tcp,
            PROTO_UDP => TransportProtocol::Udp,
            PROTO_ICMP | PROTO_ICMPV6 => TransportProtocol::Icmp,
            other => TransportProtocol::Other(other),
        }
    }

    /// Get protocol number
    pub fn to_proto(&self) -> u8 {
        match self {
            TransportProtocol::Tcp => PROTO_TCP,
            TransportProtocol::Udp => PROTO_UDP,
            TransportProtocol::Icmp => PROTO_ICMP,
            TransportProtocol::Other(p) => *p,
        }
    }
}

/// Parsed IP packet header information
#[derive(Debug, Clone)]
pub struct IpPacketInfo {
    /// IP version
    pub version: IpVersion,
    /// Source IP address
    pub src_ip: IpAddr,
    /// Destination IP address
    pub dst_ip: IpAddr,
    /// Transport protocol
    pub protocol: TransportProtocol,
    /// Total packet length
    pub total_len: usize,
    /// IP header length
    pub header_len: usize,
    /// Payload offset in the packet
    pub payload_offset: usize,
}

impl IpPacketInfo {
    /// Parse an IP packet header
    pub fn parse(data: &[u8]) -> Result<Self, VoyageError> {
        if data.is_empty() {
            return Err(VoyageError::InvalidPacket("Empty packet".into()));
        }

        let version = data[0] >> 4;
        match version {
            4 => Self::parse_ipv4(data),
            6 => Self::parse_ipv6(data),
            _ => Err(VoyageError::InvalidPacket(format!(
                "Unknown IP version: {}",
                version
            ))),
        }
    }

    /// Parse IPv4 header
    fn parse_ipv4(data: &[u8]) -> Result<Self, VoyageError> {
        if data.len() < IPV4_MIN_HEADER_LEN {
            return Err(VoyageError::InvalidPacket("IPv4 packet too short".into()));
        }

        let ihl = (data[0] & 0x0F) as usize * 4;
        if ihl < IPV4_MIN_HEADER_LEN || data.len() < ihl {
            return Err(VoyageError::InvalidPacket("Invalid IPv4 IHL".into()));
        }

        let total_len = u16::from_be_bytes([data[2], data[3]]) as usize;
        let protocol = data[9];

        let src_ip = IpAddr::V4(Ipv4Addr::new(data[12], data[13], data[14], data[15]));
        let dst_ip = IpAddr::V4(Ipv4Addr::new(data[16], data[17], data[18], data[19]));

        Ok(Self {
            version: IpVersion::V4,
            src_ip,
            dst_ip,
            protocol: TransportProtocol::from_proto(protocol),
            total_len,
            header_len: ihl,
            payload_offset: ihl,
        })
    }

    /// Parse IPv6 header
    fn parse_ipv6(data: &[u8]) -> Result<Self, VoyageError> {
        if data.len() < IPV6_HEADER_LEN {
            return Err(VoyageError::InvalidPacket("IPv6 packet too short".into()));
        }

        let payload_len = u16::from_be_bytes([data[4], data[5]]) as usize;
        let protocol = data[6]; // Next Header

        let mut src_bytes = [0u8; 16];
        let mut dst_bytes = [0u8; 16];
        src_bytes.copy_from_slice(&data[8..24]);
        dst_bytes.copy_from_slice(&data[24..40]);

        let src_ip = IpAddr::V6(Ipv6Addr::from(src_bytes));
        let dst_ip = IpAddr::V6(Ipv6Addr::from(dst_bytes));

        Ok(Self {
            version: IpVersion::V6,
            src_ip,
            dst_ip,
            protocol: TransportProtocol::from_proto(protocol),
            total_len: IPV6_HEADER_LEN + payload_len,
            header_len: IPV6_HEADER_LEN,
            payload_offset: IPV6_HEADER_LEN,
        })
    }

    /// Get the transport layer payload
    pub fn get_payload<'a>(&self, data: &'a [u8]) -> &'a [u8] {
        if data.len() > self.payload_offset {
            &data[self.payload_offset..]
        } else {
            &[]
        }
    }
}

/// Parsed TCP header information
#[derive(Debug, Clone)]
pub struct TcpPacketInfo {
    /// Source port
    pub src_port: u16,
    /// Destination port
    pub dst_port: u16,
    /// Sequence number
    pub seq_num: u32,
    /// Acknowledgment number
    pub ack_num: u32,
    /// Data offset (header length in 32-bit words)
    pub data_offset: usize,
    /// TCP flags
    pub flags: TcpFlags,
    /// Window size
    pub window: u16,
    /// Checksum
    pub checksum: u16,
    /// Urgent pointer
    pub urgent_ptr: u16,
}

/// TCP flags
#[derive(Debug, Clone, Copy, Default)]
pub struct TcpFlags {
    pub fin: bool,
    pub syn: bool,
    pub rst: bool,
    pub psh: bool,
    pub ack: bool,
    pub urg: bool,
    pub ece: bool,
    pub cwr: bool,
}

impl TcpFlags {
    /// Parse TCP flags from the flags byte
    pub fn from_byte(flags: u8) -> Self {
        Self {
            fin: flags & 0x01 != 0,
            syn: flags & 0x02 != 0,
            rst: flags & 0x04 != 0,
            psh: flags & 0x08 != 0,
            ack: flags & 0x10 != 0,
            urg: flags & 0x20 != 0,
            ece: flags & 0x40 != 0,
            cwr: flags & 0x80 != 0,
        }
    }

    /// Convert to byte
    pub fn to_byte(&self) -> u8 {
        let mut flags = 0u8;
        if self.fin {
            flags |= 0x01;
        }
        if self.syn {
            flags |= 0x02;
        }
        if self.rst {
            flags |= 0x04;
        }
        if self.psh {
            flags |= 0x08;
        }
        if self.ack {
            flags |= 0x10;
        }
        if self.urg {
            flags |= 0x20;
        }
        if self.ece {
            flags |= 0x40;
        }
        if self.cwr {
            flags |= 0x80;
        }
        flags
    }

    /// Check if this is a SYN packet (connection initiation)
    pub fn is_syn(&self) -> bool {
        self.syn && !self.ack
    }

    /// Check if this is a SYN-ACK packet
    pub fn is_syn_ack(&self) -> bool {
        self.syn && self.ack
    }

    /// Check if this is a FIN packet
    pub fn is_fin(&self) -> bool {
        self.fin
    }

    /// Check if this is a RST packet
    pub fn is_rst(&self) -> bool {
        self.rst
    }
}

impl TcpPacketInfo {
    /// Parse TCP header from transport layer data
    pub fn parse(data: &[u8]) -> Result<Self, VoyageError> {
        if data.len() < TCP_MIN_HEADER_LEN {
            return Err(VoyageError::InvalidPacket("TCP header too short".into()));
        }

        let src_port = u16::from_be_bytes([data[0], data[1]]);
        let dst_port = u16::from_be_bytes([data[2], data[3]]);
        let seq_num = u32::from_be_bytes([data[4], data[5], data[6], data[7]]);
        let ack_num = u32::from_be_bytes([data[8], data[9], data[10], data[11]]);
        let data_offset = ((data[12] >> 4) as usize) * 4;
        let flags = TcpFlags::from_byte(data[13]);
        let window = u16::from_be_bytes([data[14], data[15]]);
        let checksum = u16::from_be_bytes([data[16], data[17]]);
        let urgent_ptr = u16::from_be_bytes([data[18], data[19]]);

        if data_offset < TCP_MIN_HEADER_LEN || data.len() < data_offset {
            return Err(VoyageError::InvalidPacket("Invalid TCP data offset".into()));
        }

        Ok(Self {
            src_port,
            dst_port,
            seq_num,
            ack_num,
            data_offset,
            flags,
            window,
            checksum,
            urgent_ptr,
        })
    }

    /// Get TCP payload
    pub fn get_payload<'a>(&self, data: &'a [u8]) -> &'a [u8] {
        if data.len() > self.data_offset {
            &data[self.data_offset..]
        } else {
            &[]
        }
    }

    /// Get payload length
    pub fn payload_len(&self, transport_data_len: usize) -> usize {
        if transport_data_len > self.data_offset {
            transport_data_len - self.data_offset
        } else {
            0
        }
    }
}

/// Parsed UDP header information
#[derive(Debug, Clone)]
pub struct UdpPacketInfo {
    /// Source port
    pub src_port: u16,
    /// Destination port
    pub dst_port: u16,
    /// Total length (header + payload)
    pub length: u16,
    /// Checksum
    pub checksum: u16,
}

impl UdpPacketInfo {
    /// Parse UDP header from transport layer data
    pub fn parse(data: &[u8]) -> Result<Self, VoyageError> {
        if data.len() < UDP_HEADER_LEN {
            return Err(VoyageError::InvalidPacket("UDP header too short".into()));
        }

        let src_port = u16::from_be_bytes([data[0], data[1]]);
        let dst_port = u16::from_be_bytes([data[2], data[3]]);
        let length = u16::from_be_bytes([data[4], data[5]]);
        let checksum = u16::from_be_bytes([data[6], data[7]]);

        Ok(Self {
            src_port,
            dst_port,
            length,
            checksum,
        })
    }

    /// Get UDP payload
    pub fn get_payload<'a>(&self, data: &'a [u8]) -> &'a [u8] {
        if data.len() > UDP_HEADER_LEN {
            &data[UDP_HEADER_LEN..]
        } else {
            &[]
        }
    }

    /// Get payload length
    pub fn payload_len(&self) -> usize {
        if self.length > UDP_HEADER_LEN as u16 {
            (self.length - UDP_HEADER_LEN as u16) as usize
        } else {
            0
        }
    }
}

/// Complete parsed packet info
#[derive(Debug, Clone)]
pub struct ParsedPacket {
    /// IP layer info
    pub ip: IpPacketInfo,
    /// TCP info (if TCP packet)
    pub tcp: Option<TcpPacketInfo>,
    /// UDP info (if UDP packet)
    pub udp: Option<UdpPacketInfo>,
}

impl ParsedPacket {
    /// Parse a complete IP packet
    pub fn parse(data: &[u8]) -> Result<Self, VoyageError> {
        let ip = IpPacketInfo::parse(data)?;

        let transport_data = ip.get_payload(data);

        let (tcp, udp) = match ip.protocol {
            TransportProtocol::Tcp => (Some(TcpPacketInfo::parse(transport_data)?), None),
            TransportProtocol::Udp => (None, Some(UdpPacketInfo::parse(transport_data)?)),
            _ => (None, None),
        };

        Ok(Self { ip, tcp, udp })
    }

    /// Get source socket address (for TCP/UDP)
    pub fn src_addr(&self) -> Option<SocketAddr> {
        if let Some(ref tcp) = self.tcp {
            Some(SocketAddr::new(self.ip.src_ip, tcp.src_port))
        } else if let Some(ref udp) = self.udp {
            Some(SocketAddr::new(self.ip.src_ip, udp.src_port))
        } else {
            None
        }
    }

    /// Get destination socket address (for TCP/UDP)
    pub fn dst_addr(&self) -> Option<SocketAddr> {
        if let Some(ref tcp) = self.tcp {
            Some(SocketAddr::new(self.ip.dst_ip, tcp.dst_port))
        } else if let Some(ref udp) = self.udp {
            Some(SocketAddr::new(self.ip.dst_ip, udp.dst_port))
        } else {
            None
        }
    }

    /// Create a NAT key for this packet
    pub fn to_nat_key(&self) -> Option<NatKey> {
        let src = self.src_addr()?;
        let dst = self.dst_addr()?;

        match self.ip.protocol {
            TransportProtocol::Tcp => Some(NatKey::tcp(src, dst)),
            TransportProtocol::Udp => Some(NatKey::udp(src, dst)),
            _ => None,
        }
    }

    /// Check if this is a TCP SYN packet
    pub fn is_tcp_syn(&self) -> bool {
        self.tcp.as_ref().map(|t| t.flags.is_syn()).unwrap_or(false)
    }

    /// Check if this is a TCP FIN packet
    pub fn is_tcp_fin(&self) -> bool {
        self.tcp.as_ref().map(|t| t.flags.is_fin()).unwrap_or(false)
    }

    /// Check if this is a TCP RST packet
    pub fn is_tcp_rst(&self) -> bool {
        self.tcp.as_ref().map(|t| t.flags.is_rst()).unwrap_or(false)
    }

    /// Get TCP payload if available
    pub fn tcp_payload<'a>(&self, data: &'a [u8]) -> Option<&'a [u8]> {
        let transport_data = self.ip.get_payload(data);
        self.tcp.as_ref().map(|t| t.get_payload(transport_data))
    }

    /// Get UDP payload if available
    pub fn udp_payload<'a>(&self, data: &'a [u8]) -> Option<&'a [u8]> {
        let transport_data = self.ip.get_payload(data);
        self.udp.as_ref().map(|u| u.get_payload(transport_data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a minimal IPv4 TCP SYN packet
    fn make_ipv4_tcp_syn() -> Vec<u8> {
        let mut packet = vec![0u8; 40]; // 20 byte IP + 20 byte TCP

        // IPv4 header
        packet[0] = 0x45; // Version 4, IHL 5
        packet[2] = 0x00; // Total length
        packet[3] = 0x28; // 40 bytes
        packet[9] = 0x06; // TCP

        // Source IP: 192.168.1.1
        packet[12] = 192;
        packet[13] = 168;
        packet[14] = 1;
        packet[15] = 1;

        // Dest IP: 8.8.8.8
        packet[16] = 8;
        packet[17] = 8;
        packet[18] = 8;
        packet[19] = 8;

        // TCP header
        packet[20] = 0x30; // Source port 12345 >> 8
        packet[21] = 0x39; // Source port 12345 & 0xff
        packet[22] = 0x01; // Dest port 443 >> 8
        packet[23] = 0xBB; // Dest port 443 & 0xff
        packet[32] = 0x50; // Data offset 5 (20 bytes)
        packet[33] = 0x02; // SYN flag

        packet
    }

    /// Create a minimal IPv4 UDP packet
    fn make_ipv4_udp() -> Vec<u8> {
        let mut packet = vec![0u8; 28]; // 20 byte IP + 8 byte UDP

        // IPv4 header
        packet[0] = 0x45; // Version 4, IHL 5
        packet[2] = 0x00; // Total length
        packet[3] = 0x1C; // 28 bytes
        packet[9] = 0x11; // UDP

        // Source IP: 10.0.0.1
        packet[12] = 10;
        packet[13] = 0;
        packet[14] = 0;
        packet[15] = 1;

        // Dest IP: 8.8.8.8
        packet[16] = 8;
        packet[17] = 8;
        packet[18] = 8;
        packet[19] = 8;

        // UDP header
        packet[20] = 0x1F; // Source port 8000 >> 8
        packet[21] = 0x40; // Source port 8000 & 0xff
        packet[22] = 0x00; // Dest port 53 >> 8
        packet[23] = 0x35; // Dest port 53 & 0xff
        packet[24] = 0x00; // Length
        packet[25] = 0x08; // 8 bytes (header only)

        packet
    }

    #[test]
    fn test_parse_ipv4_tcp_syn() {
        let packet = make_ipv4_tcp_syn();
        let parsed = ParsedPacket::parse(&packet).unwrap();

        assert_eq!(parsed.ip.version, IpVersion::V4);
        assert_eq!(
            parsed.ip.src_ip,
            IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))
        );
        assert_eq!(parsed.ip.dst_ip, IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)));
        assert!(matches!(parsed.ip.protocol, TransportProtocol::Tcp));

        let tcp = parsed.tcp.unwrap();
        assert_eq!(tcp.src_port, 12345);
        assert_eq!(tcp.dst_port, 443);
        assert!(tcp.flags.is_syn());
    }

    #[test]
    fn test_parse_ipv4_udp() {
        let packet = make_ipv4_udp();
        let parsed = ParsedPacket::parse(&packet).unwrap();

        assert_eq!(parsed.ip.version, IpVersion::V4);
        assert!(matches!(parsed.ip.protocol, TransportProtocol::Udp));

        let udp = parsed.udp.unwrap();
        assert_eq!(udp.src_port, 8000);
        assert_eq!(udp.dst_port, 53);
    }

    #[test]
    fn test_tcp_flags() {
        let syn = TcpFlags::from_byte(0x02);
        assert!(syn.is_syn());
        assert!(!syn.is_fin());
        assert!(!syn.is_rst());

        let syn_ack = TcpFlags::from_byte(0x12);
        assert!(syn_ack.is_syn_ack());

        let fin = TcpFlags::from_byte(0x11);
        assert!(fin.is_fin());
        assert!(fin.ack);

        let rst = TcpFlags::from_byte(0x04);
        assert!(rst.is_rst());
    }

    #[test]
    fn test_flags_roundtrip() {
        let flags = TcpFlags {
            fin: true,
            syn: false,
            rst: false,
            psh: true,
            ack: true,
            urg: false,
            ece: false,
            cwr: false,
        };

        let byte = flags.to_byte();
        let parsed = TcpFlags::from_byte(byte);

        assert_eq!(parsed.fin, flags.fin);
        assert_eq!(parsed.syn, flags.syn);
        assert_eq!(parsed.psh, flags.psh);
        assert_eq!(parsed.ack, flags.ack);
    }

    #[test]
    fn test_nat_key_creation() {
        let packet = make_ipv4_tcp_syn();
        let parsed = ParsedPacket::parse(&packet).unwrap();

        let key = parsed.to_nat_key().unwrap();
        assert!(key.is_tcp());
        assert_eq!(key.src_port, 12345);
        assert_eq!(key.dst_port, 443);
    }

    #[test]
    fn test_src_dst_addr() {
        let packet = make_ipv4_tcp_syn();
        let parsed = ParsedPacket::parse(&packet).unwrap();

        let src = parsed.src_addr().unwrap();
        let dst = parsed.dst_addr().unwrap();

        assert_eq!(src.port(), 12345);
        assert_eq!(dst.port(), 443);
    }

    #[test]
    fn test_empty_packet() {
        let result = ParsedPacket::parse(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_too_short_packet() {
        let result = ParsedPacket::parse(&[0x45, 0x00]);
        assert!(result.is_err());
    }

    #[test]
    fn test_transport_protocol_conversion() {
        assert!(matches!(
            TransportProtocol::from_proto(6),
            TransportProtocol::Tcp
        ));
        assert!(matches!(
            TransportProtocol::from_proto(17),
            TransportProtocol::Udp
        ));
        assert!(matches!(
            TransportProtocol::from_proto(1),
            TransportProtocol::Icmp
        ));
        assert!(matches!(
            TransportProtocol::from_proto(99),
            TransportProtocol::Other(99)
        ));

        assert_eq!(TransportProtocol::Tcp.to_proto(), 6);
        assert_eq!(TransportProtocol::Udp.to_proto(), 17);
    }
}
