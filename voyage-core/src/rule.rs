//! Rule Engine
//!
//! This module provides a Surge-style rule engine for routing decisions.
//! Rules are evaluated in order, and the first matching rule determines the action.

use std::net::{IpAddr, Ipv4Addr};
use std::str::FromStr;

/// Routing action for a matched rule
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RouteAction {
    /// Direct connection without proxy
    Direct,
    /// Route through SOCKS5 proxy
    Proxy,
    /// Reject the connection
    Reject,
}

/// Rule type for matching connections
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleType {
    /// Match exact domain
    Domain(String),
    /// Match domain suffix (e.g., ".google.com" matches "www.google.com")
    DomainSuffix(String),
    /// Match domain keyword
    DomainKeyword(String),
    /// Match IP CIDR range
    IpCidr(Ipv4Addr, u8),
    /// Match destination port
    DstPort(u16),
    /// Match source port
    SrcPort(u16),
    /// Match any connection (final rule)
    Final,
}

/// A single routing rule
#[derive(Debug, Clone)]
pub struct Rule {
    /// Rule type for matching
    pub rule_type: RuleType,
    /// Action to take when matched
    pub action: RouteAction,
    /// Optional rule name/comment
    pub name: Option<String>,
}

impl Rule {
    /// Create a new rule
    pub fn new(rule_type: RuleType, action: RouteAction) -> Self {
        Self {
            rule_type,
            action,
            name: None,
        }
    }

    /// Create a new rule with a name
    pub fn with_name(rule_type: RuleType, action: RouteAction, name: impl Into<String>) -> Self {
        Self {
            rule_type,
            action,
            name: Some(name.into()),
        }
    }

    /// Check if this rule matches the given connection
    pub fn matches(&self, domain: Option<&str>, ip: Option<IpAddr>, dst_port: u16, src_port: u16) -> bool {
        match &self.rule_type {
            RuleType::Domain(d) => domain.map(|h| h.eq_ignore_ascii_case(d)).unwrap_or(false),
            
            RuleType::DomainSuffix(suffix) => {
                domain.map(|h| {
                    let h_lower = h.to_ascii_lowercase();
                    let suffix_lower = suffix.to_ascii_lowercase();
                    h_lower.ends_with(&suffix_lower) || h_lower == suffix_lower.trim_start_matches('.')
                }).unwrap_or(false)
            }
            
            RuleType::DomainKeyword(keyword) => {
                domain.map(|h| h.to_ascii_lowercase().contains(&keyword.to_ascii_lowercase())).unwrap_or(false)
            }
            
            RuleType::IpCidr(network, prefix_len) => {
                if let Some(IpAddr::V4(addr)) = ip {
                    ip_in_cidr(addr, *network, *prefix_len)
                } else {
                    false
                }
            }
            
            RuleType::DstPort(port) => dst_port == *port,
            
            RuleType::SrcPort(port) => src_port == *port,
            
            RuleType::Final => true,
        }
    }
}

/// Check if an IP address is within a CIDR range
fn ip_in_cidr(addr: Ipv4Addr, network: Ipv4Addr, prefix_len: u8) -> bool {
    if prefix_len == 0 {
        return true;
    }
    if prefix_len > 32 {
        return false;
    }

    let addr_bits = u32::from(addr);
    let network_bits = u32::from(network);
    let mask = !0u32 << (32 - prefix_len);

    (addr_bits & mask) == (network_bits & mask)
}

/// Rule engine for evaluating routing decisions
pub struct RuleEngine {
    /// Ordered list of rules
    rules: Vec<Rule>,
    /// Default action when no rule matches
    default_action: RouteAction,
}

impl RuleEngine {
    /// Create a new rule engine with default direct routing
    pub fn new() -> Self {
        Self {
            rules: Vec::new(),
            default_action: RouteAction::Direct,
        }
    }

    /// Create a new rule engine with a custom default action
    pub fn with_default(default_action: RouteAction) -> Self {
        Self {
            rules: Vec::new(),
            default_action,
        }
    }

    /// Add a rule to the engine
    pub fn add_rule(&mut self, rule: Rule) {
        self.rules.push(rule);
    }

    /// Add multiple rules
    pub fn add_rules(&mut self, rules: impl IntoIterator<Item = Rule>) {
        self.rules.extend(rules);
    }

    /// Clear all rules
    pub fn clear(&mut self) {
        self.rules.clear();
    }

    /// Get the number of rules
    pub fn len(&self) -> usize {
        self.rules.len()
    }

    /// Check if there are no rules
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// Evaluate rules for a connection and return the action
    pub fn evaluate(&self, domain: Option<&str>, ip: Option<IpAddr>, dst_port: u16, src_port: u16) -> RouteAction {
        for rule in &self.rules {
            if rule.matches(domain, ip, dst_port, src_port) {
                return rule.action.clone();
            }
        }
        self.default_action.clone()
    }

    /// Load rules from a Surge-style configuration string
    pub fn load_from_config(&mut self, config: &str) -> Result<usize, String> {
        let mut count = 0;

        for line in config.lines() {
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') || line.starts_with("//") {
                continue;
            }

            if let Some(rule) = Self::parse_rule_line(line)? {
                self.add_rule(rule);
                count += 1;
            }
        }

        Ok(count)
    }

    /// Parse a single rule line
    fn parse_rule_line(line: &str) -> Result<Option<Rule>, String> {
        let parts: Vec<&str> = line.split(',').map(|s| s.trim()).collect();

        if parts.len() < 2 {
            return Err(format!("Invalid rule format: {}", line));
        }

        let rule_type_str = parts[0].to_uppercase();
        let action = Self::parse_action(parts.last().unwrap())?;

        let rule_type = match rule_type_str.as_str() {
            "DOMAIN" => {
                if parts.len() < 3 {
                    return Err("DOMAIN rule requires a domain".into());
                }
                RuleType::Domain(parts[1].to_string())
            }
            "DOMAIN-SUFFIX" => {
                if parts.len() < 3 {
                    return Err("DOMAIN-SUFFIX rule requires a suffix".into());
                }
                RuleType::DomainSuffix(parts[1].to_string())
            }
            "DOMAIN-KEYWORD" => {
                if parts.len() < 3 {
                    return Err("DOMAIN-KEYWORD rule requires a keyword".into());
                }
                RuleType::DomainKeyword(parts[1].to_string())
            }
            "IP-CIDR" | "IP-CIDR6" => {
                if parts.len() < 3 {
                    return Err("IP-CIDR rule requires a CIDR".into());
                }
                let cidr_parts: Vec<&str> = parts[1].split('/').collect();
                if cidr_parts.len() != 2 {
                    return Err(format!("Invalid CIDR format: {}", parts[1]));
                }
                let ip = Ipv4Addr::from_str(cidr_parts[0])
                    .map_err(|e| format!("Invalid IP: {}", e))?;
                let prefix: u8 = cidr_parts[1]
                    .parse()
                    .map_err(|e| format!("Invalid prefix length: {}", e))?;
                RuleType::IpCidr(ip, prefix)
            }
            "DST-PORT" => {
                if parts.len() < 3 {
                    return Err("DST-PORT rule requires a port".into());
                }
                let port: u16 = parts[1]
                    .parse()
                    .map_err(|e| format!("Invalid port: {}", e))?;
                RuleType::DstPort(port)
            }
            "SRC-PORT" => {
                if parts.len() < 3 {
                    return Err("SRC-PORT rule requires a port".into());
                }
                let port: u16 = parts[1]
                    .parse()
                    .map_err(|e| format!("Invalid port: {}", e))?;
                RuleType::SrcPort(port)
            }
            "FINAL" => RuleType::Final,
            _ => return Err(format!("Unknown rule type: {}", rule_type_str)),
        };

        Ok(Some(Rule::new(rule_type, action)))
    }

    /// Parse action string
    fn parse_action(s: &str) -> Result<RouteAction, String> {
        match s.to_uppercase().as_str() {
            "DIRECT" => Ok(RouteAction::Direct),
            "PROXY" => Ok(RouteAction::Proxy),
            "REJECT" => Ok(RouteAction::Reject),
            _ => Err(format!("Unknown action: {}", s)),
        }
    }

    /// Get all rules
    pub fn rules(&self) -> &[Rule] {
        &self.rules
    }
}

impl Default for RuleEngine {
    fn default() -> Self {
        Self::new()
    }
}

/// FFI-friendly route action enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub enum FfiRouteAction {
    Direct = 0,
    Proxy = 1,
    Reject = 2,
}

impl From<RouteAction> for FfiRouteAction {
    fn from(action: RouteAction) -> Self {
        match action {
            RouteAction::Direct => FfiRouteAction::Direct,
            RouteAction::Proxy => FfiRouteAction::Proxy,
            RouteAction::Reject => FfiRouteAction::Reject,
        }
    }
}

impl From<FfiRouteAction> for RouteAction {
    fn from(action: FfiRouteAction) -> Self {
        match action {
            FfiRouteAction::Direct => RouteAction::Direct,
            FfiRouteAction::Proxy => RouteAction::Proxy,
            FfiRouteAction::Reject => RouteAction::Reject,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_domain_match() {
        let rule = Rule::new(RuleType::Domain("example.com".into()), RouteAction::Proxy);

        assert!(rule.matches(Some("example.com"), None, 443, 0));
        assert!(rule.matches(Some("EXAMPLE.COM"), None, 443, 0));
        assert!(!rule.matches(Some("www.example.com"), None, 443, 0));
        assert!(!rule.matches(Some("example.org"), None, 443, 0));
        assert!(!rule.matches(None, None, 443, 0));
    }

    #[test]
    fn test_domain_suffix_match() {
        let rule = Rule::new(RuleType::DomainSuffix(".google.com".into()), RouteAction::Proxy);

        assert!(rule.matches(Some("www.google.com"), None, 443, 0));
        assert!(rule.matches(Some("mail.google.com"), None, 443, 0));
        assert!(rule.matches(Some("google.com"), None, 443, 0));
        assert!(!rule.matches(Some("google.org"), None, 443, 0));
        assert!(!rule.matches(Some("notgoogle.com"), None, 443, 0));
    }

    #[test]
    fn test_domain_keyword_match() {
        let rule = Rule::new(RuleType::DomainKeyword("google".into()), RouteAction::Proxy);

        assert!(rule.matches(Some("www.google.com"), None, 443, 0));
        assert!(rule.matches(Some("google.co.jp"), None, 443, 0));
        assert!(rule.matches(Some("googleapis.com"), None, 443, 0));
        assert!(!rule.matches(Some("example.com"), None, 443, 0));
    }

    #[test]
    fn test_ip_cidr_match() {
        let rule = Rule::new(
            RuleType::IpCidr(Ipv4Addr::new(192, 168, 0, 0), 16),
            RouteAction::Direct,
        );

        assert!(rule.matches(
            None,
            Some(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1))),
            443,
            0
        ));
        assert!(rule.matches(
            None,
            Some(IpAddr::V4(Ipv4Addr::new(192, 168, 255, 255))),
            443,
            0
        ));
        assert!(!rule.matches(
            None,
            Some(IpAddr::V4(Ipv4Addr::new(192, 169, 0, 1))),
            443,
            0
        ));
        assert!(!rule.matches(
            None,
            Some(IpAddr::V4(Ipv4Addr::new(10, 0, 0, 1))),
            443,
            0
        ));
    }

    #[test]
    fn test_port_match() {
        let dst_rule = Rule::new(RuleType::DstPort(443), RouteAction::Direct);
        let src_rule = Rule::new(RuleType::SrcPort(8080), RouteAction::Proxy);

        assert!(dst_rule.matches(None, None, 443, 0));
        assert!(!dst_rule.matches(None, None, 80, 0));

        assert!(src_rule.matches(None, None, 443, 8080));
        assert!(!src_rule.matches(None, None, 443, 9000));
    }

    #[test]
    fn test_final_match() {
        let rule = Rule::new(RuleType::Final, RouteAction::Proxy);

        assert!(rule.matches(None, None, 0, 0));
        assert!(rule.matches(Some("anything"), Some(IpAddr::V4(Ipv4Addr::new(1, 2, 3, 4))), 443, 8080));
    }

    #[test]
    fn test_rule_engine_evaluate() {
        let mut engine = RuleEngine::new();

        engine.add_rule(Rule::new(
            RuleType::DomainSuffix(".google.com".into()),
            RouteAction::Proxy,
        ));
        engine.add_rule(Rule::new(
            RuleType::IpCidr(Ipv4Addr::new(10, 0, 0, 0), 8),
            RouteAction::Direct,
        ));
        engine.add_rule(Rule::new(RuleType::Final, RouteAction::Proxy));

        assert_eq!(
            engine.evaluate(Some("www.google.com"), None, 443, 0),
            RouteAction::Proxy
        );
        assert_eq!(
            engine.evaluate(None, Some(IpAddr::V4(Ipv4Addr::new(10, 1, 2, 3))), 443, 0),
            RouteAction::Direct
        );
        assert_eq!(
            engine.evaluate(Some("example.com"), Some(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))), 443, 0),
            RouteAction::Proxy
        );
    }

    #[test]
    fn test_load_from_config() {
        let config = r#"
# This is a comment
DOMAIN, example.com, DIRECT
DOMAIN-SUFFIX, .google.com, PROXY
DOMAIN-KEYWORD, facebook, REJECT
IP-CIDR, 192.168.0.0/16, DIRECT
DST-PORT, 443, PROXY
FINAL, DIRECT
"#;

        let mut engine = RuleEngine::new();
        let count = engine.load_from_config(config).unwrap();

        assert_eq!(count, 6);
        assert_eq!(engine.len(), 6);
    }

    #[test]
    fn test_ip_in_cidr() {
        // /8 network
        assert!(ip_in_cidr(
            Ipv4Addr::new(10, 1, 2, 3),
            Ipv4Addr::new(10, 0, 0, 0),
            8
        ));
        assert!(!ip_in_cidr(
            Ipv4Addr::new(11, 0, 0, 1),
            Ipv4Addr::new(10, 0, 0, 0),
            8
        ));

        // /24 network
        assert!(ip_in_cidr(
            Ipv4Addr::new(192, 168, 1, 100),
            Ipv4Addr::new(192, 168, 1, 0),
            24
        ));
        assert!(!ip_in_cidr(
            Ipv4Addr::new(192, 168, 2, 1),
            Ipv4Addr::new(192, 168, 1, 0),
            24
        ));

        // /32 (exact match)
        assert!(ip_in_cidr(
            Ipv4Addr::new(8, 8, 8, 8),
            Ipv4Addr::new(8, 8, 8, 8),
            32
        ));
        assert!(!ip_in_cidr(
            Ipv4Addr::new(8, 8, 8, 9),
            Ipv4Addr::new(8, 8, 8, 8),
            32
        ));

        // /0 (match all)
        assert!(ip_in_cidr(
            Ipv4Addr::new(1, 2, 3, 4),
            Ipv4Addr::new(0, 0, 0, 0),
            0
        ));
    }

    #[test]
    fn test_ffi_route_action_conversion() {
        assert_eq!(FfiRouteAction::from(RouteAction::Direct), FfiRouteAction::Direct);
        assert_eq!(FfiRouteAction::from(RouteAction::Proxy), FfiRouteAction::Proxy);
        assert_eq!(FfiRouteAction::from(RouteAction::Reject), FfiRouteAction::Reject);

        assert_eq!(RouteAction::from(FfiRouteAction::Direct), RouteAction::Direct);
        assert_eq!(RouteAction::from(FfiRouteAction::Proxy), RouteAction::Proxy);
        assert_eq!(RouteAction::from(FfiRouteAction::Reject), RouteAction::Reject);
    }

    #[test]
    fn test_rule_with_name() {
        let rule = Rule::with_name(
            RuleType::Domain("example.com".into()),
            RouteAction::Direct,
            "Example rule",
        );

        assert_eq!(rule.name, Some("Example rule".to_string()));
    }

    #[test]
    fn test_parse_invalid_config() {
        let mut engine = RuleEngine::new();

        // Unknown rule type
        let result = engine.load_from_config("UNKNOWN, foo, DIRECT");
        assert!(result.is_err());

        // Missing action
        let result = engine.load_from_config("DOMAIN");
        assert!(result.is_err());
    }

    #[test]
    fn test_clear_rules() {
        let mut engine = RuleEngine::new();
        engine.add_rule(Rule::new(RuleType::Final, RouteAction::Direct));

        assert_eq!(engine.len(), 1);

        engine.clear();

        assert_eq!(engine.len(), 0);
        assert!(engine.is_empty());
    }
}
