//! SOCKS5 Client Implementation
//!
//! This module provides a SOCKS5 client for proxying TCP connections
//! through a SOCKS5 proxy server.

use std::net::{Ipv4Addr, Ipv6Addr, SocketAddr, SocketAddrV4, SocketAddrV6};

use bytes::{BufMut, BytesMut};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::error::VoyageError;

/// SOCKS5 version
const SOCKS5_VERSION: u8 = 0x05;

/// SOCKS5 authentication methods
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AuthMethod {
    /// No authentication required
    NoAuth = 0x00,
    /// Username/password authentication
    UsernamePassword = 0x02,
    /// No acceptable methods
    NoAcceptable = 0xFF,
}

impl From<u8> for AuthMethod {
    fn from(value: u8) -> Self {
        match value {
            0x00 => AuthMethod::NoAuth,
            0x02 => AuthMethod::UsernamePassword,
            _ => AuthMethod::NoAcceptable,
        }
    }
}

/// SOCKS5 command types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Command {
    /// Connect to a destination
    Connect = 0x01,
    /// Bind a port
    Bind = 0x02,
    /// UDP associate
    UdpAssociate = 0x03,
}

/// SOCKS5 address types
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum AddressType {
    /// IPv4 address
    IPv4 = 0x01,
    /// Domain name
    DomainName = 0x03,
    /// IPv6 address
    IPv6 = 0x04,
}

/// SOCKS5 reply codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ReplyCode {
    /// Succeeded
    Succeeded = 0x00,
    /// General SOCKS server failure
    GeneralFailure = 0x01,
    /// Connection not allowed by ruleset
    ConnectionNotAllowed = 0x02,
    /// Network unreachable
    NetworkUnreachable = 0x03,
    /// Host unreachable
    HostUnreachable = 0x04,
    /// Connection refused
    ConnectionRefused = 0x05,
    /// TTL expired
    TtlExpired = 0x06,
    /// Command not supported
    CommandNotSupported = 0x07,
    /// Address type not supported
    AddressTypeNotSupported = 0x08,
}

impl From<u8> for ReplyCode {
    fn from(value: u8) -> Self {
        match value {
            0x00 => ReplyCode::Succeeded,
            0x01 => ReplyCode::GeneralFailure,
            0x02 => ReplyCode::ConnectionNotAllowed,
            0x03 => ReplyCode::NetworkUnreachable,
            0x04 => ReplyCode::HostUnreachable,
            0x05 => ReplyCode::ConnectionRefused,
            0x06 => ReplyCode::TtlExpired,
            0x07 => ReplyCode::CommandNotSupported,
            0x08 => ReplyCode::AddressTypeNotSupported,
            _ => ReplyCode::GeneralFailure,
        }
    }
}

impl ReplyCode {
    /// Convert to error message
    pub fn to_error_message(&self) -> &'static str {
        match self {
            ReplyCode::Succeeded => "Succeeded",
            ReplyCode::GeneralFailure => "General SOCKS server failure",
            ReplyCode::ConnectionNotAllowed => "Connection not allowed by ruleset",
            ReplyCode::NetworkUnreachable => "Network unreachable",
            ReplyCode::HostUnreachable => "Host unreachable",
            ReplyCode::ConnectionRefused => "Connection refused",
            ReplyCode::TtlExpired => "TTL expired",
            ReplyCode::CommandNotSupported => "Command not supported",
            ReplyCode::AddressTypeNotSupported => "Address type not supported",
        }
    }
}

/// Target address for SOCKS5 connection
#[derive(Debug, Clone)]
pub enum TargetAddr {
    /// IPv4 address
    Ip(SocketAddr),
    /// Domain name with port
    Domain(String, u16),
}

impl TargetAddr {
    /// Create from socket address
    pub fn from_socket_addr(addr: SocketAddr) -> Self {
        TargetAddr::Ip(addr)
    }

    /// Create from domain and port
    pub fn from_domain(domain: impl Into<String>, port: u16) -> Self {
        TargetAddr::Domain(domain.into(), port)
    }

    /// Get the port
    pub fn port(&self) -> u16 {
        match self {
            TargetAddr::Ip(addr) => addr.port(),
            TargetAddr::Domain(_, port) => *port,
        }
    }

    /// Encode the address for SOCKS5 protocol
    pub fn encode(&self) -> BytesMut {
        let mut buf = BytesMut::new();

        match self {
            TargetAddr::Ip(SocketAddr::V4(addr)) => {
                buf.put_u8(AddressType::IPv4 as u8);
                buf.put_slice(&addr.ip().octets());
                buf.put_u16(addr.port());
            }
            TargetAddr::Ip(SocketAddr::V6(addr)) => {
                buf.put_u8(AddressType::IPv6 as u8);
                buf.put_slice(&addr.ip().octets());
                buf.put_u16(addr.port());
            }
            TargetAddr::Domain(domain, port) => {
                buf.put_u8(AddressType::DomainName as u8);
                let domain_bytes = domain.as_bytes();
                buf.put_u8(domain_bytes.len() as u8);
                buf.put_slice(domain_bytes);
                buf.put_u16(*port);
            }
        }

        buf
    }
}

/// SOCKS5 client for establishing proxy connections
pub struct Socks5Client {
    /// Proxy server address
    proxy_addr: SocketAddr,
    /// Username for authentication
    username: Option<String>,
    /// Password for authentication
    password: Option<String>,
}

impl Socks5Client {
    /// Create a new SOCKS5 client
    pub fn new(proxy_addr: SocketAddr) -> Self {
        Self {
            proxy_addr,
            username: None,
            password: None,
        }
    }

    /// Create a new SOCKS5 client with authentication
    pub fn with_auth(
        proxy_addr: SocketAddr,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        Self {
            proxy_addr,
            username: Some(username.into()),
            password: Some(password.into()),
        }
    }

    /// Connect to the target through the SOCKS5 proxy
    pub async fn connect(&self, target: TargetAddr) -> Result<TcpStream, VoyageError> {
        // Connect to the proxy server
        let mut stream = TcpStream::connect(self.proxy_addr)
            .await
            .map_err(|e| VoyageError::IoError(e.to_string()))?;

        // Perform handshake
        self.handshake(&mut stream).await?;

        // Send connect request
        self.send_connect_request(&mut stream, &target).await?;

        Ok(stream)
    }

    /// Perform SOCKS5 handshake
    async fn handshake(&self, stream: &mut TcpStream) -> Result<(), VoyageError> {
        // Build greeting message
        let mut greeting = BytesMut::new();
        greeting.put_u8(SOCKS5_VERSION);

        if self.username.is_some() && self.password.is_some() {
            greeting.put_u8(2); // 2 methods
            greeting.put_u8(AuthMethod::NoAuth as u8);
            greeting.put_u8(AuthMethod::UsernamePassword as u8);
        } else {
            greeting.put_u8(1); // 1 method
            greeting.put_u8(AuthMethod::NoAuth as u8);
        }

        stream
            .write_all(&greeting)
            .await
            .map_err(|e| VoyageError::IoError(e.to_string()))?;

        // Read server response
        let mut response = [0u8; 2];
        stream
            .read_exact(&mut response)
            .await
            .map_err(|e| VoyageError::IoError(e.to_string()))?;

        if response[0] != SOCKS5_VERSION {
            return Err(VoyageError::Socks5Error("Invalid SOCKS version".into()));
        }

        let method = AuthMethod::from(response[1]);

        match method {
            AuthMethod::NoAuth => Ok(()),
            AuthMethod::UsernamePassword => self.authenticate(stream).await,
            AuthMethod::NoAcceptable => {
                Err(VoyageError::Socks5Error("No acceptable auth method".into()))
            }
        }
    }

    /// Perform username/password authentication
    async fn authenticate(&self, stream: &mut TcpStream) -> Result<(), VoyageError> {
        let username = self.username.as_ref().ok_or_else(|| {
            VoyageError::Socks5Error("Authentication required but no username".into())
        })?;
        let password = self.password.as_ref().ok_or_else(|| {
            VoyageError::Socks5Error("Authentication required but no password".into())
        })?;

        let mut auth_request = BytesMut::new();
        auth_request.put_u8(0x01); // Auth version
        auth_request.put_u8(username.len() as u8);
        auth_request.put_slice(username.as_bytes());
        auth_request.put_u8(password.len() as u8);
        auth_request.put_slice(password.as_bytes());

        stream
            .write_all(&auth_request)
            .await
            .map_err(|e| VoyageError::IoError(e.to_string()))?;

        let mut response = [0u8; 2];
        stream
            .read_exact(&mut response)
            .await
            .map_err(|e| VoyageError::IoError(e.to_string()))?;

        if response[1] != 0x00 {
            return Err(VoyageError::Socks5Error("Authentication failed".into()));
        }

        Ok(())
    }

    /// Send SOCKS5 connect request
    async fn send_connect_request(
        &self,
        stream: &mut TcpStream,
        target: &TargetAddr,
    ) -> Result<(), VoyageError> {
        let mut request = BytesMut::new();
        request.put_u8(SOCKS5_VERSION);
        request.put_u8(Command::Connect as u8);
        request.put_u8(0x00); // Reserved
        request.put(target.encode());

        stream
            .write_all(&request)
            .await
            .map_err(|e| VoyageError::IoError(e.to_string()))?;

        // Read response header
        let mut header = [0u8; 4];
        stream
            .read_exact(&mut header)
            .await
            .map_err(|e| VoyageError::IoError(e.to_string()))?;

        if header[0] != SOCKS5_VERSION {
            return Err(VoyageError::Socks5Error("Invalid SOCKS version in reply".into()));
        }

        let reply_code = ReplyCode::from(header[1]);
        if reply_code != ReplyCode::Succeeded {
            return Err(VoyageError::Socks5Error(
                reply_code.to_error_message().into(),
            ));
        }

        // Read and discard bound address
        let addr_type = header[3];
        match addr_type {
            0x01 => {
                // IPv4: 4 bytes + 2 port
                let mut addr = [0u8; 6];
                stream
                    .read_exact(&mut addr)
                    .await
                    .map_err(|e| VoyageError::IoError(e.to_string()))?;
            }
            0x03 => {
                // Domain: 1 byte len + domain + 2 port
                let mut len = [0u8; 1];
                stream
                    .read_exact(&mut len)
                    .await
                    .map_err(|e| VoyageError::IoError(e.to_string()))?;
                let mut domain = vec![0u8; len[0] as usize + 2];
                stream
                    .read_exact(&mut domain)
                    .await
                    .map_err(|e| VoyageError::IoError(e.to_string()))?;
            }
            0x04 => {
                // IPv6: 16 bytes + 2 port
                let mut addr = [0u8; 18];
                stream
                    .read_exact(&mut addr)
                    .await
                    .map_err(|e| VoyageError::IoError(e.to_string()))?;
            }
            _ => {
                return Err(VoyageError::Socks5Error(
                    "Unknown address type in reply".into(),
                ));
            }
        }

        Ok(())
    }
}

/// Helper function to create a SOCKS5 client from host and port
pub fn create_socks5_client(
    host: &str,
    port: u16,
    username: Option<&str>,
    password: Option<&str>,
) -> Result<Socks5Client, VoyageError> {
    // Try to parse as IP address first
    let addr: SocketAddr = if let Ok(ip) = host.parse::<Ipv4Addr>() {
        SocketAddr::V4(SocketAddrV4::new(ip, port))
    } else if let Ok(ip) = host.parse::<Ipv6Addr>() {
        SocketAddr::V6(SocketAddrV6::new(ip, port, 0, 0))
    } else {
        // For hostnames, we need to resolve - this is a simplified version
        return Err(VoyageError::ConfigError(
            "Hostname resolution not supported in sync context".into(),
        ));
    };

    Ok(match (username, password) {
        (Some(u), Some(p)) => Socks5Client::with_auth(addr, u, p),
        _ => Socks5Client::new(addr),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_method_from() {
        assert_eq!(AuthMethod::from(0x00), AuthMethod::NoAuth);
        assert_eq!(AuthMethod::from(0x02), AuthMethod::UsernamePassword);
        assert_eq!(AuthMethod::from(0xFF), AuthMethod::NoAcceptable);
        assert_eq!(AuthMethod::from(0x99), AuthMethod::NoAcceptable);
    }

    #[test]
    fn test_reply_code_from() {
        assert_eq!(ReplyCode::from(0x00), ReplyCode::Succeeded);
        assert_eq!(ReplyCode::from(0x01), ReplyCode::GeneralFailure);
        assert_eq!(ReplyCode::from(0x05), ReplyCode::ConnectionRefused);
        assert_eq!(ReplyCode::from(0x99), ReplyCode::GeneralFailure);
    }

    #[test]
    fn test_target_addr_ipv4() {
        let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 8080));
        let target = TargetAddr::from_socket_addr(addr);

        assert_eq!(target.port(), 8080);

        let encoded = target.encode();
        assert_eq!(encoded[0], AddressType::IPv4 as u8);
        assert_eq!(&encoded[1..5], &[127, 0, 0, 1]);
        assert_eq!(&encoded[5..7], &[0x1F, 0x90]); // 8080 in big endian
    }

    #[test]
    fn test_target_addr_domain() {
        let target = TargetAddr::from_domain("example.com", 443);

        assert_eq!(target.port(), 443);

        let encoded = target.encode();
        assert_eq!(encoded[0], AddressType::DomainName as u8);
        assert_eq!(encoded[1], 11); // "example.com".len()
        assert_eq!(&encoded[2..13], b"example.com");
        assert_eq!(&encoded[13..15], &[0x01, 0xBB]); // 443 in big endian
    }

    #[test]
    fn test_target_addr_ipv6() {
        let addr = SocketAddr::V6(SocketAddrV6::new(
            Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1),
            8080,
            0,
            0,
        ));
        let target = TargetAddr::from_socket_addr(addr);

        assert_eq!(target.port(), 8080);

        let encoded = target.encode();
        assert_eq!(encoded[0], AddressType::IPv6 as u8);
        assert_eq!(encoded.len(), 1 + 16 + 2); // type + ipv6 + port
    }

    #[test]
    fn test_socks5_client_new() {
        let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 1080));
        let client = Socks5Client::new(addr);

        assert_eq!(client.proxy_addr, addr);
        assert!(client.username.is_none());
        assert!(client.password.is_none());
    }

    #[test]
    fn test_socks5_client_with_auth() {
        let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 1080));
        let client = Socks5Client::with_auth(addr, "user", "pass");

        assert_eq!(client.proxy_addr, addr);
        assert_eq!(client.username, Some("user".to_string()));
        assert_eq!(client.password, Some("pass".to_string()));
    }

    #[test]
    fn test_reply_code_to_error_message() {
        assert_eq!(ReplyCode::Succeeded.to_error_message(), "Succeeded");
        assert_eq!(
            ReplyCode::ConnectionRefused.to_error_message(),
            "Connection refused"
        );
        assert_eq!(
            ReplyCode::NetworkUnreachable.to_error_message(),
            "Network unreachable"
        );
    }

    #[test]
    fn test_create_socks5_client_ipv4() {
        let client = create_socks5_client("127.0.0.1", 1080, None, None).unwrap();
        assert_eq!(
            client.proxy_addr,
            SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), 1080))
        );
    }

    #[test]
    fn test_create_socks5_client_with_auth() {
        let client =
            create_socks5_client("127.0.0.1", 1080, Some("user"), Some("pass")).unwrap();
        assert_eq!(client.username, Some("user".to_string()));
        assert_eq!(client.password, Some("pass".to_string()));
    }

    #[test]
    fn test_create_socks5_client_hostname_fails() {
        let result = create_socks5_client("localhost", 1080, None, None);
        assert!(result.is_err());
    }
}
