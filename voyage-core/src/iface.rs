//! Network interface manager for smoltcp

use crate::device::VirtualTunDevice;
use smoltcp::iface::{Config, Interface, SocketHandle, SocketSet};
use smoltcp::socket::tcp::{Socket as TcpSocket, SocketBuffer as TcpSocketBuffer, State as TcpState};
use smoltcp::time::Instant;
use smoltcp::wire::{HardwareAddress, IpAddress, IpCidr};
use std::collections::HashMap;
use std::time::SystemTime;

/// Buffer size for TCP sockets
const TCP_RX_BUFFER_SIZE: usize = 65536;
const TCP_TX_BUFFER_SIZE: usize = 65536;

/// Get current time as smoltcp Instant
fn smoltcp_now() -> Instant {
    let duration = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    Instant::from_millis(duration.as_millis() as i64)
}

/// Connection info for debugging
#[derive(Debug, Clone)]
pub struct IfaceConnectionInfo {
    pub handle: SocketHandle,
    pub state: String,
}

/// Manages the smoltcp network interface
pub struct InterfaceManager {
    device: VirtualTunDevice,
    iface: Interface,
    sockets: SocketSet<'static>,
    socket_map: HashMap<SocketHandle, IfaceConnectionInfo>,
    next_local_port: u16,
}

impl InterfaceManager {
    pub fn new() -> Self {
        let mut device = VirtualTunDevice::new();

        let config = Config::new(HardwareAddress::Ip);
        let mut iface = Interface::new(config, &mut device, smoltcp_now());

        // Configure interface with a private IP range
        iface.update_ip_addrs(|addrs| {
            let _ = addrs.push(IpCidr::new(IpAddress::v4(10, 0, 0, 1), 24));
        });

        let sockets = SocketSet::new(vec![]);

        Self {
            device,
            iface,
            sockets,
            socket_map: HashMap::new(),
            next_local_port: 49152,
        }
    }

    pub fn inject_packet(&mut self, packet: Vec<u8>) {
        self.device.inject_packet(packet);
    }

    pub fn take_packets(&mut self) -> Vec<Vec<u8>> {
        self.device.take_packets()
    }

    pub fn poll(&mut self) -> bool {
        self.iface.poll(smoltcp_now(), &mut self.device, &mut self.sockets)
    }

    pub fn create_tcp_socket(&mut self) -> SocketHandle {
        let rx_buffer = TcpSocketBuffer::new(vec![0u8; TCP_RX_BUFFER_SIZE]);
        let tx_buffer = TcpSocketBuffer::new(vec![0u8; TCP_TX_BUFFER_SIZE]);
        let socket = TcpSocket::new(rx_buffer, tx_buffer);
        self.sockets.add(socket)
    }

    pub fn get_tcp_socket(&mut self, handle: SocketHandle) -> &mut TcpSocket<'static> {
        self.sockets.get_mut::<TcpSocket>(handle)
    }

    pub fn remove_socket(&mut self, handle: SocketHandle) {
        self.socket_map.remove(&handle);
        self.sockets.remove(handle);
    }

    pub fn allocate_local_port(&mut self) -> u16 {
        let port = self.next_local_port;
        self.next_local_port = self.next_local_port.wrapping_add(1);
        if self.next_local_port < 49152 {
            self.next_local_port = 49152;
        }
        port
    }

    pub fn socket_count(&self) -> usize {
        self.sockets.iter().count()
    }

    pub fn cleanup_closed_sockets(&mut self) {
        let mut to_remove = Vec::new();

        for (handle, _) in self.socket_map.iter() {
            let socket = self.sockets.get::<TcpSocket>(*handle);
            if socket.state() == TcpState::Closed {
                to_remove.push(*handle);
            }
        }

        for handle in to_remove {
            self.remove_socket(handle);
        }
    }
}

impl Default for InterfaceManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interface_manager_creation() {
        let manager = InterfaceManager::new();
        assert_eq!(manager.socket_count(), 0);
    }

    #[test]
    fn test_create_tcp_socket() {
        let mut manager = InterfaceManager::new();
        let handle = manager.create_tcp_socket();
        assert_eq!(manager.socket_count(), 1);
        manager.remove_socket(handle);
        assert_eq!(manager.socket_count(), 0);
    }

    #[test]
    fn test_port_allocation() {
        let mut manager = InterfaceManager::new();
        let port1 = manager.allocate_local_port();
        let port2 = manager.allocate_local_port();
        assert!(port1 >= 49152);
        assert_eq!(port2, port1 + 1);
    }

    #[test]
    fn test_packet_injection() {
        let mut manager = InterfaceManager::new();
        manager.inject_packet(vec![1, 2, 3, 4]);
        assert!(manager.device.has_rx_packets());
    }

    #[test]
    fn test_poll() {
        let mut manager = InterfaceManager::new();
        let _ = manager.poll(); // Should not panic
    }
}
