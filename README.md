# Voyage - Cross-Platform Proxy App with Rust Core

A Surge-like proxy application for **iOS** and **macOS** using Rust and smoltcp for the network engine.

## Platforms

| Platform | App | Extension Type | Status |
|----------|-----|----------------|--------|
| iOS | Voyage | Network Extension | ✅ Complete |
| macOS | VoyageMac | System Extension | ✅ Complete |

## Architecture Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                     Main App (SwiftUI)                          │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                    SwiftUI Interface                      │   │
│  │  • Configuration UI     • Connection Status               │   │
│  │  • Rules Editor         • Statistics Display              │   │
│  │  • Menu Bar (macOS)     • Logs Viewer                     │   │
│  └─────────────────────────────────────────────────────────┘   │
└──────────────────────────┬──────────────────────────────────────┘
                           │ NETunnelProviderManager
                           ▼
┌─────────────────────────────────────────────────────────────────┐
│          Network/System Extension (VoyageTunnel)                │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │              NEPacketTunnelProvider                       │   │
│  │  • Reads IP packets from UTUN interface                  │   │
│  │  • Forwards packets to Rust Core via FFI                 │   │
│  │  • Writes processed packets back to UTUN                 │   │
│  └───────────────────────┬─────────────────────────────────┘   │
│                          │ UniFFI                               │
│                          ▼                                      │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                 Rust Core (voyage-core)                   │   │
│  │  ┌─────────────────────────────────────────────────────┐ │   │
│  │  │           smoltcp (Userspace TCP/IP Stack)           │ │   │
│  │  └─────────────────────────────────────────────────────┘ │   │
│  │  ┌─────────────────────────────────────────────────────┐ │   │
│  │  │                NAT & Socket Manager                   │ │   │
│  │  └─────────────────────────────────────────────────────┘ │   │
│  │  ┌─────────────────────────────────────────────────────┐ │   │
│  │  │              Rule Engine & SOCKS5 Client             │ │   │
│  │  └─────────────────────────────────────────────────────┘ │   │
│  └─────────────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────────────┘
```

## Project Structure

```
voyage/
├── .gitignore               # Git ignore rules
├── README.md                # This file
│
├── voyage-core/             # Rust library (shared core)
│   ├── Cargo.toml          # Dependencies & build config
│   ├── Cargo.lock          # Locked dependencies
│   ├── build.rs            # UniFFI build script
│   ├── build-apple.sh      # macOS/iOS build script
│   ├── build-windows.ps1   # Windows build script
│   ├── src/
│   │   ├── lib.rs          # Main library & public API
│   │   ├── config.rs       # Configuration types
│   │   ├── error.rs        # Error handling
│   │   ├── device.rs       # smoltcp virtual TUN device
│   │   ├── iface.rs        # Interface manager
│   │   ├── nat.rs          # NAT manager
│   │   ├── packet.rs       # IPv4 packet parsing
│   │   ├── connection.rs   # Connection manager
│   │   ├── rule.rs         # Rule engine
│   │   ├── socks5.rs       # SOCKS5 client
│   │   ├── proxy.rs        # Proxy manager
│   │   ├── ffi.rs          # FFI interface
│   │   ├── voyage_core.udl # UniFFI interface definition
│   │   └── bin/
│   │       └── demo.rs     # Demo binary
│   └── tests/
│       └── integration_test.rs
│
├── Voyage/                  # iOS Xcode project
│   ├── Voyage/             # Main app target
│   │   ├── VoyageApp.swift
│   │   ├── ContentView.swift
│   │   ├── Info.plist
│   │   └── Voyage.entitlements
│   └── VoyageTunnel/       # Network Extension
│       ├── PacketTunnelProvider.swift
│       ├── Info.plist
│       └── VoyageTunnel.entitlements
│
└── VoyageMac/              # macOS Xcode project
    ├── README.md           # macOS-specific documentation
    ├── VoyageMac/          # Main app target
    │   ├── VoyageMacApp.swift
    │   ├── ContentView.swift
    │   ├── VPNManager.swift
    │   ├── SettingsView.swift
    │   ├── MenuBarView.swift
    │   ├── SystemExtensionManager.swift
    │   ├── Info.plist
    │   ├── VoyageMac.entitlements
    │   └── Assets.xcassets/
    └── VoyageTunnel/       # System Extension
        └── PacketTunnelProvider.swift
```

## Quick Start

### Prerequisites

1. **Rust** - Install from https://rustup.rs/
2. **Xcode** (macOS only) - Install from Mac App Store
3. **Apple Developer Account** - Required for Network/System Extension entitlements

### Build & Test on Windows/Linux

```bash
cd voyage-core

# Run all tests (100 tests)
cargo test

# Run the demo
cargo run --bin demo
```

### Build & Test on Windows (PowerShell)

```powershell
cd voyage-core

# Full build and test
.\build-windows.ps1

# Or individual commands
.\build-windows.ps1 test   # Run tests
.\build-windows.ps1 demo   # Run demo
.\build-windows.ps1 build  # Build only
```

### Build for iOS/macOS (on Mac)

```bash
cd voyage-core

# Make script executable
chmod +x build-apple.sh

# Build everything (iOS + macOS + bindings)
./build-apple.sh

# Or individual targets
./build-apple.sh ios      # iOS only
./build-apple.sh macos    # macOS only
./build-apple.sh bindings # Generate Swift bindings
./build-apple.sh summary  # Show build artifacts
```

## Rust Core Modules

| Module | Description |
|--------|-------------|
| `lib.rs` | Public API, VoyageCore struct |
| `config.rs` | ProxyConfig with server settings |
| `error.rs` | VoyageError enum with thiserror |
| `device.rs` | VirtualTunDevice for smoltcp |
| `iface.rs` | InterfaceManager wrapping smoltcp |
| `nat.rs` | NatManager for connection tracking |
| `packet.rs` | ParsedPacket for IPv4/TCP/UDP parsing |
| `connection.rs` | ConnectionManager combining NAT + sockets |
| `rule.rs` | RuleEngine with Surge-style rules |
| `proxy.rs` | ProxyManager for routing decisions |
| `socks5.rs` | SOCKS5 client implementation |
| `ffi.rs` | UniFFI exported functions |

## Rule Engine

Supports Surge-style routing rules:

```
# Domain matching
DOMAIN, www.google.com, PROXY
DOMAIN-SUFFIX, google.com, PROXY
DOMAIN-KEYWORD, facebook, REJECT

# IP matching
IP-CIDR, 10.0.0.0/8, DIRECT
IP-CIDR, 192.168.0.0/16, DIRECT

# GEOIP (placeholder)
GEOIP, CN, DIRECT

# Port matching
DST-PORT, 443, PROXY
DST-PORT, 80, DIRECT

# Default rule
FINAL, PROXY
```

## Test Results

```
cargo test
   
running 86 tests (unit tests)
...
test result: ok. 86 passed; 0 failed

running 14 tests (integration tests)
...
test result: ok. 14 passed; 0 failed

Total: 100 tests passed
```

## Memory Constraints

⚠️ **Important**: iOS Network Extensions have a hard memory limit of 15-50MB.

### Rust Optimizations (configured in Cargo.toml)

```toml
[profile.release]
opt-level = "z"      # Optimize for size
lto = true           # Link-time optimization
codegen-units = 1    # Better optimization
panic = "abort"      # Smaller binary
strip = true         # Remove symbols
```

## Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| smoltcp | 0.11 | Userspace TCP/IP stack |
| tokio | 1 | Async runtime (minimal) |
| uniffi | 0.28 | Swift FFI bindings |
| thiserror | 1 | Error handling |
| env_logger | 0.11 | Logging |
| serial_test | 3 | Test serialization |

## Development Roadmap

- [x] **Step 1**: Setup cross-compilation environment
- [x] **Step 2**: Implement smoltcp virtual device
- [x] **Step 3**: Build NAT & Socket Manager
- [x] **Step 4**: Implement Rule Engine & SOCKS5 Client
- [x] **Step 5**: iOS Tunnel Provider Integration
- [x] **Step 6**: macOS App with System Extension
- [ ] **Step 7**: Full proxy tunnel implementation
- [ ] **Step 8**: App Store release

## Features

### Rust Core (voyage-core)
- ✅ Userspace TCP/IP stack (smoltcp 0.11)
- ✅ NAT & connection tracking
- ✅ Surge-style rule engine (DOMAIN, DOMAIN-SUFFIX, IP-CIDR, DST-PORT, FINAL)
- ✅ SOCKS5 client with authentication
- ✅ Proxy routing (DIRECT, PROXY, REJECT)
- ✅ 86 unit tests + 14 integration tests
- ✅ Cross-platform (Windows, macOS, Linux, iOS)

### iOS App (Voyage)
- ✅ SwiftUI interface
- ✅ VPN connection management
- ✅ Statistics display
- ✅ NEPacketTunnelProvider integration

### macOS App (VoyageMac)
- ✅ Native SwiftUI interface
- ✅ Menu bar app with quick access
- ✅ Dashboard with real-time stats
- ✅ Connections viewer
- ✅ Rules editor
- ✅ Logs viewer with filtering
- ✅ Settings panel
- ✅ System Extension for VPN

## Tech Stack

| Component | Technology |
|-----------|------------|
| Core Language | Rust |
| UI Language | Swift (SwiftUI) |
| TCP/IP Stack | smoltcp (Userspace) |
| Async Runtime | tokio |
| FFI | UniFFI |
| iOS Extension | Network Extension |
| macOS Extension | System Extension |
| Build Tool | cargo + Xcode |

## License

MIT License
