# Voyage - iOS Proxy Application

A Surge-like proxy application for iOS using SwiftUI and the Voyage Rust core.

## Overview

Voyage is an iOS VPN/proxy app that routes network traffic through a SOCKS5 proxy server. It uses a Network Extension to intercept all device traffic and processes it through a high-performance Rust networking core.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Voyage App (SwiftUI)                 │
│  ┌─────────────────────────────────────────────────┐   │
│  │                  ContentView                      │   │
│  │  • VPN Connect/Disconnect toggle                  │   │
│  │  • Server configuration (host, port)              │   │
│  │  • Real-time statistics display                   │   │
│  └────────────────────┬────────────────────────────┘   │
│                       │ NETunnelProviderManager         │
└───────────────────────┼─────────────────────────────────┘
                        ▼
┌─────────────────────────────────────────────────────────┐
│               VoyageTunnel (Network Extension)          │
│  ┌─────────────────────────────────────────────────┐   │
│  │            PacketTunnelProvider                   │   │
│  │  • Reads IP packets from UTUN interface           │   │
│  │  • Forwards packets to Rust core via FFI          │   │
│  │  • Writes processed packets back to UTUN          │   │
│  │  • Manages tunnel lifecycle                       │   │
│  └────────────────────┬────────────────────────────┘   │
│                       │ UniFFI                          │
│                       ▼                                 │
│              voyage-core (Rust)                         │
└─────────────────────────────────────────────────────────┘
```

## Project Structure

```
Voyage/
├── Voyage/                      # Main App Target
│   ├── VoyageApp.swift         # App entry point (@main)
│   ├── ContentView.swift       # Main UI with VPN controls
│   ├── Info.plist              # App configuration
│   └── Voyage.entitlements     # Network Extension entitlements
│
└── VoyageTunnel/                # Network Extension Target
    ├── PacketTunnelProvider.swift  # NEPacketTunnelProvider subclass
    ├── Info.plist                  # Extension configuration
    └── VoyageTunnel.entitlements   # Extension entitlements
```

## Components

### Main App (`Voyage/`)

#### ContentView.swift
The main user interface providing:
- **VPN Status Display**: Shows Connected/Disconnected/Connecting states
- **Connect/Disconnect Button**: Toggle VPN connection
- **Server Configuration**: SOCKS5 server host and port inputs
- **Statistics Section**: 
  - Bytes sent/received (formatted with ByteCountFormatter)
  - Active connections count

#### VPNManager
`@MainActor` class managing VPN lifecycle:
- `loadFromPreferences()` - Loads saved NETunnelProviderManager
- `toggleVPN()` - Connect or disconnect the VPN
- `isConnected` / `isConnecting` - Connection state
- `stats` - VPNStats struct with traffic data

### Network Extension (`VoyageTunnel/`)

#### PacketTunnelProvider.swift
Subclass of `NEPacketTunnelProvider` responsible for:

1. **Tunnel Lifecycle**
   - `startTunnel(options:completionHandler:)` - Initialize Rust core, configure network settings, start packet loop
   - `stopTunnel(with:completionHandler:)` - Shutdown core, log final stats

2. **Rust Core Integration**
   - Calls `initCore()` on tunnel start
   - Calls `shutdownCore()` on tunnel stop
   - Uses FFI functions: `processInboundPacket()`, `processOutboundPacket()`, `getStats()`

3. **Packet Processing Loop**
   - Reads packets from `packetFlow.readPackets()`
   - Sends each packet through `process_inbound_packet()`
   - Writes responses back with `packetFlow.writePackets()`

4. **Configuration Loading**
   - Reads server settings from `protocolConfiguration.providerConfiguration`
   - Loads routing rules from config or defaults

5. **Default Routing Rules**
   ```
   IP-CIDR,127.0.0.0/8,DIRECT
   IP-CIDR,192.168.0.0/16,DIRECT
   IP-CIDR,10.0.0.0/8,DIRECT
   DOMAIN-SUFFIX,google.com,PROXY
   DOMAIN-SUFFIX,youtube.com,PROXY
   FINAL,DIRECT
   ```

## Setup Instructions

### 1. Prerequisites
- macOS with Xcode 15+
- Apple Developer Account (paid, for Network Extension)
- Rust toolchain installed

### 2. Build Rust Library
```bash
cd voyage-core
chmod +x build-apple.sh
./build-apple.sh ios
```

### 3. Xcode Project Setup

1. Open `Voyage.xcodeproj` in Xcode
2. Select your development team for both targets
3. Update Bundle Identifiers:
   - App: `com.yourteam.voyage`
   - Extension: `com.yourteam.voyage.tunnel`

### 4. Configure Entitlements

Both targets need these capabilities:
- **Network Extensions** → Packet Tunnel
- **App Groups** → `group.com.voyage.app`

### 5. Link Rust Library

1. Add `libvoyage_core.a` to both targets
2. Add generated Swift files to both targets
3. Configure Header Search Paths and Library Search Paths

### 6. Run

1. Build and run on a physical iOS device (simulator doesn't support Network Extensions)
2. The app will prompt to add a VPN configuration
3. Enable VPN from the app or iOS Settings

## Memory Constraints

⚠️ **Critical**: iOS Network Extensions have a 15-50MB memory limit.

The Rust core is optimized for minimal memory usage:
- Static allocation where possible
- Connection limits
- Efficient buffer management

## Entitlements Required

### Voyage.entitlements
```xml
<key>com.apple.developer.networking.networkextension</key>
<array>
    <string>packet-tunnel-provider</string>
</array>
<key>com.apple.security.application-groups</key>
<array>
    <string>group.com.voyage.app</string>
</array>
```

### VoyageTunnel.entitlements
```xml
<key>com.apple.developer.networking.networkextension</key>
<array>
    <string>packet-tunnel-provider</string>
</array>
<key>com.apple.security.application-groups</key>
<array>
    <string>group.com.voyage.app</string>
</array>
```

## Data Flow

1. **App → Extension**: User taps Connect → `NETunnelProviderManager.startVPNTunnel()`
2. **Extension Starts**: `startTunnel()` called → Initialize Rust core
3. **Packet Flow**: 
   - iOS routes all traffic to UTUN interface
   - Extension reads packets with `readPackets()`
   - Packets processed by Rust core (NAT, rule matching, proxy routing)
   - Responses written back with `writePackets()`
4. **Statistics**: Extension periodically calls `getStats()` and shares via App Groups

## License

MIT License
