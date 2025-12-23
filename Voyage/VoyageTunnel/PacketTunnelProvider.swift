import NetworkExtension
import os.log

/// The Packet Tunnel Provider that bridges iOS network traffic to the Rust core
class PacketTunnelProvider: NEPacketTunnelProvider {
    
    private let logger = Logger(subsystem: "com.voyage.tunnel", category: "PacketTunnel")
    private var isRunning = false
    
    /// Timer for polling the Rust core
    private var pollTimer: DispatchSourceTimer?
    
    /// Packet processing queue
    private let packetQueue = DispatchQueue(label: "com.voyage.tunnel.packets", qos: .userInteractive)
    
    /// Core polling queue  
    private let pollQueue = DispatchQueue(label: "com.voyage.tunnel.poll", qos: .utility)
    
    // MARK: - Tunnel Lifecycle
    
    override func startTunnel(options: [String : NSObject]?, completionHandler: @escaping (Error?) -> Void) {
        logger.info("Starting Voyage tunnel...")
        
        // Step 1: Initialize the Rust core
        initCore()
        logger.info("Rust core initialized, version: \(getCoreVersion())")
        
        // Step 2: Load configuration
        if let config = loadConfiguration() {
            setProxyConfig(config: config)
            logger.info("Proxy config set: \(config.serverHost):\(config.serverPort)")
        }
        
        // Step 3: Load routing rules
        loadRoutingRules()
        
        // Step 4: Configure tunnel network settings
        let settings = createTunnelSettings()
        
        setTunnelNetworkSettings(settings) { [weak self] error in
            if let error = error {
                self?.logger.error("Failed to set tunnel settings: \(error.localizedDescription)")
                completionHandler(error)
                return
            }
            
            self?.logger.info("Tunnel settings configured successfully")
            self?.isRunning = true
            
            // Step 5: Start the packet processing loop
            self?.startPacketLoop()
            
            // Step 6: Start polling timer
            self?.startPollTimer()
            
            completionHandler(nil)
        }
    }
    
    override func stopTunnel(with reason: NEProviderStopReason, completionHandler: @escaping () -> Void) {
        logger.info("Stopping Voyage tunnel, reason: \(String(describing: reason))")
        
        isRunning = false
        
        // Stop poll timer
        pollTimer?.cancel()
        pollTimer = nil
        
        // Log final stats
        let stats = getStats()
        logger.info("Final stats - Sent: \(stats.bytesSent) bytes, Received: \(stats.bytesReceived) bytes, Connections: \(stats.totalConnections)")
        
        // Shutdown the Rust core
        shutdownCore()
        
        completionHandler()
    }
    
    override func handleAppMessage(_ messageData: Data, completionHandler: ((Data?) -> Void)?) {
        // Handle messages from the main app (e.g., configuration updates, stats requests)
        logger.debug("Received app message: \(messageData.count) bytes")
        
        // Parse message and respond
        if let response = handleMessage(messageData) {
            completionHandler?(response)
        } else {
            completionHandler?(nil)
        }
    }
    
    // MARK: - Configuration
    
    private func loadConfiguration() -> ProxyConfig? {
        guard let providerConfig = (protocolConfiguration as? NETunnelProviderProtocol)?.providerConfiguration else {
            logger.warning("No provider configuration found")
            return nil
        }
        
        guard let host = providerConfig["serverHost"] as? String,
              let port = providerConfig["serverPort"] as? UInt16 else {
            logger.warning("Missing server configuration")
            return nil
        }
        
        return ProxyConfig(
            serverHost: host,
            serverPort: port,
            username: providerConfig["username"] as? String,
            password: providerConfig["password"] as? String
        )
    }
    
    private func loadRoutingRules() {
        // Try to load custom rules from configuration
        if let providerConfig = (protocolConfiguration as? NETunnelProviderProtocol)?.providerConfiguration,
           let rulesText = providerConfig["rules"] as? String {
            let count = loadRules(rulesText: rulesText)
            logger.info("Loaded \(count) custom rules")
            return
        }
        
        // Load default rules
        let defaultRules = """
        # Default Voyage routing rules
        
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
        
        # Default
        FINAL,DIRECT
        """
        let count = loadRules(rulesText: defaultRules)
        logger.info("Loaded \(count) default rules")
    }
    
    // MARK: - Packet Processing
    
    private func startPacketLoop() {
        logger.info("Starting packet processing loop...")
        readPackets()
    }
    
    private func readPackets() {
        packetFlow.readPackets { [weak self] packets, protocols in
            guard let self = self, self.isRunning else { return }
            
            self.packetQueue.async {
                for (index, packet) in packets.enumerated() {
                    let protocolNumber = protocols[index].int32Value
                    
                    // Only process IPv4 (AF_INET=2) and IPv6 (AF_INET6=30) packets
                    guard protocolNumber == AF_INET || protocolNumber == AF_INET6 else {
                        continue
                    }
                    
                    // Send to Rust core for processing
                    let packetBytes = [UInt8](packet)
                    let responsePackets = processInboundPacket(packet: packetBytes)
                    
                    // Write response packets back to the tunnel
                    if !responsePackets.isEmpty {
                        self.writePacketsToTunnel(responsePackets)
                    }
                }
            }
            
            // Continue reading
            self.readPackets()
        }
    }
    
    private func startPollTimer() {
        let timer = DispatchSource.makeTimerSource(queue: pollQueue)
        timer.schedule(deadline: .now(), repeating: .milliseconds(50))
        
        timer.setEventHandler { [weak self] in
            guard let self = self, self.isRunning else { return }
            
            // Poll the Rust core for any pending work
            pollCore()
            
            // Get any pending outbound packets
            let outboundPackets = getOutboundPackets()
            
            if !outboundPackets.isEmpty {
                self.writePacketsToTunnel(outboundPackets)
            }
        }
        
        timer.resume()
        pollTimer = timer
    }
    
    private func writePacketsToTunnel(_ packets: [[UInt8]]) {
        var dataPackets: [Data] = []
        var protocols: [NSNumber] = []
        
        for packet in packets {
            let data = Data(packet)
            dataPackets.append(data)
            
            // Determine IP version from first nibble
            if let firstByte = packet.first {
                let version = firstByte >> 4
                if version == 4 {
                    protocols.append(NSNumber(value: AF_INET))
                } else if version == 6 {
                    protocols.append(NSNumber(value: AF_INET6))
                } else {
                    protocols.append(NSNumber(value: AF_INET))
                }
            }
        }
        
        if !dataPackets.isEmpty {
            packetFlow.writePackets(dataPackets, withProtocols: protocols)
        }
    }
    
    // MARK: - Tunnel Settings
    
    private func createTunnelSettings() -> NEPacketTunnelNetworkSettings {
        // Configure the virtual TUN interface
        let settings = NEPacketTunnelNetworkSettings(tunnelRemoteAddress: "10.0.0.1")
        
        // IPv4 settings
        let ipv4Settings = NEIPv4Settings(addresses: ["10.0.0.2"], subnetMasks: ["255.255.255.0"])
        ipv4Settings.includedRoutes = [NEIPv4Route.default()]
        ipv4Settings.excludedRoutes = [
            // Exclude local network
            NEIPv4Route(destinationAddress: "10.0.0.0", subnetMask: "255.0.0.0"),
            NEIPv4Route(destinationAddress: "172.16.0.0", subnetMask: "255.240.0.0"),
            NEIPv4Route(destinationAddress: "192.168.0.0", subnetMask: "255.255.0.0"),
        ]
        settings.ipv4Settings = ipv4Settings
        
        // IPv6 settings (optional but recommended)
        let ipv6Settings = NEIPv6Settings(addresses: ["fd00::2"], networkPrefixLengths: [64])
        ipv6Settings.includedRoutes = [NEIPv6Route.default()]
        settings.ipv6Settings = ipv6Settings
        
        // DNS settings
        let dnsSettings = NEDNSSettings(servers: ["8.8.8.8", "8.8.4.4"])
        dnsSettings.matchDomains = [""] // Match all domains
        settings.dnsSettings = dnsSettings
        
        // MTU
        settings.mtu = 1500
        
        return settings
    }
    
    // MARK: - Message Handling
    
    private func handleMessage(_ data: Data) -> Data? {
        // Handle IPC messages from the main app
        guard let message = String(data: data, encoding: .utf8) else {
            return nil
        }
        
        switch message {
        case "getStats":
            let stats = getStats()
            let statsJson = """
            {"bytesSent": \(stats.bytesSent), "bytesReceived": \(stats.bytesReceived), "activeConnections": \(stats.activeConnections), "totalConnections": \(stats.totalConnections)}
            """
            return statsJson.data(using: .utf8)
            
        case "getVersion":
            let version = getCoreVersion()
            return version.data(using: .utf8)
            
        case "reloadRules":
            loadRoutingRules()
            return "OK".data(using: .utf8)
            
        default:
            // Check for route evaluation: "route:domain.com" or "route:1.2.3.4"
            if message.hasPrefix("route:") {
                let target = String(message.dropFirst(6))
                let isProxy: Bool
                if target.contains(".") && !target.allSatisfy({ $0.isNumber || $0 == "." }) {
                    // Likely a domain
                    isProxy = shouldProxyDomain(domain: target)
                } else {
                    // Likely an IP
                    let action = evaluateRoute(domain: nil, ip: target)
                    isProxy = action == .proxy
                }
                return (isProxy ? "PROXY" : "DIRECT").data(using: .utf8)
            }
            
            logger.warning("Unknown message: \(message)")
            return nil
        }
    }
}