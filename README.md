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
├── voyage-core/              # Rust library (shared)
│   ├── Cargo.toml           # Dependencies & build config
│   ├── build.rs             # UniFFI build script
│   ├── Makefile             # Build automation (iOS + macOS)
│   ├── build_ios.sh         # iOS build script
│   ├── build_macos.sh       # macOS build script
│   └── src/
│       ├── lib.rs           # Main library
│       ├── config.rs        # Configuration types
│       ├── device.rs        # smoltcp virtual device
│       ├── iface.rs         # Interface manager
│       ├── nat.rs           # NAT manager
│       ├── connection.rs    # Connection manager
│       ├── rule.rs          # Rule engine
│       ├── socks5.rs        # SOCKS5 client
│       ├── proxy.rs         # Proxy manager
│       ├── ffi.rs           # FFI interface
│       └── error.rs         # Error handling
│
├── Voyage/                   # iOS Xcode project
│   ├── Voyage/              # Main app target
│   │   ├── VoyageApp.swift
│   │   └── ContentView.swift
│   └── VoyageTunnel/        # Network Extension
│       └── PacketTunnelProvider.swift
│
└── VoyageMac/                # macOS Xcode project
    ├── VoyageMac/           # Main app target
    │   ├── VoyageMacApp.swift
    │   ├── ContentView.swift
    │   ├── VPNManager.swift
    │   ├── SettingsView.swift
    │   ├── MenuBarView.swift
    │   └── SystemExtensionManager.swift
    └── VoyageTunnel/        # System Extension
        └── PacketTunnelProvider.swift
```

## Quick Start

### Prerequisites

1. **Rust** - Install from https://rustup.rs/
2. **Xcode** - Install from Mac App Store
3. **Apple Developer Account** - Required for Network/System Extension entitlements

### Build Rust Core

```bash
cd voyage-core

# Install all targets
make setup

# Build for iOS
make build-ios

# Build for macOS (universal binary)
make build-universal

# Generate Swift bindings
make generate-bindings

# Run tests
cargo test
```

### iOS Development

See the [Voyage iOS README](Voyage/README.md) for iOS-specific setup.

### macOS Development

See the [VoyageMac README](VoyageMac/README.md) for macOS-specific setup.

## Step 1: Setup Cross-Compilation Environment

### Install Rust Targets

```bash
# iOS targets
rustup target add aarch64-apple-ios
rustup target add aarch64-apple-ios-sim

# macOS targets
rustup target add x86_64-apple-darwin
rustup target add aarch64-apple-darwin
```

### Install UniFFI Bindgen

```bash
cargo install uniffi_bindgen
```

### Install iOS Rust Targets

Run the following commands to install the iOS compilation targets:

```bash
# Install iOS device target (arm64)
rustup target add aarch64-apple-ios

# Install iOS Simulator target (Apple Silicon)
rustup target add aarch64-apple-ios-sim

# Optional: Intel Mac simulator target
rustup target add x86_64-apple-ios
```

### Install UniFFI Bindgen

```bash
cargo install uniffi_bindgen
```

### Verify Installation

```bash
rustup target list --installed | grep ios
# Should show:
# aarch64-apple-ios
# aarch64-apple-ios-sim
```

## Building the Rust Library

### Using Makefile (Recommended)

```bash
cd voyage-core

# First-time setup
make setup

# Build for iOS
make build-ios

# Generate Swift bindings
make generate-bindings

# Full release build
make release
```

### Using Build Script

```bash
cd voyage-core
chmod +x build_ios.sh
./build_ios.sh
```

### Manual Build

```bash
cd voyage-core

# Build for iOS device
cargo build --release --target aarch64-apple-ios

# Build for iOS Simulator (Apple Silicon)
cargo build --release --target aarch64-apple-ios-sim

# Generate Swift bindings
uniffi-bindgen generate \
    --library target/aarch64-apple-ios/release/libvoyage_core.a \
    --language swift \
    --out-dir generated/swift
```

## Xcode Project Setup

### 1. Create New Xcode Project

1. Open Xcode
2. File → New → Project
3. Select "App" under iOS
4. Product Name: `Voyage`
5. Bundle Identifier: `com.voyage.app`
6. Interface: SwiftUI
7. Language: Swift

### 2. Add Network Extension Target

1. File → New → Target
2. Select "Network Extension"
3. Product Name: `VoyageTunnel`
4. Provider Type: Packet Tunnel
5. Bundle Identifier: `com.voyage.app.tunnel`

### 3. Configure Entitlements

Both the main app and the tunnel extension need Network Extension entitlements:

1. Select your target → Signing & Capabilities
2. Click "+ Capability"
3. Add "Network Extensions"
4. Enable "Packet Tunnel"
5. Add "App Groups" capability
6. Create group: `group.com.voyage.app`

### 4. Link Rust Library

1. Add the generated Swift files to both targets
2. Add the static library:
   - Build Settings → Library Search Paths
   - Add: `$(PROJECT_DIR)/../voyage-core/target/aarch64-apple-ios/release`
3. Link the library:
   - Build Phases → Link Binary With Libraries
   - Add `libvoyage_core.a`

### 5. Configure Header Search Path

1. Build Settings → Header Search Paths
2. Add: `$(PROJECT_DIR)/../voyage-core/generated/include`

## Memory Constraints

⚠️ **Critical**: iOS Network Extensions have a hard memory limit of 15-50MB.

### Rust Optimizations (already configured in Cargo.toml)

```toml
[profile.release]
opt-level = "z"      # Optimize for size
lto = true           # Link-time optimization
codegen-units = 1    # Better optimization
panic = "abort"      # Smaller binary
strip = true         # Remove symbols
```

### Runtime Guidelines

1. Pre-allocate socket buffers (64KB per connection max)
2. Limit concurrent connections (e.g., max 100)
3. Avoid large Vec allocations
4. Use streaming instead of buffering entire payloads

## Development Roadmap

- [x] **Step 1**: Setup cross-compilation environment
- [x] **Step 2**: Implement smoltcp virtual device
- [x] **Step 3**: Build NAT & Socket Manager
- [x] **Step 4**: Implement Rule Engine & SOCKS5 Client
- [x] **Step 5**: iOS Tunnel Provider Integration
- [x] **Step 6**: macOS App with System Extension

## Testing

```bash
cd voyage-core

# Run all unit tests (60 tests)
cargo test --lib

# Run integration tests (simulates tunnel behavior)
cargo test --test integration_test

# Run everything
cargo test

# Run demo binary (Windows/macOS/Linux)
cargo run --bin demo
```

## Features

### Rust Core (voyage-core)
- ✅ Userspace TCP/IP stack (smoltcp)
- ✅ NAT & connection tracking
- ✅ Surge-style rule engine (DOMAIN, DOMAIN-SUFFIX, IP-CIDR, GEOIP, etc.)
- ✅ SOCKS5 client with authentication
- ✅ Proxy routing (DIRECT, PROXY, REJECT)
- ✅ 60+ unit tests, 5 integration tests

### iOS App (Voyage)
- ✅ SwiftUI interface
- ✅ VPN connection management
- ✅ Statistics display
- ✅ NEPacketTunnelProvider integration

### macOS App (VoyageMac)
- ✅ Native SwiftUI interface
- ✅ Menu bar app
- ✅ Dashboard with real-time stats
- ✅ Connections viewer
- ✅ Rules editor
- ✅ Logs viewer with filtering
- ✅ Settings panel
- ✅ System Extension for VPN

## Tech Stack

| Component | Technology |
|-----------|------------|
| Language | Rust (Core), Swift (UI/Extension) |
| TCP/IP Stack | smoltcp (Userspace) |
| Async Runtime | tokio (Minimalistic config) |
| FFI | UniFFI |
| iOS Extension | Network Extension |
| macOS Extension | System Extension |
| Build Tool | cargo + Xcode |

## License

MIT License
