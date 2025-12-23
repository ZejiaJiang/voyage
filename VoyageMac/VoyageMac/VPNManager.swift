import SwiftUI
import NetworkExtension

/// Statistics model matching Rust CoreStats
struct VPNStats {
    var bytesSent: UInt64 = 0
    var bytesReceived: UInt64 = 0
    var activeConnections: UInt32 = 0
    var totalConnections: UInt32 = 0
}

/// Connection info for display
struct ConnectionInfo: Identifiable {
    let id = UUID()
    let sourceAddress: String
    let destAddress: String
    let protocolType: String
    let route: String
    var bytesTransferred: UInt64
}

@MainActor
class VPNManager: ObservableObject {
    @Published var isConnected: Bool = false
    @Published var isConnecting: Bool = false
    @Published var stats: VPNStats = VPNStats()
    @Published var connections: [ConnectionInfo] = []
    @Published var serverHost: String = "127.0.0.1"
    @Published var serverPort: UInt16 = 1080
    @Published var coreVersion: String = "0.1.0"
    
    private var manager: NETunnelProviderManager?
    private var statusObserver: NSObjectProtocol?
    private var statsTimer: Timer?
    
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
            return .gray
        }
    }
    
    deinit {
        if let observer = statusObserver {
            NotificationCenter.default.removeObserver(observer)
        }
        statsTimer?.invalidate()
    }
    
    func loadFromPreferences() async {
        do {
            let managers = try await NETunnelProviderManager.loadAllFromPreferences()
            
            if let existingManager = managers.first {
                self.manager = existingManager
                observeStatusChanges()
                updateConnectionStatus()
            } else {
                // Create new manager
                let newManager = NETunnelProviderManager()
                let proto = NETunnelProviderProtocol()
                
                // Configure the tunnel provider
                proto.providerBundleIdentifier = "com.voyage.mac.tunnel"
                proto.serverAddress = "Voyage"
                proto.providerConfiguration = [
                    "serverHost": serverHost,
                    "serverPort": serverPort
                ]
                
                newManager.protocolConfiguration = proto
                newManager.localizedDescription = "Voyage VPN"
                newManager.isEnabled = true
                
                try await newManager.saveToPreferences()
                try await newManager.loadFromPreferences()
                self.manager = newManager
                observeStatusChanges()
            }
            
            // Try to get core version
            // coreVersion = getCoreVersion()
        } catch {
            print("Failed to load VPN preferences: \(error)")
        }
    }
    
    private func observeStatusChanges() {
        guard let manager = manager else { return }
        
        statusObserver = NotificationCenter.default.addObserver(
            forName: .NEVPNStatusDidChange,
            object: manager.connection,
            queue: .main
        ) { [weak self] _ in
            self?.updateConnectionStatus()
        }
    }
    
    func toggleVPN() async {
        guard let manager = manager else { return }
        
        if isConnected {
            manager.connection.stopVPNTunnel()
        } else {
            isConnecting = true
            do {
                // Update configuration before connecting
                if let proto = manager.protocolConfiguration as? NETunnelProviderProtocol {
                    proto.providerConfiguration = [
                        "serverHost": serverHost,
                        "serverPort": serverPort
                    ]
                    try await manager.saveToPreferences()
                }
                
                try manager.connection.startVPNTunnel()
                startStatsTimer()
            } catch {
                print("Failed to start VPN: \(error)")
                isConnecting = false
            }
        }
    }
    
    private func updateConnectionStatus() {
        guard let manager = manager else { return }
        
        switch manager.connection.status {
        case .connected:
            isConnected = true
            isConnecting = false
            startStatsTimer()
        case .connecting, .reasserting:
            isConnecting = true
            isConnected = false
        case .disconnected, .invalid:
            isConnected = false
            isConnecting = false
            stopStatsTimer()
        case .disconnecting:
            isConnecting = true
        @unknown default:
            break
        }
    }
    
    private func startStatsTimer() {
        statsTimer?.invalidate()
        statsTimer = Timer.scheduledTimer(withTimeInterval: 1.0, repeats: true) { [weak self] _ in
            Task { @MainActor in
                self?.refreshStats()
            }
        }
    }
    
    private func stopStatsTimer() {
        statsTimer?.invalidate()
        statsTimer = nil
    }
    
    private func refreshStats() {
        // TODO: Get stats from Rust core via IPC
        // This would communicate with the System Extension
        // For now, simulate some data
        if isConnected {
            stats.bytesSent += UInt64.random(in: 100...10000)
            stats.bytesReceived += UInt64.random(in: 500...50000)
            stats.activeConnections = UInt32.random(in: 0...10)
            stats.totalConnections += UInt32.random(in: 0...1)
        }
    }
    
    func updateConfiguration(host: String, port: UInt16) async {
        serverHost = host
        serverPort = port
        
        guard let manager = manager,
              let proto = manager.protocolConfiguration as? NETunnelProviderProtocol else {
            return
        }
        
        proto.providerConfiguration = [
            "serverHost": host,
            "serverPort": port
        ]
        
        do {
            try await manager.saveToPreferences()
        } catch {
            print("Failed to save configuration: \(error)")
        }
    }
}
