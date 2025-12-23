import SwiftUI
import NetworkExtension

struct ContentView: View {
    @StateObject private var vpnManager = VPNManager()
    @State private var serverHost: String = "127.0.0.1"
    @State private var serverPort: String = "1080"
    
    var body: some View {
        NavigationView {
            Form {
                Section(header: Text("VPN Status")) {
                    HStack {
                        Text("Status")
                        Spacer()
                        Text(vpnManager.statusText)
                            .foregroundColor(vpnManager.statusColor)
                    }
                    
                    Button(action: {
                        Task {
                            await vpnManager.toggleVPN()
                        }
                    }) {
                        HStack {
                            Spacer()
                            Text(vpnManager.isConnected ? "Disconnect" : "Connect")
                                .fontWeight(.semibold)
                            Spacer()
                        }
                    }
                    .disabled(vpnManager.isConnecting)
                }
                
                Section(header: Text("SOCKS5 Proxy Server")) {
                    TextField("Server Host", text: $serverHost)
                        .textContentType(.URL)
                        .autocapitalization(.none)
                    
                    TextField("Port", text: $serverPort)
                        .keyboardType(.numberPad)
                }
                
                Section(header: Text("Statistics")) {
                    HStack {
                        Text("Bytes Sent")
                        Spacer()
                        Text(formatBytes(vpnManager.stats.bytesSent))
                    }
                    HStack {
                        Text("Bytes Received")
                        Spacer()
                        Text(formatBytes(vpnManager.stats.bytesReceived))
                    }
                    HStack {
                        Text("Active Connections")
                        Spacer()
                        Text("\(vpnManager.stats.activeConnections)")
                    }
                }
            }
            .navigationTitle("Voyage")
        }
        .task {
            await vpnManager.loadFromPreferences()
        }
    }
    
    private func formatBytes(_ bytes: UInt64) -> String {
        let formatter = ByteCountFormatter()
        formatter.countStyle = .binary
        return formatter.string(fromByteCount: Int64(bytes))
    }
}

// Statistics model matching Rust CoreStats
struct VPNStats {
    var bytesSent: UInt64 = 0
    var bytesReceived: UInt64 = 0
    var activeConnections: UInt32 = 0
    var totalConnections: UInt32 = 0
}

@MainActor
class VPNManager: ObservableObject {
    @Published var isConnected: Bool = false
    @Published var isConnecting: Bool = false
    @Published var stats: VPNStats = VPNStats()
    
    private var manager: NETunnelProviderManager?
    
    var statusText: String {
        if isConnecting {
            return "Connecting..."
        } else if isConnected {
            return "Connected"
        } else {
            return "Disconnected"
        }
    }
    
    var statusColor: Color {
        if isConnecting {
            return .orange
        } else if isConnected {
            return .green
        } else {
            return .secondary
        }
    }
    
    func loadFromPreferences() async {
        do {
            let managers = try await NETunnelProviderManager.loadAllFromPreferences()
            if let existingManager = managers.first {
                self.manager = existingManager
                updateConnectionStatus()
            } else {
                // Create new manager
                let newManager = NETunnelProviderManager()
                let proto = NETunnelProviderProtocol()
                proto.providerBundleIdentifier = "com.voyage.app.tunnel"
                proto.serverAddress = "Voyage"
                newManager.protocolConfiguration = proto
                newManager.localizedDescription = "Voyage VPN"
                newManager.isEnabled = true
                
                try await newManager.saveToPreferences()
                try await newManager.loadFromPreferences()
                self.manager = newManager
            }
        } catch {
            print("Failed to load VPN preferences: \(error)")
        }
    }
    
    func toggleVPN() async {
        guard let manager = manager else { return }
        
        if isConnected {
            manager.connection.stopVPNTunnel()
            isConnected = false
        } else {
            isConnecting = true
            do {
                try manager.connection.startVPNTunnel()
                // Wait a bit for connection to establish
                try await Task.sleep(nanoseconds: 1_000_000_000)
                updateConnectionStatus()
            } catch {
                print("Failed to start VPN: \(error)")
            }
            isConnecting = false
        }
    }
    
    private func updateConnectionStatus() {
        guard let manager = manager else { return }
        switch manager.connection.status {
        case .connected:
            isConnected = true
        case .disconnected, .invalid:
            isConnected = false
        default:
            break
        }
    }
}

#Preview {
    ContentView()
}
