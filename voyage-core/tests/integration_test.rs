//! Integration tests for voyage-core
//!
//! These tests simulate the iOS Network Extension environment and verify
//! the complete packet processing pipeline.

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use serial_test::serial;

// Import the public API
use voyage_core::config::ProxyConfig;
use voyage_core::connection::ConnectionManager;
use voyage_core::device::VirtualTunDevice;
use voyage_core::nat::{NatKey, NatManager};
use voyage_core::packet::{ParsedPacket, TransportProtocol};
use voyage_core::proxy::ProxyManager;
use voyage_core::rule::{RouteAction, Rule, RuleEngine, RuleType};

/// Create a minimal IPv4 TCP SYN packet for testing
fn make_tcp_syn_packet(src_port: u16, dst_port: u16) -> Vec<u8> {
    let mut packet = vec![0u8; 40]; // 20 byte IP + 20 byte TCP

    // IPv4 header
    packet[0] = 0x45; // Version 4, IHL 5
    packet[2] = 0x00; // Total length high byte
    packet[3] = 0x28; // Total length: 40 bytes
    packet[9] = 0x06; // TCP protocol

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

    // TCP header - source port
    packet[20] = (src_port >> 8) as u8;
    packet[21] = (src_port & 0xFF) as u8;

    // TCP header - dest port
    packet[22] = (dst_port >> 8) as u8;
    packet[23] = (dst_port & 0xFF) as u8;

    // Data offset (5 * 4 = 20 bytes)
    packet[32] = 0x50;

    // Flags: SYN
    packet[33] = 0x02;

    packet
}

/// Create a minimal IPv4 UDP packet for testing
fn make_udp_packet(src_port: u16, dst_port: u16) -> Vec<u8> {
    let mut packet = vec![0u8; 28]; // 20 byte IP + 8 byte UDP

    // IPv4 header
    packet[0] = 0x45;
    packet[2] = 0x00;
    packet[3] = 0x1C; // 28 bytes
    packet[9] = 0x11; // UDP protocol

    // Source IP: 10.0.0.1
    packet[12] = 10;
    packet[13] = 0;
    packet[14] = 0;
    packet[15] = 1;

    // Dest IP: 8.8.4.4
    packet[16] = 8;
    packet[17] = 8;
    packet[18] = 4;
    packet[19] = 4;

    // UDP header
    packet[20] = (src_port >> 8) as u8;
    packet[21] = (src_port & 0xFF) as u8;
    packet[22] = (dst_port >> 8) as u8;
    packet[23] = (dst_port & 0xFF) as u8;
    packet[24] = 0x00;
    packet[25] = 0x08; // Length: 8 bytes

    packet
}

#[test]
#[serial]
fn test_full_packet_processing_pipeline() {
    // Create components
    let mut conn_manager = ConnectionManager::new();
    let mut proxy_manager = ProxyManager::with_config(ProxyConfig {
        server_host: "127.0.0.1".into(),
        server_port: 1080,
        username: None,
        password: None,
    });

    // Load rules
    proxy_manager
        .load_rules(
            r#"
DOMAIN-SUFFIX, .google.com, PROXY
IP-CIDR, 8.8.8.0/24, PROXY
FINAL, DIRECT
"#,
        )
        .unwrap();

    // Create a TCP SYN packet
    let packet = make_tcp_syn_packet(12345, 443);

    // Parse the packet
    let parsed = ParsedPacket::parse(&packet).unwrap();

    // Verify parsing
    assert!(parsed.is_tcp_syn());
    assert_eq!(parsed.tcp.as_ref().unwrap().src_port, 12345);
    assert_eq!(parsed.tcp.as_ref().unwrap().dst_port, 443);

    // Process through connection manager
    let conn_info = conn_manager.process_packet(&parsed).unwrap();
    assert!(conn_info.local_port > 0);

    // Evaluate routing (should match IP-CIDR rule for 8.8.8.8)
    let decision = proxy_manager.evaluate_route(
        None,
        parsed.dst_addr().map(|a| a.ip()),
        443,
        12345,
    );
    assert_eq!(decision.action, RouteAction::Proxy);
}

#[test]
#[serial]
fn test_nat_connection_tracking() {
    let mut nat = NatManager::new();

    // Create multiple connections
    for i in 0..100 {
        let src = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(10, 0, 0, 1), 10000 + i));
        let dst = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(8, 8, 8, 8), 443));
        let key = NatKey::tcp(src, dst);

        nat.get_or_create(key).unwrap();
    }

    assert_eq!(nat.len(), 100);

    // Verify each connection has a unique local port
    let ports: std::collections::HashSet<u16> = nat
        .get_all_connections()
        .iter()
        .map(|(_, e)| e.local_port)
        .collect();

    assert_eq!(ports.len(), 100);
}

#[test]
#[serial]
fn test_rule_engine_evaluation_order() {
    let mut engine = RuleEngine::new();

    // Rules are evaluated in order - first match wins
    engine.add_rule(Rule::new(
        RuleType::Domain("specific.google.com".into()),
        RouteAction::Reject,
    ));
    engine.add_rule(Rule::new(
        RuleType::DomainSuffix(".google.com".into()),
        RouteAction::Proxy,
    ));
    engine.add_rule(Rule::new(RuleType::Final, RouteAction::Direct));

    // Specific domain should be rejected
    assert_eq!(
        engine.evaluate(Some("specific.google.com"), None, 443, 0),
        RouteAction::Reject
    );

    // Other google.com domains should be proxied
    assert_eq!(
        engine.evaluate(Some("www.google.com"), None, 443, 0),
        RouteAction::Proxy
    );

    // Other domains should be direct
    assert_eq!(
        engine.evaluate(Some("example.com"), None, 443, 0),
        RouteAction::Direct
    );
}

#[test]
#[serial]
fn test_packet_queue_concurrent_access() {
    use std::sync::Arc;
    use std::thread;

    let device = Arc::new(std::sync::Mutex::new(VirtualTunDevice::new()));

    // Spawn producer threads
    let mut handles = vec![];

    for i in 0..4 {
        let device = Arc::clone(&device);
        handles.push(thread::spawn(move || {
            for _j in 0..100 {
                let packet = vec![i as u8; 100];
                if let Ok(d) = device.lock() {
                    d.inject_packet(packet);
                }
            }
        }));
    }

    // Wait for producers
    for h in handles {
        h.join().unwrap();
    }

    // Verify all packets were queued (by checking if device has packets)
    let mut count = 0;
    if let Ok(device) = device.lock() {
        while device.has_rx_packets() {
            // Get packets through the device's rx_queue
            if let Ok(mut queue) = device.rx_queue().lock() {
                if queue.pop_front().is_some() {
                    count += 1;
                } else {
                    break;
                }
            }
        }
    }

    assert_eq!(count, 400);
}

#[test]
#[serial]
fn test_connection_state_lifecycle() {
    let mut conn_manager = ConnectionManager::new();

    // Simulate a TCP connection lifecycle
    let packet = make_tcp_syn_packet(12345, 443);
    let parsed = ParsedPacket::parse(&packet).unwrap();
    let key = parsed.to_nat_key().unwrap();

    // Initial state (SYN sent)
    let info = conn_manager.process_packet(&parsed).unwrap();
    assert_eq!(
        info.state,
        voyage_core::connection::ConnectionState::Connecting
    );

    // Establish connection
    conn_manager.establish(&key);
    let connections = conn_manager.get_all_connections();
    let conn = connections.iter().find(|c| c.key == key).unwrap();
    assert_eq!(
        conn.state,
        voyage_core::connection::ConnectionState::Established
    );

    // Track bytes
    conn_manager.add_bytes_sent(&key, 1000);
    conn_manager.add_bytes_received(&key, 2000);

    assert_eq!(conn_manager.total_bytes_sent(), 1000);
    assert_eq!(conn_manager.total_bytes_received(), 2000);

    // Close connection
    conn_manager.close_connection(&key);
    let connections = conn_manager.get_all_connections();
    let conn = connections.iter().find(|c| c.key == key).unwrap();
    assert_eq!(conn.state, voyage_core::connection::ConnectionState::Closed);

    // Cleanup
    conn_manager.cleanup();
    assert_eq!(conn_manager.active_connections(), 0);
}

#[test]
#[serial]
fn test_udp_packet_processing() {
    let packet = make_udp_packet(8000, 53);
    let parsed = ParsedPacket::parse(&packet).unwrap();

    assert!(matches!(parsed.ip.protocol, TransportProtocol::Udp));
    assert!(parsed.tcp.is_none());
    assert!(parsed.udp.is_some());

    let udp = parsed.udp.as_ref().unwrap();
    assert_eq!(udp.src_port, 8000);
    assert_eq!(udp.dst_port, 53);

    let key = parsed.to_nat_key().unwrap();
    assert!(key.is_udp());
}

#[test]
#[serial]
fn test_proxy_stats_tracking() {
    let mut manager = ProxyManager::with_config(ProxyConfig {
        server_host: "127.0.0.1".into(),
        server_port: 1080,
        username: None,
        password: None,
    });

    manager
        .load_rules(
            r#"
DOMAIN, proxy.com, PROXY
DOMAIN, reject.com, REJECT
FINAL, DIRECT
"#,
        )
        .unwrap();

    // Generate some traffic
    for _ in 0..10 {
        manager.evaluate_route(Some("proxy.com"), None, 443, 0);
    }
    for _ in 0..5 {
        manager.evaluate_route(Some("reject.com"), None, 443, 0);
    }
    for _ in 0..20 {
        manager.evaluate_route(Some("other.com"), None, 443, 0);
    }

    let stats = manager.get_stats();
    assert_eq!(stats.proxied_connections, 10);
    assert_eq!(stats.rejected_connections, 5);
    assert_eq!(stats.direct_connections, 20);
}

#[test]
#[serial]
fn test_virtual_tun_device_creation() {
    let _device = VirtualTunDevice::new();
    // Device creation should not panic
}

#[test]
#[serial]
fn test_multiple_protocol_packets() {
    let mut conn_manager = ConnectionManager::new();

    // TCP packet
    let tcp_packet = make_tcp_syn_packet(10001, 443);
    let tcp_parsed = ParsedPacket::parse(&tcp_packet).unwrap();
    conn_manager.process_packet(&tcp_parsed).unwrap();

    // UDP packet
    let udp_packet = make_udp_packet(10002, 53);
    let udp_parsed = ParsedPacket::parse(&udp_packet).unwrap();
    conn_manager.process_packet(&udp_parsed).unwrap();

    // Should have 2 connections
    assert_eq!(conn_manager.active_connections(), 2);
}

#[test]
#[serial]
fn test_cidr_matching() {
    let mut engine = RuleEngine::new();

    engine.add_rule(Rule::new(
        RuleType::IpCidr(Ipv4Addr::new(192, 168, 0, 0), 16),
        RouteAction::Direct,
    ));
    engine.add_rule(Rule::new(
        RuleType::IpCidr(Ipv4Addr::new(10, 0, 0, 0), 8),
        RouteAction::Direct,
    ));
    engine.add_rule(Rule::new(RuleType::Final, RouteAction::Proxy));

    // Private IPs should be direct
    assert_eq!(
        engine.evaluate(
            None,
            Some(std::net::IpAddr::V4(Ipv4Addr::new(192, 168, 1, 100))),
            443,
            0
        ),
        RouteAction::Direct
    );
    assert_eq!(
        engine.evaluate(
            None,
            Some(std::net::IpAddr::V4(Ipv4Addr::new(10, 1, 2, 3))),
            443,
            0
        ),
        RouteAction::Direct
    );

    // Public IPs should be proxied
    assert_eq!(
        engine.evaluate(
            None,
            Some(std::net::IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))),
            443,
            0
        ),
        RouteAction::Proxy
    );
}

#[test]
#[serial]
fn test_config_loading() {
    let config = ProxyConfig {
        server_host: "proxy.example.com".into(),
        server_port: 1080,
        username: Some("user".into()),
        password: Some("password".into()),
    };

    let manager = ProxyManager::with_config(config.clone());

    let (host, port) = manager.get_proxy_addr().unwrap();
    assert_eq!(host, "proxy.example.com");
    assert_eq!(port, 1080);

    let (user, pass) = manager.get_credentials().unwrap();
    assert_eq!(user, "user");
    assert_eq!(pass, "password");
}

#[test]
#[serial]
fn test_rule_config_parsing() {
    let config = r#"
# Comment line
DOMAIN, example.com, DIRECT
DOMAIN-SUFFIX, .google.com, PROXY
DOMAIN-KEYWORD, facebook, REJECT
IP-CIDR, 10.0.0.0/8, DIRECT
DST-PORT, 443, PROXY
FINAL, DIRECT
"#;

    let mut engine = RuleEngine::new();
    let count = engine.load_from_config(config).unwrap();

    assert_eq!(count, 6);
    assert_eq!(engine.len(), 6);
}

#[test]
#[serial]
fn test_tcp_flags_parsing() {
    // Create packets with different flags
    let mut syn_packet = make_tcp_syn_packet(12345, 443);

    // SYN only
    let parsed = ParsedPacket::parse(&syn_packet).unwrap();
    assert!(parsed.is_tcp_syn());
    assert!(!parsed.is_tcp_fin());
    assert!(!parsed.is_tcp_rst());

    // Modify to SYN-ACK
    syn_packet[33] = 0x12;
    let parsed = ParsedPacket::parse(&syn_packet).unwrap();
    assert!(parsed.tcp.as_ref().unwrap().flags.is_syn_ack());

    // Modify to FIN-ACK
    syn_packet[33] = 0x11;
    let parsed = ParsedPacket::parse(&syn_packet).unwrap();
    assert!(parsed.is_tcp_fin());

    // Modify to RST
    syn_packet[33] = 0x04;
    let parsed = ParsedPacket::parse(&syn_packet).unwrap();
    assert!(parsed.is_tcp_rst());
}

#[test]
#[serial]
fn test_enable_disable_proxy() {
    let mut manager = ProxyManager::with_config(ProxyConfig {
        server_host: "127.0.0.1".into(),
        server_port: 1080,
        username: None,
        password: None,
    });

    manager.load_rules("FINAL, PROXY").unwrap();

    // Enabled: should return PROXY
    assert!(manager.is_enabled());
    let decision = manager.evaluate_route(Some("example.com"), None, 443, 0);
    assert_eq!(decision.action, RouteAction::Proxy);

    // Disabled: should return DIRECT
    manager.disable();
    assert!(!manager.is_enabled());
    let decision = manager.evaluate_route(Some("example.com"), None, 443, 0);
    assert_eq!(decision.action, RouteAction::Direct);

    // Re-enabled
    manager.enable();
    assert!(manager.is_enabled());
    let decision = manager.evaluate_route(Some("example.com"), None, 443, 0);
    assert_eq!(decision.action, RouteAction::Proxy);
}
