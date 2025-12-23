import Foundation
import SystemExtensions
import os.log

/// Manages the System Extension lifecycle for macOS
class SystemExtensionManager: NSObject, OSSystemExtensionRequestDelegate {
    static let shared = SystemExtensionManager()
    
    private let logger = Logger(subsystem: "com.voyage.mac", category: "SystemExtension")
    private var activationRequest: OSSystemExtensionRequest?
    
    private override init() {
        super.init()
    }
    
    // MARK: - Public API
    
    /// Check if the system extension is installed and approved
    func checkAndRequestApproval() {
        logger.info("Checking system extension status...")
        
        // Request activation of the system extension
        let request = OSSystemExtensionRequest.activationRequest(
            forExtensionWithIdentifier: "com.voyage.mac.tunnel",
            queue: .main
        )
        request.delegate = self
        
        activationRequest = request
        OSSystemExtensionManager.shared.submitRequest(request)
    }
    
    /// Deactivate the system extension
    func deactivate() {
        logger.info("Deactivating system extension...")
        
        let request = OSSystemExtensionRequest.deactivationRequest(
            forExtensionWithIdentifier: "com.voyage.mac.tunnel",
            queue: .main
        )
        request.delegate = self
        
        OSSystemExtensionManager.shared.submitRequest(request)
    }
    
    // MARK: - OSSystemExtensionRequestDelegate
    
    func request(_ request: OSSystemExtensionRequest,
                 actionForReplacingExtension existing: OSSystemExtensionProperties,
                 withExtension ext: OSSystemExtensionProperties) -> OSSystemExtensionRequest.ReplacementAction {
        logger.info("Replacing extension: \(existing.bundleVersion) -> \(ext.bundleVersion)")
        return .replace
    }
    
    func requestNeedsUserApproval(_ request: OSSystemExtensionRequest) {
        logger.info("System extension requires user approval")
        logger.info("Please go to System Preferences > Privacy & Security to approve the extension")
        
        // Notify the user
        DispatchQueue.main.async {
            self.showApprovalNotification()
        }
    }
    
    func request(_ request: OSSystemExtensionRequest, didFinishWithResult result: OSSystemExtensionRequest.Result) {
        switch result {
        case .completed:
            logger.info("System extension activated successfully")
        case .willCompleteAfterReboot:
            logger.info("System extension will complete after reboot")
        @unknown default:
            logger.warning("Unknown result: \(String(describing: result))")
        }
    }
    
    func request(_ request: OSSystemExtensionRequest, didFailWithError error: Error) {
        logger.error("System extension request failed: \(error.localizedDescription)")
        
        // Handle specific error cases
        if let systemError = error as? OSSystemExtensionError {
            switch systemError.code {
            case .extensionNotFound:
                logger.error("Extension not found in bundle")
            case .extensionRequired:
                logger.error("Extension is required but missing")
            case .authorizationRequired:
                logger.error("Authorization required")
            default:
                logger.error("System extension error: \(systemError.code.rawValue)")
            }
        }
    }
    
    // MARK: - Private Helpers
    
    private func showApprovalNotification() {
        let notification = NSUserNotification()
        notification.title = "Voyage VPN"
        notification.informativeText = "Please approve the system extension in System Preferences to enable VPN functionality."
        notification.soundName = NSUserNotificationDefaultSoundName
        
        NSUserNotificationCenter.default.deliver(notification)
    }
}
