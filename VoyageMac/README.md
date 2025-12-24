# VoyageMac - macOS Proxy Application

A powerful Surge-like network proxy application for macOS using SwiftUI and the Voyage Rust core.

## Overview

VoyageMac is a native macOS application that provides VPN/proxy functionality through a System Extension. It features a full SwiftUI interface with menu bar integration, real-time connection monitoring, and a powerful rule engine.

## Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    VoyageMac App (SwiftUI)                      │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                    Main Window                            │   │
│  │  ┌─────────┐ ┌──────────────────────────────────────┐   │   │
│  │  │ Sidebar │ │          Content Area                 │   │   │
│  │  │         │ │  • Dashboard (stats, graphs)          │   │   │
│  │  │ • Dash  │ │  • Connections (active list)          │   │   │
│  │  │ • Conn  │ │  • Rules (editor, import/export)      │   │   │
│  │  │ • Rules │ │  • Logs (filtered, searchable)        │   │   │
│  │  │ • Logs  │ │  • Settings (proxy config)            │   │   │
│  │  └─────────┘ └──────────────────────────────────────┘   │   │
│  └─────────────────────────────────────────────────────────┘   │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │                   Menu Bar Extra                          │   │
│  │  • Quick connect/disconnect                               │   │
│  │  • Status indicator                                       │   │
│  │  • Mini stats display                                     │   │
│  └─────────────────────────────────────────────────────────┘   │
└───────────────────────────┬─────────────────────────────────────┘
                            │ SystemExtensions.framework
                            ▼
┌─────────────────────────────────────────────────────────────────┐
│              VoyageTunnel (System Extension)                    │
│  ┌─────────────────────────────────────────────────────────┐   │
│  │            NEPacketTunnelProvider                         │   │
│  │  • Intercepts all network traffic                         │   │
│  │  • Routes through Rust core                               │   │
│  │  • Applies routing rules                                  │   │
│  └─────────────────────────────────────────────────────────┘   │
│                            │ UniFFI                             │
│                            ▼                                    │
│                   voyage-core (Rust)                            │
└─────────────────────────────────────────────────────────────────┘
```

## Project Structure

```
VoyageMac/
├── README.md                        # This file
│
├── VoyageMac/                       # Main App Target
│   ├── VoyageMacApp.swift          # App entry point with MenuBarExtra
│   ├── ContentView.swift           # Main window with sidebar navigation
│   ├── VPNManager.swift            # VPN connection management
│   ├── SettingsView.swift          # Settings panel
│   ├── MenuBarView.swift           # Menu bar dropdown UI
│   ├── SystemExtensionManager.swift # System Extension lifecycle
│   ├── Info.plist                  # App configuration
│   ├── VoyageMac.entitlements      # App entitlements
│   └── Assets.xcassets/            # App icons and assets
│
└── VoyageTunnel/                    # System Extension Target
    ├── PacketTunnelProvider.swift  # NEPacketTunnelProvider subclass
    ├── Info.plist                  # Extension configuration
    └── VoyageTunnel.entitlements   # Extension entitlements
```

## Components

### Main App (`VoyageMac/`)

#### VoyageMacApp.swift
App entry point using `@main`:
- **WindowGroup**: Main application window
- **MenuBarExtra**: System tray/menu bar item with quick controls
- **Settings Scene**: Preferences window (⌘,)

#### ContentView.swift
Main window with sidebar navigation:
- **Dashboard**: Real-time statistics, connection graphs
- **Connections**: List of active/recent connections with details
- **Rules**: Rule editor with syntax highlighting, import/export
- **Logs**: Filterable log viewer with search
- **Settings**: Proxy server configuration

#### VPNManager.swift
`@MainActor ObservableObject` managing VPN state:

| Property | Type | Description |
|----------|------|-------------|
| `isConnected` | `Bool` | Current connection state |
| `isConnecting` | `Bool` | Connection in progress |
| `stats` | `VPNStats` | Traffic statistics |
| `connections` | `[ConnectionInfo]` | Active connections |

| Method | Description |
|--------|-------------|
| `loadFromPreferences()` | Load saved VPN configuration |
| `toggleVPN()` | Connect or disconnect |
| `saveConfiguration()` | Persist settings to preferences |

#### SystemExtensionManager.swift
Handles System Extension lifecycle:
- **Installation**: Request user approval to install extension
- **Activation**: Enable the packet tunnel provider
- **Updates**: Handle extension version updates
- **Removal**: Uninstall extension when needed

Key methods:
- `installExtension()` - Submits activation request
- `handleRequest(_:)` - OSSystemExtensionRequestDelegate callbacks

#### MenuBarView.swift
Menu bar dropdown interface:
- Connection status with colored indicator
- Quick connect/disconnect button
- Mini stats (bytes sent/received)
- Quit application option

#### SettingsView.swift
Preferences window with tabs:
- **General**: Server host, port configuration
- **Authentication**: Username/password for SOCKS5
- **Rules**: Default rule behavior
- **Advanced**: Debug options, logging level

### System Extension (`VoyageTunnel/`)

#### PacketTunnelProvider.swift
Same responsibilities as iOS version:
1. Initialize Rust core on tunnel start
2. Configure network settings (DNS, routes)
3. Run packet processing loop
4. Apply routing rules via Rust core
5. Collect and report statistics

## Key Features

### 1. Menu Bar Integration
```swift
MenuBarExtra("Voyage", systemImage: "network") {
    MenuBarView()
}
.menuBarExtraStyle(.window)
```
- Always-accessible status indicator
- Quick toggle without opening main window
- Real-time traffic display

### 2. Sidebar Navigation
```swift
NavigationSplitView {
    List(selection: $selectedTab) {
        Label("Dashboard", systemImage: "gauge")
        Label("Connections", systemImage: "link")
        Label("Rules", systemImage: "list.bullet")
        Label("Logs", systemImage: "doc.text")
    }
} detail: {
    // Content based on selection
}
```

### 3. System Extension Management
Unlike iOS Network Extensions, macOS requires explicit user approval:
1. App requests extension installation
2. System shows approval dialog
3. User must go to System Settings → Privacy & Security
4. Click "Allow" for the extension

### 4. Rule Engine Integration
Full Surge-style rule support:
```
DOMAIN-SUFFIX,google.com,PROXY
IP-CIDR,10.0.0.0/8,DIRECT
GEOIP,CN,DIRECT
FINAL,PROXY
```

## Setup Instructions

### 1. Prerequisites
- macOS 12.0 (Monterey) or later
- Xcode 14.0+
- Apple Developer Account
- Rust toolchain

### 2. Build Rust Library
```bash
cd voyage-core
chmod +x build-apple.sh
./build-apple.sh macos
```

This creates:
- `target/aarch64-apple-darwin/release/libvoyage_core.a` (Apple Silicon)
- `target/x86_64-apple-darwin/release/libvoyage_core.a` (Intel)
- `target/universal-macos/release/libvoyage_core.a` (Universal)

### 3. Xcode Project Setup

1. Open `VoyageMac.xcodeproj`
2. Configure signing for both targets
3. Update bundle identifiers:
   - App: `com.yourteam.voyagemac`
   - Extension: `com.yourteam.voyagemac.tunnel`

### 4. Configure Capabilities

#### Main App
- **Network Extensions** → Packet Tunnel
- **System Extension**
- **App Groups** → `group.com.voyage.mac`
- **App Sandbox** → Network Client/Server

#### System Extension
- **Network Extensions** → Packet Tunnel Provider (System Extension)
- **App Groups** → `group.com.voyage.mac`

### 5. Link Rust Library

1. Add `libvoyage_core.a` (universal) to both targets
2. Add generated `voyage_core.swift` to both targets
3. Build Settings:
   - Header Search Paths: `$(PROJECT_DIR)/../voyage-core/generated/include`
   - Library Search Paths: `$(PROJECT_DIR)/../voyage-core/target/universal-macos/release`

### 6. Run

1. Build and run from Xcode
2. On first run, approve the System Extension in System Settings
3. Configure proxy server settings
4. Click Connect

## Differences from iOS Version

| Aspect | iOS | macOS |
|--------|-----|-------|
| Extension Type | Network Extension | System Extension |
| Approval | Automatic with profile | Manual in System Settings |
| Memory Limit | 15-50MB | None (system process) |
| UI | Single view | Multi-window, menu bar |
| Distribution | App Store only | App Store or direct |

## Entitlements

### VoyageMac.entitlements
```xml
<key>com.apple.developer.networking.networkextension</key>
<array>
    <string>packet-tunnel-provider</string>
</array>
<key>com.apple.developer.system-extension.install</key>
<true/>
<key>com.apple.security.application-groups</key>
<array>
    <string>group.com.voyage.mac</string>
</array>
<key>com.apple.security.app-sandbox</key>
<true/>
<key>com.apple.security.network.client</key>
<true/>
<key>com.apple.security.network.server</key>
<true/>
```

### VoyageTunnel.entitlements
```xml
<key>com.apple.developer.networking.networkextension</key>
<array>
    <string>packet-tunnel-provider-systemextension</string>
</array>
<key>com.apple.security.application-groups</key>
<array>
    <string>group.com.voyage.mac</string>
</array>
```

## Debugging

### System Extension Logs
```bash
log stream --predicate 'subsystem == "com.voyage.mac.tunnel"'
```

### Check Extension Status
```bash
systemextensionsctl list
```

### Reset Extension (Development)
```bash
systemextensionsctl uninstall <team-id> com.voyage.mac.tunnel
```

## License

MIT License
