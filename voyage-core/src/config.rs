//! Configuration types for Voyage Core

/// Proxy server configuration
#[derive(Debug, Clone)]
pub struct ProxyConfig {
    pub server_host: String,
    pub server_port: u16,
    pub username: Option<String>,
    pub password: Option<String>,
}

impl ProxyConfig {
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            server_host: host.into(),
            server_port: port,
            username: None,
            password: None,
        }
    }

    pub fn with_auth(mut self, username: impl Into<String>, password: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self.password = Some(password.into());
        self
    }
}

impl Default for ProxyConfig {
    fn default() -> Self {
        Self::new("127.0.0.1", 1080)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_proxy_config_default() {
        let config = ProxyConfig::default();
        assert_eq!(config.server_host, "127.0.0.1");
        assert_eq!(config.server_port, 1080);
        assert!(config.username.is_none());
    }

    #[test]
    fn test_proxy_config_with_auth() {
        let config = ProxyConfig::new("proxy.example.com", 8080)
            .with_auth("user", "pass");
        assert_eq!(config.username, Some("user".to_string()));
        assert_eq!(config.password, Some("pass".to_string()));
    }
}
