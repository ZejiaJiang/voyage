# voyage-core - Rust Network Engine

The cross-platform Rust core library powering Voyage iOS and VoyageMac applications. Provides a userspace TCP/IP stack, NAT management, rule-based routing, and SOCKS5 proxy support.

## Overview

`voyage-core` is designed to run inside iOS Network Extensions and macOS System Extensions, processing raw IP packets and routing them through a SOCKS5 proxy based on configurable rules.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                        voyage-core                               │
├─────────────────────────────────────────────────────────────────┤
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                     FFI Layer (ffi.rs)                     │  │
│  │  • init_core()          • load_rules()                     │  │
│  │  • shutdown_core()      • evaluate_route()                 │  │
│  │  • process_inbound_packet()    • get_stats()              │  │
│  │  • process_outbound_packet()   • enable/disable_proxy()   │  │
│  └───────────────────────────────────────────────────────────┘  │
│                              │                                   │
│  ┌───────────────────────────┼───────────────────────────────┐  │
│  │                    VoyageCore (lib.rs)                     │  │
│  │  Main orchestrator combining all components                 │  │
│  └───────────────────────────┼───────────────────────────────┘  │
│                              │                                   │
│  ┌───────────────┬───────────┴───────────┬───────────────────┐  │
│  │               │                       │                   │  │
│  ▼               ▼                       ▼                   ▼  │
│ ┌─────────┐ ┌──────────┐ ┌─────────────────┐ ┌─────────────┐   │
│ │ device  │ │   nat    │ │   connection    │ │    proxy    │   │
│ │         │ │          │ │                 │ │             │   │
│ │ Virtual │ │ NAT      │ │ Connection      │ │ Proxy       │   │
│ │ TUN     │ │ Manager  │ │ Manager         │ │ Manager     │   │
│ │ Device  │ │          │ │                 │ │             │   │
│ └────┬────┘ └────┬─────┘ └────────┬────────┘ └──────┬──────┘   │
│      │           │                │                 │           │
│  ┌───┴───┐  ┌────┴────┐     ┌─────┴─────┐    ┌─────┴─────┐     │
│  │ iface │  │ packet  │     │   rule    │    │  socks5   │     │
│  │       │  │         │     │           │    │           │     │
│  │smoltcp│  │ IPv4    │     │ Rule      │    │ SOCKS5    │     │
│  │Iface  │  │ Parser  │     │ Engine    │    │ Client    │     │
│  └───────┘  └─────────┘     └───────────┘    └───────────┘     │
└─────────────────────────────────────────────────────────────────┘
```

## Module Responsibilities

### `lib.rs` - Main Library
**Purpose**: Public API and module orchestration

| Export | Description |
|--------|-------------|
| `VoyageCore` | Main engine struct combining all components |
| `ProxyConfig` | SOCKS5 server configuration |
| `VoyageError` | Unified error type |
| FFI functions | All functions exposed to Swift |

```rust
pub struct VoyageCore {
    pub config: ProxyConfig,
    pub conn_manager: ConnectionManager,
    pub proxy_manager: ProxyManager,
}
```

### `config.rs` - Configuration
**Purpose**: Define configuration types for the proxy

```rust
pub struct ProxyConfig {
    pub server_host: String,    // SOCKS5 server address
    pub server_port: u16,       // SOCKS5 server port
    pub username: Option<String>,
    pub password: Option<String>,
}
```

### `error.rs` - Error Handling
**Purpose**: Unified error type with thiserror

```rust
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum VoyageError {
    #[error("Not initialized")]
    NotInitialized,
    #[error("Already initialized")]
    AlreadyInitialized,
    #[error("Invalid packet: {0}")]
    InvalidPacket(String),
    #[error("NAT table full")]
    NatTableFull,
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    // ... more variants
}
```

### `device.rs` - Virtual TUN Device
**Purpose**: smoltcp-compatible virtual network device

**Components**:
- `VirtualTunDevice` - Implements smoltcp's `Device` trait
- `PacketQueue` - Thread-safe packet buffer (`Arc<Mutex<VecDeque<Vec<u8>>>>`)
- `VirtualRxToken` / `VirtualTxToken` - smoltcp token types

**Key Methods**:
```rust
impl VirtualTunDevice {
    pub fn new() -> Self;
    pub fn inject_packet(&self, packet: Vec<u8>);  // From TUN
    pub fn take_packets(&self) -> Vec<Vec<u8>>;    // To TUN
    pub fn rx_queue(&self) -> PacketQueue;
    pub fn tx_queue(&self) -> PacketQueue;
}
```

### `iface.rs` - Interface Manager
**Purpose**: Wrapper around smoltcp's `Interface` and `SocketSet`

**Responsibilities**:
- Manage smoltcp network interface
- Allocate TCP/UDP sockets
- Drive the TCP/IP state machine with `poll()`
- Handle port allocation (10000-65535)

```rust
pub struct InterfaceManager {
    iface: Interface,
    sockets: SocketSet<'static>,
    next_local_port: u16,
}
```

### `nat.rs` - NAT Manager
**Purpose**: Network Address Translation for connection tracking

**Components**:
- `NatKey` - 5-tuple identifier (src_ip, src_port, dst_ip, dst_port, protocol)
- `NatEntry` - Connection state and metadata
- `NatState` - Connection lifecycle (New → Established → Closed)
- `NatManager` - HashMap-based NAT table

**Flow**:
1. Packet arrives → Generate `NatKey`
2. Lookup or create `NatEntry`
3. Assign local port for smoltcp socket
4. Track bytes sent/received
5. Update connection state

```rust
pub struct NatManager {
    entries: HashMap<NatKey, NatEntry>,
    port_to_key: HashMap<u16, NatKey>,
    next_port: u16,
}
```

### `packet.rs` - Packet Parser
**Purpose**: Parse raw IPv4/TCP/UDP packets

**Output Structure**:
```rust
pub struct ParsedPacket {
    pub ip: IpPacketInfo,       // src_ip, dst_ip, protocol, ttl
    pub tcp: Option<TcpPacketInfo>,  // ports, flags, seq, ack
    pub udp: Option<UdpPacketInfo>,  // ports, length
    pub payload_offset: usize,
    pub payload_len: usize,
}
```

**TCP Flags**:
```rust
pub struct TcpFlags {
    pub syn: bool,
    pub ack: bool,
    pub fin: bool,
    pub rst: bool,
    pub psh: bool,
}
```

### `connection.rs` - Connection Manager
**Purpose**: High-level connection tracking combining NAT and sockets

**Responsibilities**:
- Process packets and create/update connections
- Map NAT entries to smoltcp socket handles
- Track connection state (Connecting → Established → Closing → Closed)
- Aggregate statistics

```rust
pub struct ConnectionManager {
    nat: NatManager,
    socket_handles: HashMap<NatKey, SocketHandle>,
    handle_to_key: HashMap<SocketHandle, NatKey>,
}

pub struct ConnectionInfo {
    pub key: NatKey,
    pub state: ConnectionState,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub created_at: Instant,
}
```

### `rule.rs` - Rule Engine
**Purpose**: Surge-style routing rule evaluation

**Rule Types**:
| Type | Example | Description |
|------|---------|-------------|
| `DOMAIN` | `DOMAIN,www.google.com,PROXY` | Exact domain match |
| `DOMAIN-SUFFIX` | `DOMAIN-SUFFIX,google.com,PROXY` | Domain ends with |
| `DOMAIN-KEYWORD` | `DOMAIN-KEYWORD,facebook,REJECT` | Domain contains |
| `IP-CIDR` | `IP-CIDR,10.0.0.0/8,DIRECT` | IP range match |
| `GEOIP` | `GEOIP,CN,DIRECT` | Country code (placeholder) |
| `DST-PORT` | `DST-PORT,443,PROXY` | Destination port |
| `FINAL` | `FINAL,DIRECT` | Default action |

**Actions**:
- `DIRECT` - Connect directly without proxy
- `PROXY` - Route through SOCKS5 proxy
- `REJECT` - Drop the connection

```rust
pub struct RuleEngine {
    rules: Vec<Rule>,
}

impl RuleEngine {
    pub fn evaluate(&self, domain: Option<&str>, ip: Option<IpAddr>, 
                    port: u16, protocol: u8) -> RouteAction;
}
```

### `proxy.rs` - Proxy Manager
**Purpose**: Manage proxy routing decisions and statistics

**Components**:
- Wraps `RuleEngine` for rule evaluation
- Tracks routing statistics
- Provides enable/disable functionality

```rust
pub struct ProxyManager {
    engine: RuleEngine,
    config: ProxyConfig,
    enabled: bool,
    stats: ProxyStats,
}

pub struct RoutingDecision {
    pub action: RouteAction,
    pub rule_name: Option<String>,
    pub matched_pattern: Option<String>,
}
```

### `socks5.rs` - SOCKS5 Client
**Purpose**: Async SOCKS5 proxy protocol implementation

**Features**:
- SOCKS5 handshake (RFC 1928)
- Username/password authentication (RFC 1929)
- IPv4, IPv6, and domain name targets
- TCP CONNECT command

```rust
pub struct Socks5Client {
    server_addr: SocketAddr,
    auth: Option<(String, String)>,
}

pub enum TargetAddr {
    Ip(SocketAddr),
    Domain(String, u16),
}

impl Socks5Client {
    pub async fn connect(&self, target: TargetAddr) -> Result<TcpStream, VoyageError>;
}
```

### `ffi.rs` - Foreign Function Interface
**Purpose**: UniFFI-exported functions for Swift interop

**Global State**:
```rust
static CORE_INSTANCE: OnceLock<Arc<Mutex<VoyageCore>>> = OnceLock::new();
```

**Exported Functions**:
| Function | Description |
|----------|-------------|
| `init_core(host, port, user, pass)` | Initialize the core |
| `shutdown_core()` | Shutdown and cleanup |
| `process_inbound_packet(data)` | Process packet from TUN |
| `process_outbound_packet(data)` | Process packet to TUN |
| `load_rules(text)` | Load routing rules |
| `evaluate_route(domain, ip, port)` | Get routing decision |
| `get_stats()` | Get traffic statistics |
| `enable_proxy()` / `disable_proxy()` | Toggle proxy |
| `is_initialized()` | Check init state |

### `voyage_core.udl` - UniFFI Definition
**Purpose**: Define the FFI interface for Swift binding generation

```
namespace voyage_core {
    void init_core(...);
    CoreStats get_stats();
    // ... all exported functions
};
```

## Build Targets

| Target | Platform | Command |
|--------|----------|---------|
| `aarch64-apple-ios` | iOS Device | `./build-apple.sh ios` |
| `aarch64-apple-ios-sim` | iOS Simulator (ARM) | `./build-apple.sh ios` |
| `x86_64-apple-ios` | iOS Simulator (Intel) | `./build-apple.sh ios` |
| `aarch64-apple-darwin` | macOS (Apple Silicon) | `./build-apple.sh macos` |
| `x86_64-apple-darwin` | macOS (Intel) | `./build-apple.sh macos` |
| `x86_64-pc-windows-msvc` | Windows | `cargo build` |

## Testing

```bash
# Run all tests (100 total)
cargo test

# Unit tests only (86 tests)
cargo test --lib

# Integration tests only (14 tests)
cargo test --test integration_test

# Run with output
cargo test -- --nocapture

# Run demo binary
cargo run --bin demo
```

## Test Coverage

| Module | Tests | Coverage |
|--------|-------|----------|
| `config` | 2 | Configuration creation |
| `device` | 4 | TUN device operations |
| `iface` | 5 | Interface management |
| `nat` | 10 | NAT table operations |
| `packet` | 10 | Packet parsing |
| `connection` | 10 | Connection lifecycle |
| `rule` | 15 | Rule matching |
| `proxy` | 15 | Proxy routing |
| `socks5` | 12 | SOCKS5 protocol |
| `ffi` | 3 | FFI functions |
| Integration | 14 | End-to-end flows |

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `smoltcp` | 0.11 | Userspace TCP/IP stack |
| `tokio` | 1 | Async runtime (minimal features) |
| `uniffi` | 0.28 | Swift FFI bindings |
| `thiserror` | 1 | Error derive macro |
| `log` | 0.4 | Logging facade |
| `env_logger` | 0.11 | Logger implementation |
| `serial_test` | 3 | Test serialization |

## Memory Optimization

For iOS Network Extensions (15-50MB limit):

```toml
[profile.release]
opt-level = "z"      # Optimize for size
lto = true           # Link-time optimization
codegen-units = 1    # Single codegen unit
panic = "abort"      # No unwinding
strip = true         # Remove symbols
```

## Generated Outputs

After running `./build-apple.sh`:

```
voyage-core/
├── target/
│   ├── aarch64-apple-ios/release/libvoyage_core.a
│   ├── aarch64-apple-ios-sim/release/libvoyage_core.a
│   ├── x86_64-apple-ios/release/libvoyage_core.a
│   ├── universal-ios-sim/release/libvoyage_core.a
│   ├── aarch64-apple-darwin/release/libvoyage_core.a
│   ├── x86_64-apple-darwin/release/libvoyage_core.a
│   └── universal-macos/release/libvoyage_core.a
│
└── generated/
    ├── voyage_core.swift      # Swift bindings
    ├── voyage_coreFFI.h       # C header
    └── module.modulemap       # Module definition
```

## License

MIT License
