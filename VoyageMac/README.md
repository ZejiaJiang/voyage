# VoyageMac - macOS Application

A powerful network proxy application for macOS using Rust and SwiftUI.

## Project Structure

```
VoyageMac/
├── VoyageMac/                    # Main App Target
│   ├── VoyageMacApp.swift       # App entry point
│   ├── ContentView.swift        # Main window with sidebar navigation
│   ├── VPNManager.swift         # VPN connection management
│   ├── SettingsView.swift       # Settings window
│   ├── MenuBarView.swift        # Menu bar extra
│   ├── SystemExtensionManager.swift  # System Extension lifecycle
│   ├── Info.plist               # App configuration
│   └── VoyageMac.entitlements   # App entitlements
│
└── VoyageTunnel/                 # System Extension Target
    ├── PacketTunnelProvider.swift  # Network Extension provider
    ├── Info.plist                  # Extension configuration
    └── VoyageTunnel.entitlements   # Extension entitlements
```

## Requirements

- macOS 12.0 (Monterey) or later
- Xcode 14.0 or later
- Apple Developer Account (for System Extension signing)
- Rust toolchain (for building voyage-core)

## Setup in Xcode

### 1. Create New Xcode Project

1. Open Xcode
2. File → New → Project
3. Select **macOS** → **App**
4. Product Name: `VoyageMac`
5. Bundle Identifier: `com.voyage.mac`
6. Interface: **SwiftUI**
7. Language: **Swift**

### 2. Add System Extension Target

1. File → New → Target
2. Select **macOS** → **System Extension**
3. Product Name: `VoyageTunnel`
4. Bundle Identifier: `com.voyage.mac.tunnel`
5. Extension Type: **Network Extension**

### 3. Configure Capabilities

#### Main App (VoyageMac)
1. Select VoyageMac target → Signing & Capabilities
2. Add capabilities:
   - **Network Extensions** → Enable "Packet Tunnel"
   - **System Extension** (for installing the tunnel)
   - **App Groups** → Add `group.com.voyage.mac`
   - **App Sandbox** → Enable with network client/server

#### System Extension (VoyageTunnel)
1. Select VoyageTunnel target → Signing & Capabilities
2. Add capabilities:
   - **Network Extensions** → Enable "Packet Tunnel Provider (System Extension)"
   - **App Groups** → Add `group.com.voyage.mac`

### 4. Build Rust Library

```bash
cd voyage-core

# Build for macOS
make build-universal

# Generate Swift bindings
make generate-bindings
```

### 5. Link Rust Library

1. Drag `voyage-core/target/universal/release/libvoyage_core.a` into Xcode
2. Add to both VoyageMac and VoyageTunnel targets
3. Add `voyage-core/generated/swift/voyage_core.swift` to both targets
4. Build Settings → Header Search Paths:
   - Add `$(PROJECT_DIR)/../voyage-core/generated/include`
5. Build Settings → Library Search Paths:
   - Add `$(PROJECT_DIR)/../voyage-core/target/universal/release`

### 6. Add Dependencies

In both targets, add these system frameworks:
- Network.framework
- NetworkExtension.framework
- SystemExtensions.framework (main app only)

## Build and Run

1. Build the Rust library first (see step 4)
2. Open `VoyageMac.xcodeproj` in Xcode
3. Select the VoyageMac scheme
4. Build and Run (⌘R)

## System Extension Approval

On first launch, users need to approve the system extension:

1. A notification will appear requesting approval
2. Open **System Preferences** → **Privacy & Security**
3. Click "Allow" for the Voyage extension
4. The VPN functionality will then be available

## Features

- **Dashboard**: Connection status, statistics, quick connect/disconnect
- **Connections**: Real-time view of active network connections
- **Rules**: Surge-style routing rules editor
- **Logs**: Real-time log viewer with filtering
- **Menu Bar**: Quick access from the menu bar
- **Settings**: Proxy configuration, DNS settings, and more

## Development Notes

### Debugging System Extensions

System Extensions cannot be debugged directly. Use logging:

```swift
import os.log
let logger = Logger(subsystem: "com.voyage.mac.tunnel", category: "Debug")
logger.debug("Debug message")
```

View logs in Console.app with filter: `subsystem:com.voyage.mac`

### Replacing System Extension During Development

1. Stop the VPN if running
2. Build and run the new version
3. The old extension will be replaced automatically

### Testing Without System Extension

For UI development, you can test without the actual VPN:
- The `VPNManager` simulates stats updates when "connected"
- Comment out `SystemExtensionManager` calls during UI testing

## Troubleshooting

### "System Extension Blocked"
- Go to System Preferences → Privacy & Security
- Look for a "Allow" button for blocked extensions

### "Extension Not Found"
- Ensure the System Extension is embedded in the app bundle
- Check that bundle identifiers match

### "Activation Failed"
- Check Console.app for detailed error messages
- Ensure proper code signing with appropriate entitlements
