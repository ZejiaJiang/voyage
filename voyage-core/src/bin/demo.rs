//! Demo binary for Windows
//!
//! This demonstrates the voyage-core functionality on Windows,
//! simulating packet processing without the iOS Network Extension.

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};

use voyage_core::config::ProxyConfig;
use voyage_core::connection::ConnectionManager;
use voyage_core::device::VirtualTunDevice;
use voyage_core::nat::{NatKey, NatManager};
use voyage_core::packet::ParsedPacket;
use voyage_core::proxy::ProxyManager;
use voyage_core::rule::RuleEngine;

fn main() {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    println!("=== Voyage Core Demo ===\n");

    // Demo 1: Packet Parsing
    demo_packet_parsing();

    // Demo 2: NAT Manager
    demo_nat_manager();

    // Demo 3: Rule Engine
    demo_rule_engine();

    // Demo 4: Proxy Manager
    demo_proxy_manager();

    // Demo 5: Full Pipeline
    demo_full_pipeline();

    println!("\n=== Demo Complete ===");
}

fn demo_packet_parsing() {
    println!("--- Demo 1: Packet Parsing ---");

    // Create a sample TCP SYN packet
    let packet = create_tcp_syn_packet(12345, 443, [10, 0, 0, 1], [8, 8, 8, 8]);

    match ParsedPacket::parse(&packet) {
        Ok(parsed) => {
            println!("  Parsed TCP SYN packet:");
            println!("    Source: {:?}:{}", parsed.ip.src_ip, parsed.tcp.as_ref().unwrap().src_port);
            println!("    Dest: {:?}:{}", parsed.ip.dst_ip, parsed.tcp.as_ref().unwrap().dst_port);
            println!("    Is SYN: {}", parsed.is_tcp_syn());
        }
        Err(e) => {
            println!("  Error parsing packet: {:?}", e);
        }
    }
    println!();
}

fn demo_nat_manager() {
    println!("--- Demo 2: NAT Manager ---");

    let mut nat = NatManager::new();

    // Create some connections
    for i in 0..5 {
        let src = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(10, 0, 0, 1), 10000 + i));
        let dst = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(8, 8, 8, 8), 443));
        let key = NatKey::tcp(src, dst);

        let entry = nat.get_or_create(key).unwrap();
        println!("  Connection {}: local_port={}", i, entry.local_port);
    }

    println!("  Total connections: {}", nat.len());
    println!();
}

fn demo_rule_engine() {
    println!("--- Demo 3: Rule Engine ---");

    let mut engine = RuleEngine::new();

    let rules_config = r#"
DOMAIN-SUFFIX, .google.com, PROXY
DOMAIN-SUFFIX, .github.com, PROXY
DOMAIN-KEYWORD, facebook, REJECT
IP-CIDR, 10.0.0.0/8, DIRECT
IP-CIDR, 192.168.0.0/16, DIRECT
FINAL, DIRECT
"#;

    match engine.load_from_config(rules_config) {
        Ok(count) => println!("  Loaded {} rules", count),
        Err(e) => println!("  Error loading rules: {}", e),
    }

    // Test some domains
    let test_cases = [
        ("www.google.com", None),
        ("github.com", None),
        ("facebook.com", None),
        ("example.com", None),
        ("internal", Some(std::net::IpAddr::V4(Ipv4Addr::new(10, 1, 2, 3)))),
    ];

    for (domain, ip) in test_cases {
        let action = engine.evaluate(Some(domain), ip, 443, 0);
        println!("  {} -> {:?}", domain, action);
    }
    println!();
}

fn demo_proxy_manager() {
    println!("--- Demo 4: Proxy Manager ---");

    let mut manager = ProxyManager::with_config(ProxyConfig {
        server_host: "proxy.example.com".into(),
        server_port: 1080,
        username: Some("user".into()),
        password: Some("secret".into()),
    });

    manager
        .load_rules(
            r#"
DOMAIN-SUFFIX, .google.com, PROXY
FINAL, DIRECT
"#,
        )
        .unwrap();

    println!("  Proxy enabled: {}", manager.is_enabled());
    println!("  Rules loaded: {}", manager.rule_count());

    // Evaluate some routes
    let domains = ["www.google.com", "example.com", "mail.google.com"];
    for domain in domains {
        let decision = manager.evaluate_route(Some(domain), None, 443, 0);
        println!("  {} -> {:?}", domain, decision.action);
    }

    let stats = manager.get_stats();
    println!("  Stats: proxied={}, direct={}", stats.proxied_connections, stats.direct_connections);
    println!();
}

fn demo_full_pipeline() {
    println!("--- Demo 5: Full Pipeline ---");

    // Create components
    let device = VirtualTunDevice::new();
    let _rx_queue = device.rx_queue();
    let _tx_queue = device.tx_queue();
    let _device = device;
    let mut conn_manager = ConnectionManager::new();
    let mut proxy_manager = ProxyManager::with_config(ProxyConfig {
        server_host: "127.0.0.1".into(),
        server_port: 1080,
        username: None,
        password: None,
    });

    proxy_manager
        .load_rules("IP-CIDR, 8.8.0.0/16, PROXY\nFINAL, DIRECT")
        .unwrap();

    // Simulate packets from an app
    let packets = [
        create_tcp_syn_packet(12345, 443, [10, 0, 0, 1], [8, 8, 8, 8]),     // To 8.8.8.8 - proxy
        create_tcp_syn_packet(12346, 80, [10, 0, 0, 1], [93, 184, 216, 34]), // To example.com - direct
        create_tcp_syn_packet(12347, 443, [10, 0, 0, 1], [8, 8, 4, 4]),     // To 8.8.4.4 - proxy
    ];

    for (i, packet) in packets.iter().enumerate() {
        let parsed = ParsedPacket::parse(packet).unwrap();
        let _conn_info = conn_manager.process_packet(&parsed).unwrap();
        let decision = proxy_manager.evaluate_route(
            None,
            parsed.dst_addr().map(|a| a.ip()),
            parsed.tcp.as_ref().unwrap().dst_port,
            0,
        );

        println!(
            "  Packet {}: {:?}:{} -> {:?}",
            i,
            parsed.ip.dst_ip,
            parsed.tcp.as_ref().unwrap().dst_port,
            decision.action
        );
    }

    println!("  Active connections: {}", conn_manager.active_connections());
    println!();
}

fn create_tcp_syn_packet(src_port: u16, dst_port: u16, src_ip: [u8; 4], dst_ip: [u8; 4]) -> Vec<u8> {
    let mut packet = vec![0u8; 40];

    // IPv4 header
    packet[0] = 0x45;
    packet[2] = 0x00;
    packet[3] = 0x28;
    packet[9] = 0x06;

    // Source IP
    packet[12..16].copy_from_slice(&src_ip);

    // Dest IP
    packet[16..20].copy_from_slice(&dst_ip);

    // TCP header
    packet[20] = (src_port >> 8) as u8;
    packet[21] = (src_port & 0xFF) as u8;
    packet[22] = (dst_port >> 8) as u8;
    packet[23] = (dst_port & 0xFF) as u8;
    packet[32] = 0x50;
    packet[33] = 0x02; // SYN

    packet
}
