//! Connection Manager
//!
//! This module provides the connection management layer that integrates
//! the NAT manager with smoltcp interface to handle TCP/UDP connections.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use smoltcp::iface::{SocketHandle, SocketSet};
use smoltcp::socket::tcp::{Socket as TcpSocket, State as TcpState};
use tokio::sync::Mutex;

use crate::error::VoyageError;
use crate::nat::{NatKey, NatManager, NatState};
use crate::packet::ParsedPacket;

/// Connection state combining NAT and socket state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Connection being established
    Connecting,
    /// Connection established and active
    Established,
    /// Connection closing
    Closing,
    /// Connection closed
    Closed,
}

impl From<NatState> for ConnectionState {
    fn from(state: NatState) -> Self {
        match state {
            NatState::SynSent => ConnectionState::Connecting,
            NatState::Established => ConnectionState::Established,
            NatState::FinWait | NatState::Closing => ConnectionState::Closing,
            NatState::Closed => ConnectionState::Closed,
        }
    }
}

/// Information about an active connection
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    /// NAT key for this connection
    pub key: NatKey,
    /// Local port allocated by NAT
    pub local_port: u16,
    /// smoltcp socket handle (if any)
    pub socket_handle: Option<SocketHandle>,
    /// Connection state
    pub state: ConnectionState,
    /// Bytes sent
    pub bytes_sent: u64,
    /// Bytes received
    pub bytes_received: u64,
    /// Time connection was created
    pub created_at: Instant,
}

/// Manages the mapping between app connections and proxy connections
pub struct ConnectionManager {
    /// NAT manager for connection tracking
    nat: NatManager,
    /// Map from NAT key to smoltcp socket handle
    socket_handles: HashMap<NatKey, SocketHandle>,
    /// Map from socket handle to NAT key (reverse lookup)
    handle_to_key: HashMap<SocketHandle, NatKey>,
    /// Total bytes sent
    total_bytes_sent: u64,
    /// Total bytes received
    total_bytes_received: u64,
    /// Total connections created
    total_connections: u64,
}

impl ConnectionManager {
    /// Create a new connection manager
    pub fn new() -> Self {
        Self {
            nat: NatManager::new(),
            socket_handles: HashMap::new(),
            handle_to_key: HashMap::new(),
            total_bytes_sent: 0,
            total_bytes_received: 0,
            total_connections: 0,
        }
    }

    /// Process an incoming packet and get or create a connection
    pub fn process_packet(&mut self, packet: &ParsedPacket) -> Result<ConnectionInfo, VoyageError> {
        let key = packet
            .to_nat_key()
            .ok_or_else(|| VoyageError::InvalidPacket("Cannot create NAT key".into()))?;

        // Get or create NAT entry
        let entry = self.nat.get_or_create(key)?;
        let local_port = entry.local_port;

        // Track new connections
        if entry.state == NatState::SynSent && packet.is_tcp_syn() {
            self.total_connections += 1;
        }

        // Get socket handle if exists
        let socket_handle = self.socket_handles.get(&key).copied();

        Ok(ConnectionInfo {
            key,
            local_port,
            socket_handle,
            state: entry.state.into(),
            bytes_sent: entry.bytes_sent,
            bytes_received: entry.bytes_received,
            created_at: Instant::now(), // Approximate
        })
    }

    /// Register a socket handle for a connection
    pub fn register_socket(&mut self, key: NatKey, handle: SocketHandle) {
        self.socket_handles.insert(key, handle);
        self.handle_to_key.insert(handle, key);
    }

    /// Get the socket handle for a connection
    pub fn get_socket_handle(&self, key: &NatKey) -> Option<SocketHandle> {
        self.socket_handles.get(key).copied()
    }

    /// Get the NAT key for a socket handle
    pub fn get_key_for_handle(&self, handle: SocketHandle) -> Option<&NatKey> {
        self.handle_to_key.get(&handle)
    }

    /// Get connection info by local port
    pub fn get_by_port(&self, port: u16) -> Option<ConnectionInfo> {
        let key = self.nat.get_key_by_port(port)?;
        let entry = self.nat.get(key)?;

        Some(ConnectionInfo {
            key: *key,
            local_port: entry.local_port,
            socket_handle: self.socket_handles.get(key).copied(),
            state: entry.state.into(),
            bytes_sent: entry.bytes_sent,
            bytes_received: entry.bytes_received,
            created_at: Instant::now(),
        })
    }

    /// Mark a connection as established
    pub fn establish(&mut self, key: &NatKey) {
        self.nat.establish(key);
    }

    /// Add bytes sent to a connection
    pub fn add_bytes_sent(&mut self, key: &NatKey, bytes: u64) {
        self.nat.add_bytes_sent(key, bytes);
        self.total_bytes_sent += bytes;
    }

    /// Add bytes received to a connection
    pub fn add_bytes_received(&mut self, key: &NatKey, bytes: u64) {
        self.nat.add_bytes_received(key, bytes);
        self.total_bytes_received += bytes;
    }

    /// Close a connection
    pub fn close_connection(&mut self, key: &NatKey) {
        if let Some(entry) = self.nat.get_mut(key) {
            entry.close();
        }
    }

    /// Remove a connection completely
    pub fn remove_connection(&mut self, key: &NatKey) -> Option<ConnectionInfo> {
        let entry = self.nat.remove(key)?;

        if let Some(handle) = self.socket_handles.remove(key) {
            self.handle_to_key.remove(&handle);
        }

        Some(ConnectionInfo {
            key: *key,
            local_port: entry.local_port,
            socket_handle: None,
            state: entry.state.into(),
            bytes_sent: entry.bytes_sent,
            bytes_received: entry.bytes_received,
            created_at: Instant::now(),
        })
    }

    /// Clean up expired and closed connections
    pub fn cleanup(&mut self) {
        // First, collect keys to remove
        let keys_to_remove: Vec<NatKey> = self
            .nat
            .get_all_connections()
            .iter()
            .filter(|(_, entry)| entry.state == NatState::Closed)
            .map(|(key, _)| *key)
            .collect();

        for key in keys_to_remove {
            self.remove_connection(&key);
        }

        // Then cleanup NAT table
        self.nat.cleanup_expired();
    }

    /// Get the number of active connections
    pub fn active_connections(&self) -> usize {
        self.nat.len()
    }

    /// Get total bytes sent
    pub fn total_bytes_sent(&self) -> u64 {
        self.total_bytes_sent
    }

    /// Get total bytes received
    pub fn total_bytes_received(&self) -> u64 {
        self.total_bytes_received
    }

    /// Get total connections created
    pub fn total_connections(&self) -> u64 {
        self.total_connections
    }

    /// Get all active connections
    pub fn get_all_connections(&self) -> Vec<ConnectionInfo> {
        self.nat
            .get_all_connections()
            .iter()
            .map(|(key, entry)| ConnectionInfo {
                key: *key,
                local_port: entry.local_port,
                socket_handle: self.socket_handles.get(key).copied(),
                state: entry.state.into(),
                bytes_sent: entry.bytes_sent,
                bytes_received: entry.bytes_received,
                created_at: Instant::now(),
            })
            .collect()
    }

    /// Synchronize connection states with smoltcp socket states
    pub fn sync_socket_states(&mut self, sockets: &SocketSet<'_>) {
        for (key, handle) in &self.socket_handles {
            let socket = sockets.get::<TcpSocket>(*handle);
            let new_state = match socket.state() {
                TcpState::Established => NatState::Established,
                TcpState::FinWait1 | TcpState::FinWait2 | TcpState::Closing | TcpState::TimeWait => {
                    NatState::FinWait
                }
                TcpState::Closed | TcpState::CloseWait | TcpState::LastAck => NatState::Closed,
                _ => continue,
            };

            if let Some(entry) = self.nat.get_mut(key) {
                if entry.state != new_state {
                    match new_state {
                        NatState::Established => entry.establish(),
                        NatState::FinWait => entry.start_close(),
                        NatState::Closed => entry.close(),
                        _ => {}
                    }
                }
            }
        }
    }
}

impl Default for ConnectionManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe wrapper for ConnectionManager
pub type SharedConnectionManager = Arc<Mutex<ConnectionManager>>;

/// Create a new shared connection manager
pub fn new_shared_connection_manager() -> SharedConnectionManager {
    Arc::new(Mutex::new(ConnectionManager::new()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
    use smoltcp::iface::SocketHandle;

    fn make_tcp_key(src_port: u16, dst_port: u16) -> NatKey {
        let src = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(10, 0, 0, 1), src_port));
        let dst = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::new(8, 8, 8, 8), dst_port));
        NatKey::tcp(src, dst)
    }

    // Helper to create a mock socket handle for testing
    fn mock_socket_handle(id: usize) -> SocketHandle {
        // Create a minimal SocketSet and add a dummy socket to get a real handle
        // For unit tests, we'll skip handle-based tests if we can't create them
        // This is a limitation of smoltcp's API
        unsafe { std::mem::transmute::<usize, SocketHandle>(id) }
    }

    #[test]
    fn test_connection_manager_new() {
        let manager = ConnectionManager::new();
        assert_eq!(manager.active_connections(), 0);
        assert_eq!(manager.total_connections(), 0);
        assert_eq!(manager.total_bytes_sent(), 0);
        assert_eq!(manager.total_bytes_received(), 0);
    }

    #[test]
    fn test_register_socket() {
        let mut manager = ConnectionManager::new();
        let key = make_tcp_key(12345, 443);
        let handle = mock_socket_handle(1);

        manager.register_socket(key, handle);

        assert_eq!(manager.get_socket_handle(&key), Some(handle));
        assert_eq!(manager.get_key_for_handle(handle), Some(&key));
    }

    #[test]
    fn test_connection_state_transition() {
        let mut manager = ConnectionManager::new();
        let key = make_tcp_key(12345, 443);

        // Create entry through NAT
        manager.nat.get_or_create(key).unwrap();

        // Establish
        manager.establish(&key);
        let conn = manager.get_all_connections().into_iter().find(|c| c.key == key).unwrap();
        assert_eq!(conn.state, ConnectionState::Established);

        // Close
        manager.close_connection(&key);
        let conn = manager.get_all_connections().into_iter().find(|c| c.key == key).unwrap();
        assert_eq!(conn.state, ConnectionState::Closed);
    }

    #[test]
    fn test_bytes_tracking() {
        let mut manager = ConnectionManager::new();
        let key = make_tcp_key(12345, 443);

        manager.nat.get_or_create(key).unwrap();

        manager.add_bytes_sent(&key, 100);
        manager.add_bytes_received(&key, 200);

        assert_eq!(manager.total_bytes_sent(), 100);
        assert_eq!(manager.total_bytes_received(), 200);
    }

    #[test]
    fn test_remove_connection() {
        let mut manager = ConnectionManager::new();
        let key = make_tcp_key(12345, 443);
        let handle = mock_socket_handle(1);

        manager.nat.get_or_create(key).unwrap();
        manager.register_socket(key, handle);

        assert_eq!(manager.active_connections(), 1);

        let removed = manager.remove_connection(&key);
        assert!(removed.is_some());
        assert_eq!(manager.active_connections(), 0);
        assert_eq!(manager.get_socket_handle(&key), None);
    }

    #[test]
    fn test_get_by_port() {
        let mut manager = ConnectionManager::new();
        let key = make_tcp_key(12345, 443);

        let entry = manager.nat.get_or_create(key).unwrap();
        let local_port = entry.local_port;

        let conn = manager.get_by_port(local_port);
        assert!(conn.is_some());
        assert_eq!(conn.unwrap().key, key);
    }

    #[test]
    fn test_cleanup() {
        let mut manager = ConnectionManager::new();

        for i in 0..10 {
            let key = make_tcp_key(10000 + i, 443);
            manager.nat.get_or_create(key).unwrap();
            if i % 2 == 0 {
                manager.close_connection(&key);
            }
        }

        assert_eq!(manager.active_connections(), 10);

        manager.cleanup();

        // Closed connections should be removed
        assert_eq!(manager.active_connections(), 5);
    }

    #[test]
    fn test_get_all_connections() {
        let mut manager = ConnectionManager::new();

        for i in 0..5 {
            let key = make_tcp_key(10000 + i, 443);
            manager.nat.get_or_create(key).unwrap();
        }

        let connections = manager.get_all_connections();
        assert_eq!(connections.len(), 5);
    }

    #[test]
    fn test_connection_state_from_nat_state() {
        assert_eq!(
            ConnectionState::from(NatState::SynSent),
            ConnectionState::Connecting
        );
        assert_eq!(
            ConnectionState::from(NatState::Established),
            ConnectionState::Established
        );
        assert_eq!(
            ConnectionState::from(NatState::FinWait),
            ConnectionState::Closing
        );
        assert_eq!(
            ConnectionState::from(NatState::Closing),
            ConnectionState::Closing
        );
        assert_eq!(
            ConnectionState::from(NatState::Closed),
            ConnectionState::Closed
        );
    }

    #[test]
    fn test_shared_connection_manager() {
        let shared = new_shared_connection_manager();
        // Just verify it compiles and creates
        assert!(Arc::strong_count(&shared) == 1);
    }
}
