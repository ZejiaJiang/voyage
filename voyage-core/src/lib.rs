//! Voyage Core - Cross-platform network engine for iOS/macOS proxy app
//!
//! This crate provides the core networking functionality using smoltcp
//! for userspace TCP/IP stack processing.

// Public modules
pub mod config;
pub mod connection;
pub mod device;
pub mod error;
pub mod ffi;
pub mod iface;
pub mod nat;
pub mod packet;
pub mod proxy;
pub mod rule;
pub mod socks5;

// Re-exports for convenience
pub use config::ProxyConfig;
pub use connection::{ConnectionInfo, ConnectionManager, ConnectionState};
pub use device::{PacketQueue, VirtualTunDevice, MTU};
pub use error::VoyageError;
pub use iface::InterfaceManager;
pub use nat::{NatEntry, NatKey, NatManager, NatState};
pub use packet::{IpPacketInfo, ParsedPacket, TcpFlags, TcpPacketInfo, UdpPacketInfo};
pub use proxy::{ProxyManager, ProxyStats, RoutingDecision};
pub use rule::{FfiRouteAction, RouteAction, Rule, RuleEngine, RuleType};
pub use socks5::{Socks5Client, TargetAddr};

// FFI exports
pub use ffi::{
    add_bytes_received, add_bytes_sent, clear_rules, disable_proxy, enable_proxy,
    evaluate_route, get_stats, init_core, is_initialized, is_proxy_enabled, load_rules,
    process_inbound_packet, process_outbound_packet, rule_count, shutdown_core, CoreStats,
};


/// The main core engine
pub struct VoyageCore {
    /// Proxy configuration
    pub config: ProxyConfig,
    /// Connection manager
    pub conn_manager: ConnectionManager,
    /// Proxy manager
    pub proxy_manager: ProxyManager,
}

impl VoyageCore {
    /// Create a new VoyageCore with the given configuration
    pub fn new(config: ProxyConfig) -> Self {
        log::info!(
            "Creating VoyageCore with proxy: {}:{}",
            config.server_host,
            config.server_port
        );

        let proxy_manager = ProxyManager::with_config(config.clone());

        Self {
            config,
            conn_manager: ConnectionManager::new(),
            proxy_manager,
        }
    }

    /// Load routing rules from a configuration string
    pub fn load_rules(&mut self, rules_text: &str) -> Result<usize, VoyageError> {
        self.proxy_manager.load_rules(rules_text)
    }

    /// Evaluate routing for a domain
    pub fn should_proxy_domain(&mut self, domain: &str) -> bool {
        let decision = self.proxy_manager.evaluate_route(Some(domain), None, 443, 0);
        matches!(decision.action, RouteAction::Proxy)
    }

    /// Get current statistics
    pub fn get_stats(&self) -> CoreStats {
        CoreStats {
            bytes_sent: self.conn_manager.total_bytes_sent(),
            bytes_received: self.conn_manager.total_bytes_received(),
            active_connections: self.conn_manager.active_connections() as u64,
            total_connections: self.conn_manager.total_connections(),
        }
    }

    /// Enable the proxy
    pub fn enable(&mut self) {
        self.proxy_manager.enable();
    }

    /// Disable the proxy
    pub fn disable(&mut self) {
        self.proxy_manager.disable();
    }

    /// Check if proxy is enabled
    pub fn is_enabled(&self) -> bool {
        self.proxy_manager.is_enabled()
    }
}

// UniFFI scaffolding
uniffi::include_scaffolding!("voyage_core");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_voyage_core_creation() {
        let config = ProxyConfig {
            server_host: "127.0.0.1".into(),
            server_port: 1080,
            username: None,
            password: None,
        };

        let core = VoyageCore::new(config);
        assert!(core.is_enabled());
    }

    #[test]
    fn test_load_rules() {
        let config = ProxyConfig {
            server_host: "127.0.0.1".into(),
            server_port: 1080,
            username: None,
            password: None,
        };

        let mut core = VoyageCore::new(config);
        let count = core.load_rules("FINAL, DIRECT").unwrap();
        assert_eq!(count, 1);
    }

    #[test]
    fn test_should_proxy_domain() {
        let config = ProxyConfig {
            server_host: "127.0.0.1".into(),
            server_port: 1080,
            username: None,
            password: None,
        };

        let mut core = VoyageCore::new(config);
        core.load_rules(
            r#"
DOMAIN-SUFFIX, .google.com, PROXY
FINAL, DIRECT
"#,
        )
        .unwrap();

        assert!(core.should_proxy_domain("www.google.com"));
        assert!(!core.should_proxy_domain("example.com"));
    }

    #[test]
    fn test_get_stats() {
        let config = ProxyConfig {
            server_host: "127.0.0.1".into(),
            server_port: 1080,
            username: None,
            password: None,
        };

        let core = VoyageCore::new(config);
        let stats = core.get_stats();

        assert_eq!(stats.bytes_sent, 0);
        assert_eq!(stats.bytes_received, 0);
        assert_eq!(stats.active_connections, 0);
    }

    #[test]
    fn test_enable_disable() {
        let config = ProxyConfig {
            server_host: "127.0.0.1".into(),
            server_port: 1080,
            username: None,
            password: None,
        };

        let mut core = VoyageCore::new(config);

        assert!(core.is_enabled());

        core.disable();
        assert!(!core.is_enabled());

        core.enable();
        assert!(core.is_enabled());
    }
}

/// Helper function to create a TCP packet for testing
pub fn create_tcp_packet(
    src_ip: [u8; 4],
    dst_ip: [u8; 4],
    src_port: u16,
    dst_port: u16,
    syn: bool,
) -> Vec<u8> {
    let mut packet = vec![0u8; 40];
    
    // IPv4 header
    packet[0] = 0x45; // Version 4, IHL 5
    packet[1] = 0x00; // DSCP/ECN
    packet[2] = 0x00; // Total length (high)
    packet[3] = 0x28; // Total length (low) = 40
    packet[4..6].copy_from_slice(&[0x00, 0x00]); // ID
    packet[6..8].copy_from_slice(&[0x40, 0x00]); // Flags + Fragment
    packet[8] = 64; // TTL
    packet[9] = 6; // Protocol: TCP
    packet[10..12].copy_from_slice(&[0x00, 0x00]); // Checksum (placeholder)
    packet[12..16].copy_from_slice(&src_ip);
    packet[16..20].copy_from_slice(&dst_ip);
    
    // TCP header
    packet[20] = (src_port >> 8) as u8;
    packet[21] = src_port as u8;
    packet[22] = (dst_port >> 8) as u8;
    packet[23] = dst_port as u8;
    packet[24..28].copy_from_slice(&[0x00, 0x00, 0x00, 0x01]); // Seq
    packet[28..32].copy_from_slice(&[0x00, 0x00, 0x00, 0x00]); // Ack
    packet[32] = 0x50; // Data offset (5 words)
    packet[33] = if syn { 0x02 } else { 0x10 }; // Flags: SYN or ACK
    packet[34..36].copy_from_slice(&[0xFF, 0xFF]); // Window
    packet[36..38].copy_from_slice(&[0x00, 0x00]); // Checksum
    packet[38..40].copy_from_slice(&[0x00, 0x00]); // Urgent ptr
    
    packet
}

