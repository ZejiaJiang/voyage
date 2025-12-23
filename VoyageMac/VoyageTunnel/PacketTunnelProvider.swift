import NetworkExtension
import os.log

/// The Packet Tunnel Provider that bridges macOS network traffic to the Rust core
///
/// This is the macOS equivalent of the iOS PacketTunnelProvider, running as a
/// System Extension instead of an App Extension.
class PacketTunnelProvider: NEPacketTunnelProvider {
    
    private let logger = Logger(subsystem: "com.voyage.mac.tunnel", category: "PacketTunnel")
    private var isRunning = false
    
    /// Timer for polling the Rust core
    private var pollTimer: DispatchSourceTimer?
    
    /// Packet processing queue
    private let packetQueue = DispatchQueue(label: "com.voyage.mac.tunnel.packets", qos: .userInteractive)
    
    /// Core polling queue  
    private let pollQueue = DispatchQueue(label: "com.voyage.mac.tunnel.poll", qos: .utility)
    
    // MARK: - Tunnel Lifecycle
    
    override func startTunnel(options: [String : NSObject]?, completionHandler: @escaping (Error?) -> Void) {
        logger.info("Starting Voyage tunnel on macOS...")
        
        // Step 1: Initialize the Rust core
        initializeRustCore()
        
        // Step 2: Load configuration
        loadConfiguration()
        
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
        logFinalStats()
        
        // Shutdown the Rust core
        shutdownRustCore()
        
        completionHandler()
    }
    
    override func handleAppMessage(_ messageData: Data, completionHandler: ((Data?) -> Void)?) {
        // Handle messages from the main app (e.g., configuration updates, stats requests)
        logger.debug("Received app message: \(messageData.count) bytes")
        
        if let response = handleMessage(messageData) {
            completionHandler?(response)
        } else {
            completionHandler?(nil)
        }
    }
    
    // MARK: - Rust Core Integration
    
    private func initializeRustCore() {
        // TODO: Call Rust FFI
        // initCore()
        // logger.info("Rust core initialized, version: \(getCoreVersion())")
        logger.info("Rust core initialization placeholder")
    }
    
    private func shutdownRustCore() {
        // TODO: Call Rust FFI
        // shutdownCore()
        logger.info("Rust core shutdown placeholder")
    }
    
    private func logFinalStats() {
        // TODO: Call Rust FFI
        // let stats = getStats()
        // logger.info("Final stats - Sent: \(stats.bytesSent) bytes, Received: \(stats.bytesReceived) bytes")
        logger.info("Final stats logging placeholder")
    }
    
    // MARK: - Configuration
    
    private func loadConfiguration() {
        guard let providerConfig = (protocolConfiguration as? NETunnelProviderProtocol)?.providerConfiguration else {
            logger.warning("No provider configuration found, using defaults")
            return
        }
        
        let host = providerConfig["serverHost"] as? String ?? "127.0.0.1"
        let port = providerConfig["serverPort"] as? UInt16 ?? 1080
        
        logger.info("Loaded configuration: \(host):\(port)")
        
        // TODO: Call Rust FFI
        // let config = ProxyConfig(serverHost: host, serverPort: port, username: nil, password: nil)
        // setProxyConfig(config: config)
    }
    
    private func loadRoutingRules() {
        // Try to load custom rules from configuration
        if let providerConfig = (protocolConfiguration as? NETunnelProviderProtocol)?.providerConfiguration,
           let rulesText = providerConfig["rules"] as? String {
            // TODO: Call Rust FFI
            // let count = loadRules(rulesText: rulesText)
            // logger.info("Loaded \(count) custom rules")
            logger.info("Custom rules loading placeholder")
            return
        }
        
        // Load default rules
        let defaultRules = """
        # Default Voyage routing rules for macOS
        
        # Local networks - Direct
        IP-CIDR,127.0.0.0/8,DIRECT
        IP-CIDR,192.168.0.0/16,DIRECT
        IP-CIDR,10.0.0.0/8,DIRECT
        IP-CIDR,172.16.0.0/12,DIRECT
        IP-CIDR,169.254.0.0/16,DIRECT
        
        # macOS-specific local domains
        DOMAIN-SUFFIX,local,DIRECT
        DOMAIN-SUFFIX,localhost,DIRECT
        
        # Apple services (optional: you may want to proxy these)
        DOMAIN-SUFFIX,apple.com,DIRECT
        DOMAIN-SUFFIX,icloud.com,DIRECT
        
        # Common proxy targets
        DOMAIN-SUFFIX,google.com,PROXY
        DOMAIN-SUFFIX,youtube.com,PROXY
        DOMAIN-SUFFIX,twitter.com,PROXY
        DOMAIN-SUFFIX,facebook.com,PROXY
        DOMAIN-SUFFIX,github.com,PROXY
        
        # Default
        FINAL,DIRECT
        """
        
        // TODO: Call Rust FFI
        // let count = loadRules(rulesText: defaultRules)
        // logger.info("Loaded \(count) default rules")
        logger.info("Default rules loading placeholder")
    }
    
    // MARK: - Tunnel Settings
    
    private func createTunnelSettings() -> NEPacketTunnelNetworkSettings {
        // Use a private IP range for the tunnel
        let settings = NEPacketTunnelNetworkSettings(tunnelRemoteAddress: "10.8.0.1")
        
        // IPv4 Configuration
        let ipv4Settings = NEIPv4Settings(
            addresses: ["10.8.0.2"],
            subnetMasks: ["255.255.255.0"]
        )
        
        // Route all traffic through the tunnel
        ipv4Settings.includedRoutes = [NEIPv4Route.default()]
        
        // Exclude local networks from the tunnel
        ipv4Settings.excludedRoutes = [
            NEIPv4Route(destinationAddress: "127.0.0.0", subnetMask: "255.0.0.0"),
            NEIPv4Route(destinationAddress: "192.168.0.0", subnetMask: "255.255.0.0"),
            NEIPv4Route(destinationAddress: "10.0.0.0", subnetMask: "255.0.0.0"),
            NEIPv4Route(destinationAddress: "172.16.0.0", subnetMask: "255.240.0.0"),
            NEIPv4Route(destinationAddress: "169.254.0.0", subnetMask: "255.255.0.0"),
        ]
        
        settings.ipv4Settings = ipv4Settings
        
        // IPv6 Configuration (optional)
        let ipv6Settings = NEIPv6Settings(
            addresses: ["fd00::2"],
            networkPrefixLengths: [64]
        )
        ipv6Settings.includedRoutes = [NEIPv6Route.default()]
        settings.ipv6Settings = ipv6Settings
        
        // DNS Settings
        let dnsSettings = NEDNSSettings(servers: ["8.8.8.8", "8.8.4.4"])
        dnsSettings.matchDomains = [""] // Match all domains
        settings.dnsSettings = dnsSettings
        
        // MTU
        settings.mtu = 1500
        
        return settings
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
                    
                    // TODO: Send to Rust core for processing
                    // let packetBytes = [UInt8](packet)
                    // let responsePackets = processInboundPacket(packet: packetBytes)
                    
                    // For now, just log packet info
                    self.logPacketInfo(packet, isIPv6: protocolNumber == AF_INET6)
                }
            }
            
            // Continue reading
            self.readPackets()
        }
    }
    
    private func logPacketInfo(_ packet: Data, isIPv6: Bool) {
        guard packet.count >= 20 else { return }
        
        if !isIPv6 {
            // IPv4 packet
            let srcIP = "\(packet[12]).\(packet[13]).\(packet[14]).\(packet[15])"
            let dstIP = "\(packet[16]).\(packet[17]).\(packet[18]).\(packet[19])"
            let proto = packet[9]
            
            logger.debug("IPv4: \(srcIP) -> \(dstIP), proto=\(proto), len=\(packet.count)")
        }
    }
    
    private func startPollTimer() {
        let timer = DispatchSource.makeTimerSource(queue: pollQueue)
        timer.schedule(deadline: .now(), repeating: .milliseconds(50))
        
        timer.setEventHandler { [weak self] in
            guard let self = self, self.isRunning else { return }
            
            // TODO: Poll the Rust core for any pending work
            // pollCore()
            
            // TODO: Get any pending outbound packets
            // let outboundPackets = getOutboundPackets()
            // if !outboundPackets.isEmpty {
            //     self.writePacketsToTunnel(outboundPackets)
            // }
        }
        
        timer.resume()
        pollTimer = timer
    }
    
    private func writePacketsToTunnel(_ packets: [[UInt8]]) {
        var dataPackets: [Data] = []
        var protocols: [NSNumber] = []
        
        for packet in packets {
            guard !packet.isEmpty else { continue }
            
            // Determine protocol from IP version
            let version = (packet[0] >> 4) & 0x0F
            let proto: Int32 = (version == 6) ? AF_INET6 : AF_INET
            
            dataPackets.append(Data(packet))
            protocols.append(NSNumber(value: proto))
        }
        
        if !dataPackets.isEmpty {
            packetFlow.writePackets(dataPackets, withProtocols: protocols)
        }
    }
    
    // MARK: - Message Handling
    
    private func handleMessage(_ data: Data) -> Data? {
        guard let message = String(data: data, encoding: .utf8) else {
            return nil
        }
        
        switch message {
        case "getStats":
            // TODO: Return stats from Rust core
            // let stats = getStats()
            // return try? JSONEncoder().encode(stats)
            return "{}".data(using: .utf8)
            
        case "getVersion":
            // TODO: Return version from Rust core
            // let version = getCoreVersion()
            return "0.1.0".data(using: .utf8)
            
        default:
            logger.warning("Unknown message: \(message)")
            return nil
        }
    }
}
