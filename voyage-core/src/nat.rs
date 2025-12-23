//! NAT (Network Address Translation) Manager
//!
//! This module provides NAT functionality to track connections between
//! the virtual TUN device and real network sockets.

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::time::{Duration, Instant};

use crate::error::VoyageError;

/// NAT table entry state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NatState {
    /// Initial state, connection being established
    SynSent,
    /// Connection established
    Established,
    /// FIN sent, waiting for ACK
    FinWait,
    /// Connection closing
    Closing,
    /// Connection closed
    Closed,
}

/// A NAT table entry tracking a single connection
#[derive(Debug, Clone)]
pub struct NatEntry {
    /// Original source address (from the app)
    pub src_addr: SocketAddr,
    /// Original destination address
    pub dst_addr: SocketAddr,
    /// Local port allocated for this connection
    pub local_port: u16,
    /// Connection state
    pub state: NatState,
    /// Last activity timestamp
    pub last_seen: Instant,
    /// Bytes sent through this connection
    pub bytes_sent: u64,
    /// Bytes received through this connection
    pub bytes_received: u64,
}

impl NatEntry {
    /// Create a new NAT entry
    pub fn new(src_addr: SocketAddr, dst_addr: SocketAddr, local_port: u16) -> Self {
        Self {
            src_addr,
            dst_addr,
            local_port,
            state: NatState::SynSent,
            last_seen: Instant::now(),
            bytes_sent: 0,
            bytes_received: 0,
        }
    }

    /// Update the last seen timestamp
    pub fn touch(&mut self) {
        self.last_seen = Instant::now();
    }

    /// Check if the entry has timed out
    pub fn is_expired(&self, timeout: Duration) -> bool {
        self.last_seen.elapsed() > timeout
    }

    /// Transition to established state
    pub fn establish(&mut self) {
        self.state = NatState::Established;
        self.touch();
    }

    /// Transition to closing state
    pub fn start_close(&mut self) {
        self.state = NatState::FinWait;
        self.touch();
    }

    /// Transition to closed state
    pub fn close(&mut self) {
        self.state = NatState::Closed;
        self.touch();
    }
}

/// Key for looking up NAT entries
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NatKey {
    /// Source IP address
    pub src_ip: IpAddr,
    /// Source port
    pub src_port: u16,
    /// Destination IP address
    pub dst_ip: IpAddr,
    /// Destination port
    pub dst_port: u16,
    /// Protocol (6 = TCP, 17 = UDP)
    pub protocol: u8,
}

impl NatKey {
    /// Create a new NAT key for TCP
    pub fn tcp(src: SocketAddr, dst: SocketAddr) -> Self {
        Self {
            src_ip: src.ip(),
            src_port: src.port(),
            dst_ip: dst.ip(),
            dst_port: dst.port(),
            protocol: 6,
        }
    }

    /// Create a new NAT key for UDP
    pub fn udp(src: SocketAddr, dst: SocketAddr) -> Self {
        Self {
            src_ip: src.ip(),
            src_port: src.port(),
            dst_ip: dst.ip(),
            dst_port: dst.port(),
            protocol: 17,
        }
    }

    /// Get source as SocketAddr
    pub fn src_addr(&self) -> SocketAddr {
        SocketAddr::new(self.src_ip, self.src_port)
    }

    /// Get destination as SocketAddr
    pub fn dst_addr(&self) -> SocketAddr {
        SocketAddr::new(self.dst_ip, self.dst_port)
    }

    /// Check if this is a TCP connection
    pub fn is_tcp(&self) -> bool {
        self.protocol == 6
    }

    /// Check if this is a UDP connection
    pub fn is_udp(&self) -> bool {
        self.protocol == 17
    }
}

/// NAT Manager for tracking connections
pub struct NatManager {
    /// NAT table mapping keys to entries
    entries: HashMap<NatKey, NatEntry>,
    /// Reverse lookup: local port -> NAT key
    port_to_key: HashMap<u16, NatKey>,
    /// Next available local port
    next_port: u16,
    /// Minimum local port
    min_port: u16,
    /// Maximum local port
    max_port: u16,
    /// Maximum number of entries
    max_entries: usize,
    /// TCP timeout duration
    tcp_timeout: Duration,
    /// UDP timeout duration
    udp_timeout: Duration,
}

impl NatManager {
    /// Create a new NAT manager with default settings
    pub fn new() -> Self {
        Self::with_config(10000, 60000, 65535)
    }

    /// Create a NAT manager with custom port range
    pub fn with_config(min_port: u16, max_port: u16, max_entries: usize) -> Self {
        Self {
            entries: HashMap::new(),
            port_to_key: HashMap::new(),
            next_port: min_port,
            min_port,
            max_port,
            max_entries,
            tcp_timeout: Duration::from_secs(300), // 5 minutes
            udp_timeout: Duration::from_secs(60),  // 1 minute
        }
    }

    /// Allocate a new local port
    fn allocate_port(&mut self) -> Result<u16, VoyageError> {
        let start_port = self.next_port;
        loop {
            let port = self.next_port;
            self.next_port = if self.next_port >= self.max_port {
                self.min_port
            } else {
                self.next_port + 1
            };

            if !self.port_to_key.contains_key(&port) {
                return Ok(port);
            }

            if self.next_port == start_port {
                return Err(VoyageError::NatTableFull);
            }
        }
    }

    /// Create or get a NAT entry for a connection
    pub fn get_or_create(&mut self, key: NatKey) -> Result<&NatEntry, VoyageError> {
        if self.entries.contains_key(&key) {
            return Ok(self.entries.get(&key).unwrap());
        }

        if self.entries.len() >= self.max_entries {
            // Try to clean up expired entries first
            self.cleanup_expired();
            if self.entries.len() >= self.max_entries {
                return Err(VoyageError::NatTableFull);
            }
        }

        let local_port = self.allocate_port()?;
        let entry = NatEntry::new(key.src_addr(), key.dst_addr(), local_port);

        self.port_to_key.insert(local_port, key);
        self.entries.insert(key, entry);

        Ok(self.entries.get(&key).unwrap())
    }

    /// Get a NAT entry by key
    pub fn get(&self, key: &NatKey) -> Option<&NatEntry> {
        self.entries.get(key)
    }

    /// Get a mutable NAT entry by key
    pub fn get_mut(&mut self, key: &NatKey) -> Option<&mut NatEntry> {
        self.entries.get_mut(key)
    }

    /// Get a NAT entry by local port
    pub fn get_by_port(&self, port: u16) -> Option<&NatEntry> {
        self.port_to_key.get(&port).and_then(|key| self.entries.get(key))
    }

    /// Get NAT key by local port
    pub fn get_key_by_port(&self, port: u16) -> Option<&NatKey> {
        self.port_to_key.get(&port)
    }

    /// Update entry state to established
    pub fn establish(&mut self, key: &NatKey) -> bool {
        if let Some(entry) = self.entries.get_mut(key) {
            entry.establish();
            true
        } else {
            false
        }
    }

    /// Update bytes sent for an entry
    pub fn add_bytes_sent(&mut self, key: &NatKey, bytes: u64) {
        if let Some(entry) = self.entries.get_mut(key) {
            entry.bytes_sent += bytes;
            entry.touch();
        }
    }

    /// Update bytes received for an entry
    pub fn add_bytes_received(&mut self, key: &NatKey, bytes: u64) {
        if let Some(entry) = self.entries.get_mut(key) {
            entry.bytes_received += bytes;
            entry.touch();
        }
    }

    /// Remove a NAT entry
    pub fn remove(&mut self, key: &NatKey) -> Option<NatEntry> {
        if let Some(entry) = self.entries.remove(key) {
            self.port_to_key.remove(&entry.local_port);
            Some(entry)
        } else {
            None
        }
    }

    /// Clean up expired entries
    pub fn cleanup_expired(&mut self) {
        let tcp_timeout = self.tcp_timeout;
        let udp_timeout = self.udp_timeout;

        let expired_keys: Vec<NatKey> = self
            .entries
            .iter()
            .filter(|(key, entry)| {
                let timeout = if key.is_tcp() { tcp_timeout } else { udp_timeout };
                entry.is_expired(timeout) || entry.state == NatState::Closed
            })
            .map(|(key, _)| *key)
            .collect();

        for key in expired_keys {
            self.remove(&key);
        }
    }

    /// Get the number of active entries
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Check if the NAT table is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get total bytes sent across all connections
    pub fn total_bytes_sent(&self) -> u64 {
        self.entries.values().map(|e| e.bytes_sent).sum()
    }

    /// Get total bytes received across all connections
    pub fn total_bytes_received(&self) -> u64 {
        self.entries.values().map(|e| e.bytes_received).sum()
    }

    /// Get all active connections info
    pub fn get_all_connections(&self) -> Vec<(NatKey, NatEntry)> {
        self.entries
            .iter()
            .map(|(k, v)| (*k, v.clone()))
            .collect()
    }
}

impl Default for NatManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, SocketAddrV4};

    fn make_tcp_key(src_port: u16, dst_port: u16) -> NatKey {
        let src = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(10, 0, 0, 1), src_port));
        let dst = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(8, 8, 8, 8), dst_port));
        NatKey::tcp(src, dst)
    }

    #[test]
    fn test_nat_entry_creation() {
        let src = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(10, 0, 0, 1), 12345));
        let dst = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(8, 8, 8, 8), 443));
        let entry = NatEntry::new(src, dst, 50000);

        assert_eq!(entry.src_addr, src);
        assert_eq!(entry.dst_addr, dst);
        assert_eq!(entry.local_port, 50000);
        assert_eq!(entry.state, NatState::SynSent);
        assert_eq!(entry.bytes_sent, 0);
        assert_eq!(entry.bytes_received, 0);
    }

    #[test]
    fn test_nat_entry_state_transitions() {
        let src = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(10, 0, 0, 1), 12345));
        let dst = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(8, 8, 8, 8), 443));
        let mut entry = NatEntry::new(src, dst, 50000);

        assert_eq!(entry.state, NatState::SynSent);

        entry.establish();
        assert_eq!(entry.state, NatState::Established);

        entry.start_close();
        assert_eq!(entry.state, NatState::FinWait);

        entry.close();
        assert_eq!(entry.state, NatState::Closed);
    }

    #[test]
    fn test_nat_key_creation() {
        let src = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(10, 0, 0, 1), 12345));
        let dst = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(8, 8, 8, 8), 443));

        let tcp_key = NatKey::tcp(src, dst);
        assert!(tcp_key.is_tcp());
        assert!(!tcp_key.is_udp());
        assert_eq!(tcp_key.protocol, 6);

        let udp_key = NatKey::udp(src, dst);
        assert!(udp_key.is_udp());
        assert!(!udp_key.is_tcp());
        assert_eq!(udp_key.protocol, 17);
    }

    #[test]
    fn test_nat_manager_create_entry() {
        let mut manager = NatManager::new();
        let key = make_tcp_key(12345, 443);

        let entry = manager.get_or_create(key).unwrap();
        assert_eq!(entry.state, NatState::SynSent);

        assert_eq!(manager.len(), 1);
    }

    #[test]
    fn test_nat_manager_get_existing() {
        let mut manager = NatManager::new();
        let key = make_tcp_key(12345, 443);

        let port1 = manager.get_or_create(key).unwrap().local_port;
        let port2 = manager.get_or_create(key).unwrap().local_port;

        // Should return the same entry
        assert_eq!(port1, port2);
        assert_eq!(manager.len(), 1);
    }

    #[test]
    fn test_nat_manager_multiple_entries() {
        let mut manager = NatManager::new();

        for i in 0..100 {
            let key = make_tcp_key(10000 + i, 443);
            manager.get_or_create(key).unwrap();
        }

        assert_eq!(manager.len(), 100);
    }

    #[test]
    fn test_nat_manager_remove() {
        let mut manager = NatManager::new();
        let key = make_tcp_key(12345, 443);

        manager.get_or_create(key).unwrap();
        assert_eq!(manager.len(), 1);

        let removed = manager.remove(&key);
        assert!(removed.is_some());
        assert_eq!(manager.len(), 0);
    }

    #[test]
    fn test_nat_manager_bytes_tracking() {
        let mut manager = NatManager::new();
        let key = make_tcp_key(12345, 443);

        manager.get_or_create(key).unwrap();

        manager.add_bytes_sent(&key, 100);
        manager.add_bytes_received(&key, 200);

        let entry = manager.get(&key).unwrap();
        assert_eq!(entry.bytes_sent, 100);
        assert_eq!(entry.bytes_received, 200);

        assert_eq!(manager.total_bytes_sent(), 100);
        assert_eq!(manager.total_bytes_received(), 200);
    }

    #[test]
    fn test_nat_manager_get_by_port() {
        let mut manager = NatManager::new();
        let key = make_tcp_key(12345, 443);

        let local_port = manager.get_or_create(key).unwrap().local_port;

        let entry = manager.get_by_port(local_port);
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().src_addr.port(), 12345);
    }

    #[test]
    fn test_nat_manager_establish() {
        let mut manager = NatManager::new();
        let key = make_tcp_key(12345, 443);

        manager.get_or_create(key).unwrap();
        assert_eq!(manager.get(&key).unwrap().state, NatState::SynSent);

        manager.establish(&key);
        assert_eq!(manager.get(&key).unwrap().state, NatState::Established);
    }
}
