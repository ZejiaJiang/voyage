//! Proxy Manager
//!
//! This module provides the proxy management layer that coordinates
//! routing decisions and proxy connections.

use std::net::IpAddr;
use std::sync::Arc;

use tokio::sync::Mutex;

use crate::config::ProxyConfig;
use crate::error::VoyageError;
use crate::rule::{FfiRouteAction, RouteAction, RuleEngine};

/// Connection routing decision with metadata
#[derive(Debug, Clone)]
pub struct RoutingDecision {
    /// The routing action to take
    pub action: RouteAction,
    /// Domain name if resolved
    pub domain: Option<String>,
    /// Destination IP
    pub dst_ip: Option<IpAddr>,
    /// Destination port
    pub dst_port: u16,
    /// Rule that matched (if any)
    pub matched_rule: Option<String>,
}

impl RoutingDecision {
    /// Create a new direct routing decision
    pub fn direct(dst_port: u16) -> Self {
        Self {
            action: RouteAction::Direct,
            domain: None,
            dst_ip: None,
            dst_port,
            matched_rule: None,
        }
    }

    /// Create a new proxy routing decision
    pub fn proxy(dst_port: u16) -> Self {
        Self {
            action: RouteAction::Proxy,
            domain: None,
            dst_ip: None,
            dst_port,
            matched_rule: None,
        }
    }

    /// Create a new reject routing decision
    pub fn reject(dst_port: u16) -> Self {
        Self {
            action: RouteAction::Reject,
            domain: None,
            dst_ip: None,
            dst_port,
            matched_rule: None,
        }
    }

    /// Set domain
    pub fn with_domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = Some(domain.into());
        self
    }

    /// Set destination IP
    pub fn with_dst_ip(mut self, ip: IpAddr) -> Self {
        self.dst_ip = Some(ip);
        self
    }

    /// Set matched rule name
    pub fn with_rule(mut self, rule: impl Into<String>) -> Self {
        self.matched_rule = Some(rule.into());
        self
    }
}

/// Proxy statistics
#[derive(Debug, Clone, Default)]
pub struct ProxyStats {
    /// Total direct connections
    pub direct_connections: u64,
    /// Total proxied connections
    pub proxied_connections: u64,
    /// Total rejected connections
    pub rejected_connections: u64,
    /// Total bytes sent through proxy
    pub proxy_bytes_sent: u64,
    /// Total bytes received through proxy
    pub proxy_bytes_received: u64,
}

/// Manages proxy configurations and routing decisions
pub struct ProxyManager {
    /// Proxy configuration
    config: Option<ProxyConfig>,
    /// Rule engine for routing decisions
    rule_engine: RuleEngine,
    /// Statistics
    stats: ProxyStats,
    /// Whether proxy is enabled
    enabled: bool,
}

impl ProxyManager {
    /// Create a new proxy manager
    pub fn new() -> Self {
        Self {
            config: None,
            rule_engine: RuleEngine::new(),
            stats: ProxyStats::default(),
            enabled: false,
        }
    }

    /// Create a new proxy manager with configuration
    pub fn with_config(config: ProxyConfig) -> Self {
        Self {
            config: Some(config),
            rule_engine: RuleEngine::new(),
            stats: ProxyStats::default(),
            enabled: true,
        }
    }

    /// Set the proxy configuration
    pub fn set_config(&mut self, config: ProxyConfig) {
        self.config = Some(config);
    }

    /// Get the proxy configuration
    pub fn get_config(&self) -> Option<&ProxyConfig> {
        self.config.as_ref()
    }

    /// Enable the proxy
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable the proxy
    pub fn disable(&mut self) {
        self.enabled = false;
    }

    /// Check if proxy is enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled && self.config.is_some()
    }

    /// Load rules from configuration string
    pub fn load_rules(&mut self, config: &str) -> Result<usize, VoyageError> {
        self.rule_engine
            .load_from_config(config)
            .map_err(|e| VoyageError::ConfigError(e))
    }

    /// Clear all rules
    pub fn clear_rules(&mut self) {
        self.rule_engine.clear();
    }

    /// Get the number of rules
    pub fn rule_count(&self) -> usize {
        self.rule_engine.len()
    }

    /// Evaluate routing for a connection
    pub fn evaluate_route(
        &mut self,
        domain: Option<&str>,
        dst_ip: Option<IpAddr>,
        dst_port: u16,
        src_port: u16,
    ) -> RoutingDecision {
        let action = if self.is_enabled() {
            self.rule_engine.evaluate(domain, dst_ip, dst_port, src_port)
        } else {
            RouteAction::Direct
        };

        // Update stats
        match &action {
            RouteAction::Direct => self.stats.direct_connections += 1,
            RouteAction::Proxy => self.stats.proxied_connections += 1,
            RouteAction::Reject => self.stats.rejected_connections += 1,
        }

        let decision = RoutingDecision {
            action,
            domain: domain.map(String::from),
            dst_ip,
            dst_port,
            matched_rule: None,
        };

        decision
    }

    /// Get FFI-friendly route action
    pub fn evaluate_route_ffi(
        &mut self,
        domain: Option<&str>,
        dst_ip: Option<IpAddr>,
        dst_port: u16,
        src_port: u16,
    ) -> FfiRouteAction {
        let decision = self.evaluate_route(domain, dst_ip, dst_port, src_port);
        FfiRouteAction::from(decision.action)
    }

    /// Add bytes sent through proxy
    pub fn add_proxy_bytes_sent(&mut self, bytes: u64) {
        self.stats.proxy_bytes_sent += bytes;
    }

    /// Add bytes received through proxy
    pub fn add_proxy_bytes_received(&mut self, bytes: u64) {
        self.stats.proxy_bytes_received += bytes;
    }

    /// Get statistics
    pub fn get_stats(&self) -> &ProxyStats {
        &self.stats
    }

    /// Reset statistics
    pub fn reset_stats(&mut self) {
        self.stats = ProxyStats::default();
    }

    /// Get proxy server address
    pub fn get_proxy_addr(&self) -> Option<(String, u16)> {
        self.config.as_ref().map(|c| (c.server_host.clone(), c.server_port))
    }

    /// Get proxy credentials
    pub fn get_credentials(&self) -> Option<(String, String)> {
        self.config.as_ref().and_then(|c| {
            match (&c.username, &c.password) {
                (Some(u), Some(p)) => Some((u.clone(), p.clone())),
                _ => None,
            }
        })
    }
}

impl Default for ProxyManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Thread-safe wrapper for ProxyManager
pub type SharedProxyManager = Arc<Mutex<ProxyManager>>;

/// Create a new shared proxy manager
pub fn new_shared_proxy_manager() -> SharedProxyManager {
    Arc::new(Mutex::new(ProxyManager::new()))
}

/// Create a new shared proxy manager with configuration
pub fn new_shared_proxy_manager_with_config(config: ProxyConfig) -> SharedProxyManager {
    Arc::new(Mutex::new(ProxyManager::with_config(config)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_manager_new() {
        let manager = ProxyManager::new();
        assert!(!manager.is_enabled());
        assert!(manager.get_config().is_none());
        assert_eq!(manager.rule_count(), 0);
    }

    #[test]
    fn test_proxy_manager_with_config() {
        let config = ProxyConfig {
            server_host: "proxy.example.com".into(),
            server_port: 1080,
            username: Some("user".into()),
            password: Some("pass".into()),
        };

        let manager = ProxyManager::with_config(config.clone());
        assert!(manager.is_enabled());
        assert!(manager.get_config().is_some());
        assert_eq!(manager.get_config().unwrap().server_host, "proxy.example.com");
    }

    #[test]
    fn test_enable_disable() {
        let mut manager = ProxyManager::new();
        manager.set_config(ProxyConfig {
            server_host: "proxy.example.com".into(),
            server_port: 1080,
            username: None,
            password: None,
        });

        manager.enable();
        assert!(manager.is_enabled());

        manager.disable();
        assert!(!manager.is_enabled());
    }

    #[test]
    fn test_load_rules() {
        let mut manager = ProxyManager::new();
        let config = r#"
DOMAIN-SUFFIX, .google.com, PROXY
FINAL, DIRECT
"#;

        let count = manager.load_rules(config).unwrap();
        assert_eq!(count, 2);
        assert_eq!(manager.rule_count(), 2);
    }

    #[test]
    fn test_evaluate_route_disabled() {
        let mut manager = ProxyManager::new();
        // Manager is disabled, should return Direct

        let decision = manager.evaluate_route(Some("www.google.com"), None, 443, 0);
        assert_eq!(decision.action, RouteAction::Direct);
    }

    #[test]
    fn test_evaluate_route_with_rules() {
        let mut manager = ProxyManager::with_config(ProxyConfig {
            server_host: "proxy.example.com".into(),
            server_port: 1080,
            username: None,
            password: None,
        });

        manager
            .load_rules(
                r#"
DOMAIN-SUFFIX, .google.com, PROXY
DOMAIN, blocked.com, REJECT
FINAL, DIRECT
"#,
            )
            .unwrap();

        // Should match PROXY
        let decision = manager.evaluate_route(Some("www.google.com"), None, 443, 0);
        assert_eq!(decision.action, RouteAction::Proxy);

        // Should match REJECT
        let decision = manager.evaluate_route(Some("blocked.com"), None, 443, 0);
        assert_eq!(decision.action, RouteAction::Reject);

        // Should match DIRECT (FINAL)
        let decision = manager.evaluate_route(Some("example.com"), None, 443, 0);
        assert_eq!(decision.action, RouteAction::Direct);
    }

    #[test]
    fn test_stats_tracking() {
        let mut manager = ProxyManager::with_config(ProxyConfig {
            server_host: "proxy.example.com".into(),
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

        manager.evaluate_route(Some("proxy.com"), None, 443, 0);
        manager.evaluate_route(Some("reject.com"), None, 443, 0);
        manager.evaluate_route(Some("other.com"), None, 443, 0);
        manager.evaluate_route(Some("another.com"), None, 443, 0);

        let stats = manager.get_stats();
        assert_eq!(stats.proxied_connections, 1);
        assert_eq!(stats.rejected_connections, 1);
        assert_eq!(stats.direct_connections, 2);
    }

    #[test]
    fn test_proxy_bytes_tracking() {
        let mut manager = ProxyManager::new();

        manager.add_proxy_bytes_sent(100);
        manager.add_proxy_bytes_received(200);

        let stats = manager.get_stats();
        assert_eq!(stats.proxy_bytes_sent, 100);
        assert_eq!(stats.proxy_bytes_received, 200);
    }

    #[test]
    fn test_reset_stats() {
        let mut manager = ProxyManager::new();
        manager.add_proxy_bytes_sent(100);

        manager.reset_stats();

        let stats = manager.get_stats();
        assert_eq!(stats.proxy_bytes_sent, 0);
    }

    #[test]
    fn test_get_proxy_addr() {
        let manager = ProxyManager::with_config(ProxyConfig {
            server_host: "proxy.example.com".into(),
            server_port: 1080,
            username: None,
            password: None,
        });

        let addr = manager.get_proxy_addr().unwrap();
        assert_eq!(addr, ("proxy.example.com".to_string(), 1080));
    }

    #[test]
    fn test_get_credentials() {
        let manager = ProxyManager::with_config(ProxyConfig {
            server_host: "proxy.example.com".into(),
            server_port: 1080,
            username: Some("user".into()),
            password: Some("pass".into()),
        });

        let creds = manager.get_credentials().unwrap();
        assert_eq!(creds, ("user".to_string(), "pass".to_string()));
    }

    #[test]
    fn test_get_credentials_none() {
        let manager = ProxyManager::with_config(ProxyConfig {
            server_host: "proxy.example.com".into(),
            server_port: 1080,
            username: None,
            password: None,
        });

        assert!(manager.get_credentials().is_none());
    }

    #[test]
    fn test_routing_decision_builders() {
        let decision = RoutingDecision::direct(443)
            .with_domain("example.com")
            .with_dst_ip(IpAddr::V4(std::net::Ipv4Addr::new(1, 2, 3, 4)))
            .with_rule("test rule");

        assert_eq!(decision.action, RouteAction::Direct);
        assert_eq!(decision.domain, Some("example.com".to_string()));
        assert_eq!(decision.dst_port, 443);
        assert_eq!(decision.matched_rule, Some("test rule".to_string()));
    }

    #[test]
    fn test_clear_rules() {
        let mut manager = ProxyManager::new();
        manager.load_rules("FINAL, DIRECT").unwrap();
        assert_eq!(manager.rule_count(), 1);

        manager.clear_rules();
        assert_eq!(manager.rule_count(), 0);
    }

    #[test]
    fn test_shared_proxy_manager() {
        let shared = new_shared_proxy_manager();
        assert!(Arc::strong_count(&shared) == 1);

        let config = ProxyConfig {
            server_host: "proxy.example.com".into(),
            server_port: 1080,
            username: None,
            password: None,
        };
        let shared_with_config = new_shared_proxy_manager_with_config(config);
        assert!(Arc::strong_count(&shared_with_config) == 1);
    }
}
