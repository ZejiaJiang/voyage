import SwiftUI

struct SettingsView: View {
    @StateObject private var settings = SettingsManager()
    
    var body: some View {
        TabView {
            GeneralSettingsView(settings: settings)
                .tabItem {
                    Label("General", systemImage: "gear")
                }
            
            ProxySettingsView(settings: settings)
                .tabItem {
                    Label("Proxy", systemImage: "server.rack")
                }
            
            AdvancedSettingsView(settings: settings)
                .tabItem {
                    Label("Advanced", systemImage: "slider.horizontal.3")
                }
            
            AboutView()
                .tabItem {
                    Label("About", systemImage: "info.circle")
                }
        }
        .frame(width: 500, height: 350)
    }
}

// MARK: - General Settings

struct GeneralSettingsView: View {
    @ObservedObject var settings: SettingsManager
    
    var body: some View {
        Form {
            Section {
                Toggle("Launch at Login", isOn: $settings.launchAtLogin)
                Toggle("Show in Menu Bar", isOn: $settings.showInMenuBar)
                Toggle("Show Notifications", isOn: $settings.showNotifications)
            }
            
            Section {
                Toggle("Auto-connect on Launch", isOn: $settings.autoConnect)
                Toggle("Reconnect on Network Change", isOn: $settings.reconnectOnNetworkChange)
            }
        }
        .formStyle(.grouped)
        .padding()
    }
}

// MARK: - Proxy Settings

struct ProxySettingsView: View {
    @ObservedObject var settings: SettingsManager
    
    var body: some View {
        Form {
            Section("SOCKS5 Proxy Server") {
                TextField("Server Address", text: $settings.proxyHost)
                TextField("Port", value: $settings.proxyPort, format: .number)
                Toggle("Authentication Required", isOn: $settings.proxyAuthEnabled)
                
                if settings.proxyAuthEnabled {
                    TextField("Username", text: $settings.proxyUsername)
                    SecureField("Password", text: $settings.proxyPassword)
                }
            }
            
            Section {
                Button("Test Connection") {
                    testConnection()
                }
            }
        }
        .formStyle(.grouped)
        .padding()
    }
    
    private func testConnection() {
        // TODO: Implement connection test
    }
}

// MARK: - Advanced Settings

struct AdvancedSettingsView: View {
    @ObservedObject var settings: SettingsManager
    
    var body: some View {
        Form {
            Section("Network") {
                Picker("DNS Mode", selection: $settings.dnsMode) {
                    Text("System").tag("system")
                    Text("Remote").tag("remote")
                    Text("Fake IP").tag("fakeip")
                }
                
                TextField("Custom DNS", text: $settings.customDns)
                    .disabled(settings.dnsMode == "system")
            }
            
            Section("Performance") {
                Stepper("Max Connections: \(settings.maxConnections)", 
                        value: $settings.maxConnections, 
                        in: 10...500, 
                        step: 10)
                
                Toggle("TCP Fast Open", isOn: $settings.tcpFastOpen)
                Toggle("IPv6 Support", isOn: $settings.ipv6Enabled)
            }
            
            Section("Logging") {
                Picker("Log Level", selection: $settings.logLevel) {
                    Text("Error").tag("error")
                    Text("Warning").tag("warning")
                    Text("Info").tag("info")
                    Text("Debug").tag("debug")
                }
                
                Button("Open Log Folder") {
                    openLogFolder()
                }
            }
        }
        .formStyle(.grouped)
        .padding()
    }
    
    private func openLogFolder() {
        if let logPath = FileManager.default.urls(for: .libraryDirectory, in: .userDomainMask).first?
            .appendingPathComponent("Logs/Voyage") {
            NSWorkspace.shared.open(logPath)
        }
    }
}

// MARK: - About View

struct AboutView: View {
    var body: some View {
        VStack(spacing: 20) {
            Image(systemName: "globe.americas.fill")
                .font(.system(size: 64))
                .foregroundStyle(.blue.gradient)
            
            Text("Voyage")
                .font(.largeTitle)
                .fontWeight(.bold)
            
            Text("Version 1.0.0 (Rust Core 0.1.0)")
                .foregroundColor(.secondary)
            
            Text("A powerful network proxy application\nbuilt with Rust and SwiftUI")
                .multilineTextAlignment(.center)
                .foregroundColor(.secondary)
            
            Spacer()
            
            HStack(spacing: 20) {
                Link("GitHub", destination: URL(string: "https://github.com/ZejiaJiang/voyage")!)
                Link("Documentation", destination: URL(string: "https://github.com/ZejiaJiang/voyage/wiki")!)
            }
            
            Text("© 2025 Voyage. MIT License.")
                .font(.caption)
                .foregroundColor(.secondary)
        }
        .padding(40)
    }
}

// MARK: - Settings Manager

class SettingsManager: ObservableObject {
    // General
    @Published var launchAtLogin: Bool = false
    @Published var showInMenuBar: Bool = true
    @Published var showNotifications: Bool = true
    @Published var autoConnect: Bool = false
    @Published var reconnectOnNetworkChange: Bool = true
    
    // Proxy
    @Published var proxyHost: String = "127.0.0.1"
    @Published var proxyPort: Int = 1080
    @Published var proxyAuthEnabled: Bool = false
    @Published var proxyUsername: String = ""
    @Published var proxyPassword: String = ""
    
    // Advanced
    @Published var dnsMode: String = "system"
    @Published var customDns: String = "8.8.8.8"
    @Published var maxConnections: Int = 100
    @Published var tcpFastOpen: Bool = false
    @Published var ipv6Enabled: Bool = true
    @Published var logLevel: String = "info"
    
    private let defaults = UserDefaults.standard
    
    init() {
        loadSettings()
    }
    
    func loadSettings() {
        launchAtLogin = defaults.bool(forKey: "launchAtLogin")
        showInMenuBar = defaults.bool(forKey: "showInMenuBar")
        showNotifications = defaults.bool(forKey: "showNotifications")
        autoConnect = defaults.bool(forKey: "autoConnect")
        reconnectOnNetworkChange = defaults.bool(forKey: "reconnectOnNetworkChange")
        
        proxyHost = defaults.string(forKey: "proxyHost") ?? "127.0.0.1"
        proxyPort = defaults.integer(forKey: "proxyPort")
        if proxyPort == 0 { proxyPort = 1080 }
        proxyAuthEnabled = defaults.bool(forKey: "proxyAuthEnabled")
        proxyUsername = defaults.string(forKey: "proxyUsername") ?? ""
        // Note: Password should be stored in Keychain, not UserDefaults
        
        dnsMode = defaults.string(forKey: "dnsMode") ?? "system"
        customDns = defaults.string(forKey: "customDns") ?? "8.8.8.8"
        maxConnections = defaults.integer(forKey: "maxConnections")
        if maxConnections == 0 { maxConnections = 100 }
        tcpFastOpen = defaults.bool(forKey: "tcpFastOpen")
        ipv6Enabled = defaults.bool(forKey: "ipv6Enabled")
        logLevel = defaults.string(forKey: "logLevel") ?? "info"
    }
    
    func saveSettings() {
        defaults.set(launchAtLogin, forKey: "launchAtLogin")
        defaults.set(showInMenuBar, forKey: "showInMenuBar")
        defaults.set(showNotifications, forKey: "showNotifications")
        defaults.set(autoConnect, forKey: "autoConnect")
        defaults.set(reconnectOnNetworkChange, forKey: "reconnectOnNetworkChange")
        
        defaults.set(proxyHost, forKey: "proxyHost")
        defaults.set(proxyPort, forKey: "proxyPort")
        defaults.set(proxyAuthEnabled, forKey: "proxyAuthEnabled")
        defaults.set(proxyUsername, forKey: "proxyUsername")
        
        defaults.set(dnsMode, forKey: "dnsMode")
        defaults.set(customDns, forKey: "customDns")
        defaults.set(maxConnections, forKey: "maxConnections")
        defaults.set(tcpFastOpen, forKey: "tcpFastOpen")
        defaults.set(ipv6Enabled, forKey: "ipv6Enabled")
        defaults.set(logLevel, forKey: "logLevel")
    }
}

#Preview {
    SettingsView()
}
