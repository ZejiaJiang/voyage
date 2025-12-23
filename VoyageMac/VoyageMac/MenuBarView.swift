import SwiftUI

struct MenuBarView: View {
    @StateObject private var vpnManager = VPNManager()
    
    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            // Status Header
            HStack {
                Circle()
                    .fill(vpnManager.statusColor)
                    .frame(width: 8, height: 8)
                Text(vpnManager.statusText)
                    .font(.headline)
                Spacer()
            }
            .padding()
            
            Divider()
            
            // Quick Stats
            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    Image(systemName: "arrow.up")
                        .foregroundColor(.blue)
                    Text("Upload:")
                    Spacer()
                    Text(formatBytes(vpnManager.stats.bytesSent))
                        .foregroundColor(.secondary)
                }
                
                HStack {
                    Image(systemName: "arrow.down")
                        .foregroundColor(.green)
                    Text("Download:")
                    Spacer()
                    Text(formatBytes(vpnManager.stats.bytesReceived))
                        .foregroundColor(.secondary)
                }
                
                HStack {
                    Image(systemName: "link")
                        .foregroundColor(.orange)
                    Text("Connections:")
                    Spacer()
                    Text("\(vpnManager.stats.activeConnections)")
                        .foregroundColor(.secondary)
                }
            }
            .padding()
            .font(.system(.body, design: .monospaced))
            
            Divider()
            
            // Connect/Disconnect Button
            Button(action: {
                Task {
                    await vpnManager.toggleVPN()
                }
            }) {
                HStack {
                    Image(systemName: vpnManager.isConnected ? "stop.fill" : "play.fill")
                    Text(vpnManager.isConnected ? "Disconnect" : "Connect")
                    Spacer()
                }
            }
            .buttonStyle(.borderless)
            .padding(.horizontal)
            .padding(.vertical, 8)
            .disabled(vpnManager.isConnecting)
            
            Divider()
            
            // Menu Items
            Button(action: openMainWindow) {
                HStack {
                    Image(systemName: "macwindow")
                    Text("Open Voyage")
                    Spacer()
                    Text("⌘O")
                        .foregroundColor(.secondary)
                }
            }
            .buttonStyle(.borderless)
            .padding(.horizontal)
            .padding(.vertical, 6)
            
            Button(action: openSettings) {
                HStack {
                    Image(systemName: "gear")
                    Text("Settings...")
                    Spacer()
                    Text("⌘,")
                        .foregroundColor(.secondary)
                }
            }
            .buttonStyle(.borderless)
            .padding(.horizontal)
            .padding(.vertical, 6)
            
            Divider()
            
            Button(action: quitApp) {
                HStack {
                    Image(systemName: "power")
                    Text("Quit Voyage")
                    Spacer()
                    Text("⌘Q")
                        .foregroundColor(.secondary)
                }
            }
            .buttonStyle(.borderless)
            .padding(.horizontal)
            .padding(.vertical, 6)
        }
        .frame(width: 280)
        .task {
            await vpnManager.loadFromPreferences()
        }
    }
    
    private func formatBytes(_ bytes: UInt64) -> String {
        let formatter = ByteCountFormatter()
        formatter.countStyle = .binary
        return formatter.string(fromByteCount: Int64(bytes))
    }
    
    private func openMainWindow() {
        NSApp.activate(ignoringOtherApps: true)
        if let window = NSApp.windows.first(where: { $0.title.isEmpty == false }) {
            window.makeKeyAndOrderFront(nil)
        } else {
            // Open a new window if none exists
            NSApp.sendAction(Selector(("showMainWindow:")), to: nil, from: nil)
        }
    }
    
    private func openSettings() {
        NSApp.activate(ignoringOtherApps: true)
        NSApp.sendAction(Selector(("showSettingsWindow:")), to: nil, from: nil)
    }
    
    private func quitApp() {
        NSApplication.shared.terminate(nil)
    }
}

#Preview {
    MenuBarView()
}
