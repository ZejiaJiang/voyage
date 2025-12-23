//! Virtual TUN device for smoltcp

use smoltcp::phy::{Device, DeviceCapabilities, Medium, RxToken, TxToken};
use smoltcp::time::Instant;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// Maximum Transmission Unit
pub const MTU: usize = 1500;

/// Thread-safe packet queue
pub type PacketQueue = Arc<Mutex<VecDeque<Vec<u8>>>>;

/// Virtual TUN device that interfaces with smoltcp
pub struct VirtualTunDevice {
    rx_queue: PacketQueue,
    tx_queue: PacketQueue,
    mtu: usize,
}

impl VirtualTunDevice {
    pub fn new() -> Self {
        Self {
            rx_queue: Arc::new(Mutex::new(VecDeque::new())),
            tx_queue: Arc::new(Mutex::new(VecDeque::new())),
            mtu: MTU,
        }
    }

    pub fn with_mtu(mut self, mtu: usize) -> Self {
        self.mtu = mtu;
        self
    }

    pub fn rx_queue(&self) -> PacketQueue {
        Arc::clone(&self.rx_queue)
    }

    pub fn tx_queue(&self) -> PacketQueue {
        Arc::clone(&self.tx_queue)
    }

    pub fn inject_packet(&self, packet: Vec<u8>) {
        if let Ok(mut queue) = self.rx_queue.lock() {
            queue.push_back(packet);
        }
    }

    pub fn take_packets(&self) -> Vec<Vec<u8>> {
        if let Ok(mut queue) = self.tx_queue.lock() {
            queue.drain(..).collect()
        } else {
            Vec::new()
        }
    }

    pub fn has_rx_packets(&self) -> bool {
        self.rx_queue.lock().map(|q| !q.is_empty()).unwrap_or(false)
    }

    pub fn pending_tx_count(&self) -> usize {
        self.tx_queue.lock().map(|q| q.len()).unwrap_or(0)
    }
}

impl Default for VirtualTunDevice {
    fn default() -> Self {
        Self::new()
    }
}

impl Device for VirtualTunDevice {
    type RxToken<'a> = VirtualRxToken where Self: 'a;
    type TxToken<'a> = VirtualTxToken where Self: 'a;

    fn capabilities(&self) -> DeviceCapabilities {
        let mut caps = DeviceCapabilities::default();
        caps.medium = Medium::Ip;
        caps.max_transmission_unit = self.mtu;
        caps
    }

    fn receive(&mut self, _timestamp: Instant) -> Option<(Self::RxToken<'_>, Self::TxToken<'_>)> {
        let packet = self.rx_queue.lock().ok()?.pop_front()?;
        
        Some((
            VirtualRxToken { packet },
            VirtualTxToken { queue: Arc::clone(&self.tx_queue) },
        ))
    }

    fn transmit(&mut self, _timestamp: Instant) -> Option<Self::TxToken<'_>> {
        Some(VirtualTxToken { queue: Arc::clone(&self.tx_queue) })
    }
}

pub struct VirtualRxToken {
    packet: Vec<u8>,
}

impl RxToken for VirtualRxToken {
    fn consume<R, F>(self, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        let mut packet = self.packet;
        f(&mut packet)
    }
}

pub struct VirtualTxToken {
    queue: PacketQueue,
}

impl TxToken for VirtualTxToken {
    fn consume<R, F>(self, len: usize, f: F) -> R
    where
        F: FnOnce(&mut [u8]) -> R,
    {
        let mut buffer = vec![0u8; len];
        let result = f(&mut buffer);
        
        if let Ok(mut queue) = self.queue.lock() {
            queue.push_back(buffer);
        }
        
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_device_creation() {
        let device = VirtualTunDevice::new();
        assert_eq!(device.mtu, MTU);
        assert!(!device.has_rx_packets());
    }

    #[test]
    fn test_packet_injection() {
        let device = VirtualTunDevice::new();
        device.inject_packet(vec![1, 2, 3, 4]);
        assert!(device.has_rx_packets());
    }

    #[test]
    fn test_capabilities() {
        let device = VirtualTunDevice::new();
        let caps = device.capabilities();
        assert_eq!(caps.medium, Medium::Ip);
        assert_eq!(caps.max_transmission_unit, MTU);
    }

    #[test]
    fn test_custom_mtu() {
        let device = VirtualTunDevice::new().with_mtu(9000);
        assert_eq!(device.mtu, 9000);
    }
}
