import SwiftUI
import NetworkExtension

struct ContentView: View {
    @StateObject private var vpnManager = VPNManager()
    @State private var selectedTab = 0
    
    var body: some View {
        NavigationSplitView {
            // Sidebar
            List(selection: $selectedTab) {
                NavigationLink(value: 0) {
                    Label("Dashboard", systemImage: "gauge")
                }
                NavigationLink(value: 1) {
                    Label("Connections", systemImage: "network")
                }
                NavigationLink(value: 2) {
                    Label("Rules", systemImage: "list.bullet.rectangle")
                }
                NavigationLink(value: 3) {
                    Label("Logs", systemImage: "doc.text")
                }
            }
            .listStyle(.sidebar)
            .frame(minWidth: 180)
        } detail: {
            switch selectedTab {
            case 0:
                DashboardView(vpnManager: vpnManager)
            case 1:
                ConnectionsView(vpnManager: vpnManager)
            case 2:
                RulesView()
            case 3:
                LogsView()
            default:
                DashboardView(vpnManager: vpnManager)
            }
        }
        .frame(minWidth: 800, minHeight: 500)
        .task {
            await vpnManager.loadFromPreferences()
        }
    }
}

// MARK: - Dashboard View

struct DashboardView: View {
    @ObservedObject var vpnManager: VPNManager
    
    var body: some View {
        ScrollView {
            VStack(spacing: 20) {
                // Status Card
                GroupBox {
                    VStack(spacing: 16) {
                        HStack {
                            Circle()
                                .fill(vpnManager.statusColor)
                                .frame(width: 12, height: 12)
                            Text(vpnManager.statusText)
                                .font(.headline)
                            Spacer()
                        }
                        
                        Button(action: {
                            Task {
                                await vpnManager.toggleVPN()
                            }
                        }) {
                            HStack {
                                Image(systemName: vpnManager.isConnected ? "stop.fill" : "play.fill")
                                Text(vpnManager.isConnected ? "Disconnect" : "Connect")
                            }
                            .frame(maxWidth: .infinity)
                            .padding(.vertical, 8)
                        }
                        .buttonStyle(.borderedProminent)
                        .tint(vpnManager.isConnected ? .red : .accentColor)
                        .disabled(vpnManager.isConnecting)
                    }
                    .padding()
                } label: {
                    Label("VPN Status", systemImage: "shield.checkered")
                }
                
                // Statistics Grid
                LazyVGrid(columns: [
                    GridItem(.flexible()),
                    GridItem(.flexible())
                ], spacing: 16) {
                    StatCard(
                        title: "Upload",
                        value: formatBytes(vpnManager.stats.bytesSent),
                        icon: "arrow.up.circle.fill",
                        color: .blue
                    )
                    StatCard(
                        title: "Download",
                        value: formatBytes(vpnManager.stats.bytesReceived),
                        icon: "arrow.down.circle.fill",
                        color: .green
                    )
                    StatCard(
                        title: "Active Connections",
                        value: "\(vpnManager.stats.activeConnections)",
                        icon: "link.circle.fill",
                        color: .orange
                    )
                    StatCard(
                        title: "Total Connections",
                        value: "\(vpnManager.stats.totalConnections)",
                        icon: "chart.line.uptrend.xyaxis.circle.fill",
                        color: .purple
                    )
                }
                
                // Proxy Server Configuration
                GroupBox {
                    VStack(alignment: .leading, spacing: 12) {
                        HStack {
                            Text("Server:")
                                .foregroundColor(.secondary)
                            Text(vpnManager.serverHost)
                                .fontWeight(.medium)
                            Text(":")
                                .foregroundColor(.secondary)
                            Text("\(vpnManager.serverPort)")
                                .fontWeight(.medium)
                        }
                        
                        HStack {
                            Text("Protocol:")
                                .foregroundColor(.secondary)
                            Text("SOCKS5")
                                .fontWeight(.medium)
                        }
                        
                        HStack {
                            Text("Core Version:")
                                .foregroundColor(.secondary)
                            Text(vpnManager.coreVersion)
                                .fontWeight(.medium)
                        }
                    }
                    .padding()
                } label: {
                    Label("Proxy Configuration", systemImage: "server.rack")
                }
                
                Spacer()
            }
            .padding()
        }
        .navigationTitle("Dashboard")
    }
    
    private func formatBytes(_ bytes: UInt64) -> String {
        let formatter = ByteCountFormatter()
        formatter.countStyle = .binary
        return formatter.string(fromByteCount: Int64(bytes))
    }
}

struct StatCard: View {
    let title: String
    let value: String
    let icon: String
    let color: Color
    
    var body: some View {
        GroupBox {
            VStack(alignment: .leading, spacing: 8) {
                HStack {
                    Image(systemName: icon)
                        .foregroundColor(color)
                        .font(.title2)
                    Spacer()
                }
                Text(value)
                    .font(.title)
                    .fontWeight(.bold)
                Text(title)
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
            .padding()
        }
    }
}

// MARK: - Connections View

struct ConnectionsView: View {
    @ObservedObject var vpnManager: VPNManager
    
    var body: some View {
        VStack {
            if vpnManager.connections.isEmpty {
                ContentUnavailableView(
                    "No Active Connections",
                    systemImage: "network.slash",
                    description: Text("Connections will appear here when the VPN is active.")
                )
            } else {
                Table(vpnManager.connections) {
                    TableColumn("Source") { conn in
                        Text(conn.sourceAddress)
                            .font(.system(.body, design: .monospaced))
                    }
                    TableColumn("Destination") { conn in
                        Text(conn.destAddress)
                            .font(.system(.body, design: .monospaced))
                    }
                    TableColumn("Protocol") { conn in
                        Text(conn.protocolType)
                            .font(.caption)
                            .padding(.horizontal, 6)
                            .padding(.vertical, 2)
                            .background(conn.protocolType == "TCP" ? Color.blue.opacity(0.2) : Color.green.opacity(0.2))
                            .cornerRadius(4)
                    }
                    TableColumn("Route") { conn in
                        HStack {
                            Circle()
                                .fill(conn.route == "PROXY" ? Color.orange : Color.green)
                                .frame(width: 8, height: 8)
                            Text(conn.route)
                        }
                    }
                    TableColumn("Bytes") { conn in
                        Text(formatBytes(conn.bytesTransferred))
                    }
                }
            }
        }
        .navigationTitle("Connections")
    }
    
    private func formatBytes(_ bytes: UInt64) -> String {
        let formatter = ByteCountFormatter()
        formatter.countStyle = .binary
        return formatter.string(fromByteCount: Int64(bytes))
    }
}

// MARK: - Rules View

struct RulesView: View {
    @State private var rulesText: String = """
    # Voyage Routing Rules
    # Format: TYPE,VALUE,ACTION
    
    # Local networks - Direct
    IP-CIDR,127.0.0.0/8,DIRECT
    IP-CIDR,192.168.0.0/16,DIRECT
    IP-CIDR,10.0.0.0/8,DIRECT
    IP-CIDR,172.16.0.0/12,DIRECT
    
    # Common proxy targets
    DOMAIN-SUFFIX,google.com,PROXY
    DOMAIN-SUFFIX,youtube.com,PROXY
    DOMAIN-SUFFIX,twitter.com,PROXY
    DOMAIN-SUFFIX,facebook.com,PROXY
    DOMAIN-SUFFIX,github.com,PROXY
    
    # Default action
    FINAL,DIRECT
    """
    @State private var rulesCount: Int = 0
    @State private var showingSaveAlert = false
    
    var body: some View {
        VStack(spacing: 0) {
            // Editor
            TextEditor(text: $rulesText)
                .font(.system(.body, design: .monospaced))
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            
            Divider()
            
            // Toolbar
            HStack {
                Text("\(rulesCount) rules loaded")
                    .foregroundColor(.secondary)
                
                Spacer()
                
                Button("Reset to Default") {
                    rulesText = getDefaultRules()
                }
                
                Button("Apply Rules") {
                    applyRules()
                }
                .buttonStyle(.borderedProminent)
            }
            .padding()
        }
        .navigationTitle("Rules")
        .alert("Rules Applied", isPresented: $showingSaveAlert) {
            Button("OK", role: .cancel) {}
        } message: {
            Text("\(rulesCount) rules have been loaded successfully.")
        }
    }
    
    private func applyRules() {
        // TODO: Call Rust FFI to load rules
        // rulesCount = Int(loadRules(rulesText: rulesText))
        rulesCount = rulesText.components(separatedBy: .newlines)
            .filter { !$0.trimmingCharacters(in: .whitespaces).isEmpty && !$0.hasPrefix("#") }
            .count
        showingSaveAlert = true
    }
    
    private func getDefaultRules() -> String {
        return """
        # Voyage Routing Rules
        # Format: TYPE,VALUE,ACTION
        
        # Local networks - Direct
        IP-CIDR,127.0.0.0/8,DIRECT
        IP-CIDR,192.168.0.0/16,DIRECT
        IP-CIDR,10.0.0.0/8,DIRECT
        IP-CIDR,172.16.0.0/12,DIRECT
        
        # Common proxy targets
        DOMAIN-SUFFIX,google.com,PROXY
        DOMAIN-SUFFIX,youtube.com,PROXY
        DOMAIN-SUFFIX,twitter.com,PROXY
        DOMAIN-SUFFIX,facebook.com,PROXY
        DOMAIN-SUFFIX,github.com,PROXY
        
        # Default action
        FINAL,DIRECT
        """
    }
}

// MARK: - Logs View

struct LogsView: View {
    @State private var logs: [LogEntry] = []
    @State private var filterLevel: LogLevel = .all
    
    var body: some View {
        VStack(spacing: 0) {
            // Filter bar
            HStack {
                Picker("Level", selection: $filterLevel) {
                    Text("All").tag(LogLevel.all)
                    Text("Debug").tag(LogLevel.debug)
                    Text("Info").tag(LogLevel.info)
                    Text("Warning").tag(LogLevel.warning)
                    Text("Error").tag(LogLevel.error)
                }
                .pickerStyle(.segmented)
                .frame(maxWidth: 400)
                
                Spacer()
                
                Button(action: { logs.removeAll() }) {
                    Image(systemName: "trash")
                }
            }
            .padding()
            
            Divider()
            
            // Log list
            if logs.isEmpty {
                ContentUnavailableView(
                    "No Logs",
                    systemImage: "doc.text",
                    description: Text("Logs will appear here when the VPN is running.")
                )
            } else {
                List(filteredLogs) { log in
                    HStack(alignment: .top) {
                        Image(systemName: log.level.icon)
                            .foregroundColor(log.level.color)
                            .frame(width: 20)
                        
                        VStack(alignment: .leading, spacing: 2) {
                            Text(log.message)
                                .font(.system(.body, design: .monospaced))
                            Text(log.timestamp, style: .time)
                                .font(.caption)
                                .foregroundColor(.secondary)
                        }
                    }
                }
            }
        }
        .navigationTitle("Logs")
    }
    
    private var filteredLogs: [LogEntry] {
        if filterLevel == .all {
            return logs
        }
        return logs.filter { $0.level == filterLevel }
    }
}

struct LogEntry: Identifiable {
    let id = UUID()
    let timestamp: Date
    let level: LogLevel
    let message: String
}

enum LogLevel: String, CaseIterable {
    case all, debug, info, warning, error
    
    var icon: String {
        switch self {
        case .all: return "line.3.horizontal"
        case .debug: return "ant"
        case .info: return "info.circle"
        case .warning: return "exclamationmark.triangle"
        case .error: return "xmark.circle"
        }
    }
    
    var color: Color {
        switch self {
        case .all: return .primary
        case .debug: return .gray
        case .info: return .blue
        case .warning: return .orange
        case .error: return .red
        }
    }
}

// MARK: - Preview

#Preview {
    ContentView()
}
