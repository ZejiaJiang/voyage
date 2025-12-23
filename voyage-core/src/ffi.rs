//! FFI (Foreign Function Interface) Module
//!
//! This module provides the FFI functions that are exposed to Swift
//! through UniFFI bindings.

use std::net::IpAddr;
use std::sync::{Arc, Mutex, OnceLock};

use crate::config::ProxyConfig;
use crate::error::VoyageError;
use crate::packet::ParsedPacket;
use crate::rule::FfiRouteAction;
use crate::VoyageCore;

/// Global core instance
static CORE_INSTANCE: OnceLock<Arc<Mutex<VoyageCore>>> = OnceLock::new();

/// Core statistics for FFI
#[derive(Debug, Clone, Default)]
pub struct CoreStats {
    /// Bytes sent through the proxy
    pub bytes_sent: u64,
    /// Bytes received through the proxy
    pub bytes_received: u64,
    /// Number of active connections
    pub active_connections: u64,
    /// Total connections since start
    pub total_connections: u64,
}

/// Initialize the voyage core with a proxy configuration
pub fn init_core(
    server_host: String,
    server_port: u16,
    username: Option<String>,
    password: Option<String>,
) -> Result<(), VoyageError> {
    let config = ProxyConfig {
        server_host,
        server_port,
        username,
        password,
    };

    let core = VoyageCore::new(config);
    
    CORE_INSTANCE
        .set(Arc::new(Mutex::new(core)))
        .map_err(|_| VoyageError::AlreadyInitialized)?;

    log::info!("Voyage core initialized");
    Ok(())
}

/// Shutdown the core (note: OnceLock cannot be reset, so this just logs)
pub fn shutdown_core() {
    log::info!("Voyage core shutdown requested");
    // OnceLock cannot be reset, so we just log the shutdown request
    // In a real app, you might set a shutdown flag instead
}

/// Process an inbound packet from the TUN device
pub fn process_inbound_packet(packet: Vec<u8>) -> Result<Vec<u8>, VoyageError> {
    let core = CORE_INSTANCE
        .get()
        .ok_or(VoyageError::NotInitialized)?;

    let mut core = core.lock().map_err(|_| VoyageError::LockError)?;

    // Parse the packet
    let parsed = ParsedPacket::parse(&packet)?;

    // Process through connection manager
    let _conn_info = core.conn_manager.process_packet(&parsed)?;

    // For now, just return the packet as-is
    // In a full implementation, this would involve routing through smoltcp
    Ok(packet)
}

/// Process an outbound packet to send to the TUN device
pub fn process_outbound_packet(packet: Vec<u8>) -> Result<Vec<u8>, VoyageError> {
    let core = CORE_INSTANCE
        .get()
        .ok_or(VoyageError::NotInitialized)?;

    let _core = core.lock().map_err(|_| VoyageError::LockError)?;

    // For now, just return the packet as-is
    Ok(packet)
}

/// Load routing rules from a configuration string
pub fn load_rules(config: String) -> Result<u32, VoyageError> {
    let core = CORE_INSTANCE
        .get()
        .ok_or(VoyageError::NotInitialized)?;

    let mut core = core.lock().map_err(|_| VoyageError::LockError)?;

    let count = core.proxy_manager.load_rules(&config)?;
    log::info!("Loaded {} rules", count);

    Ok(count as u32)
}

/// Evaluate routing decision for a connection
pub fn evaluate_route(
    domain: Option<String>,
    dst_ip: Option<String>,
    dst_port: u16,
    src_port: u16,
) -> Result<FfiRouteAction, VoyageError> {
    let core = CORE_INSTANCE
        .get()
        .ok_or(VoyageError::NotInitialized)?;

    let mut core = core.lock().map_err(|_| VoyageError::LockError)?;

    let ip: Option<IpAddr> = dst_ip
        .as_ref()
        .and_then(|s| s.parse().ok());

    let action = core
        .proxy_manager
        .evaluate_route_ffi(domain.as_deref(), ip, dst_port, src_port);

    Ok(action)
}

/// Get current core statistics
pub fn get_stats() -> Result<CoreStats, VoyageError> {
    let core = CORE_INSTANCE
        .get()
        .ok_or(VoyageError::NotInitialized)?;

    let core = core.lock().map_err(|_| VoyageError::LockError)?;

    let conn_stats = core.conn_manager.get_all_connections();
    let active = conn_stats.len() as u64;

    Ok(CoreStats {
        bytes_sent: core.conn_manager.total_bytes_sent(),
        bytes_received: core.conn_manager.total_bytes_received(),
        active_connections: active,
        total_connections: core.conn_manager.total_connections(),
    })
}

/// Check if the core is initialized
pub fn is_initialized() -> bool {
    CORE_INSTANCE.get().is_some()
}

/// Add bytes sent (for tracking from Swift side)
pub fn add_bytes_sent(bytes: u64) -> Result<(), VoyageError> {
    let core = CORE_INSTANCE
        .get()
        .ok_or(VoyageError::NotInitialized)?;

    let mut core = core.lock().map_err(|_| VoyageError::LockError)?;

    core.proxy_manager.add_proxy_bytes_sent(bytes);
    Ok(())
}

/// Add bytes received (for tracking from Swift side)
pub fn add_bytes_received(bytes: u64) -> Result<(), VoyageError> {
    let core = CORE_INSTANCE
        .get()
        .ok_or(VoyageError::NotInitialized)?;

    let mut core = core.lock().map_err(|_| VoyageError::LockError)?;

    core.proxy_manager.add_proxy_bytes_received(bytes);
    Ok(())
}

/// Clear all routing rules
pub fn clear_rules() -> Result<(), VoyageError> {
    let core = CORE_INSTANCE
        .get()
        .ok_or(VoyageError::NotInitialized)?;

    let mut core = core.lock().map_err(|_| VoyageError::LockError)?;

    core.proxy_manager.clear_rules();
    log::info!("Cleared all rules");
    Ok(())
}

/// Get the number of loaded rules
pub fn rule_count() -> Result<u32, VoyageError> {
    let core = CORE_INSTANCE
        .get()
        .ok_or(VoyageError::NotInitialized)?;

    let core = core.lock().map_err(|_| VoyageError::LockError)?;

    Ok(core.proxy_manager.rule_count() as u32)
}

/// Enable the proxy
pub fn enable_proxy() -> Result<(), VoyageError> {
    let core = CORE_INSTANCE
        .get()
        .ok_or(VoyageError::NotInitialized)?;

    let mut core = core.lock().map_err(|_| VoyageError::LockError)?;

    core.proxy_manager.enable();
    log::info!("Proxy enabled");
    Ok(())
}

/// Disable the proxy
pub fn disable_proxy() -> Result<(), VoyageError> {
    let core = CORE_INSTANCE
        .get()
        .ok_or(VoyageError::NotInitialized)?;

    let mut core = core.lock().map_err(|_| VoyageError::LockError)?;

    core.proxy_manager.disable();
    log::info!("Proxy disabled");
    Ok(())
}

/// Check if proxy is enabled
pub fn is_proxy_enabled() -> Result<bool, VoyageError> {
    let core = CORE_INSTANCE
        .get()
        .ok_or(VoyageError::NotInitialized)?;

    let core = core.lock().map_err(|_| VoyageError::LockError)?;

    Ok(core.proxy_manager.is_enabled())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests use serial_test because they share global state
    // In a real test environment, you would want to reset the global state

    #[test]
    fn test_core_stats_default() {
        let stats = CoreStats::default();
        assert_eq!(stats.bytes_sent, 0);
        assert_eq!(stats.bytes_received, 0);
        assert_eq!(stats.active_connections, 0);
        assert_eq!(stats.total_connections, 0);
    }

    #[test]
    fn test_ffi_route_action_values() {
        assert_eq!(FfiRouteAction::Direct as u8, 0);
        assert_eq!(FfiRouteAction::Proxy as u8, 1);
        assert_eq!(FfiRouteAction::Reject as u8, 2);
    }

    // Integration tests would need special handling for the global state
    // See tests/integration_test.rs for proper integration testing
}
