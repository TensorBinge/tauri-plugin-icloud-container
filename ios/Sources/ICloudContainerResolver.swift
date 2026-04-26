import Foundation

/// Thread-safe iCloud ubiquity container resolver
/// Caches the container URL and invalidates on identity change
public class ICloudContainerResolver {
    private static let shared = ICloudContainerResolver()
    
    private var containerUrl: URL?
    private var containerIdentifier: String?
    private let queue = DispatchQueue(label: "com.icloud.container.resolver", attributes: .concurrent)
    private var notificationToken: NSObjectProtocol?
    
    public init() {
        setupNotificationListener()
    }
    
    deinit {
        if let token = notificationToken {
            NotificationCenter.default.removeObserver(token)
        }
    }
    
    /// Set up listener for iCloud identity changes
    private func setupNotificationListener() {
        notificationToken = NotificationCenter.default.addObserver(
            forName: NSNotification.Name.NSUbiquityIdentityDidChange,
            object: nil,
            queue: .main
        ) { [weak self] _ in
            self?.invalidateCache()
        }
    }
    
    /// Get shared resolver instance
    public static func instance() -> ICloudContainerResolver {
        return shared
    }
    
    /// Resolve container URL asynchronously (off main thread)
    /// - Parameters:
    ///   - identifier: iCloud container identifier (e.g., "iCloud.com.example.app")
    ///   - completion: Called with (url, error) on background queue
    public func resolveContainerUrl(
        identifier: String?,
        completion: @escaping (URL?, Error?) -> Void
    ) {
        let normalizedIdentifier = Self.normalizeIdentifier(identifier)

        queue.async(flags: .barrier) { [weak self] in
            // Check cache first
            if self?.containerIdentifier == normalizedIdentifier, let url = self?.containerUrl {
                DispatchQueue.global(qos: .default).async {
                    completion(url, nil)
                }
                return
            }
            
            // Resolve off main thread
            DispatchQueue.global(qos: .default).async {
                let fileManager = FileManager.default
                guard let url = fileManager.url(forUbiquityContainerIdentifier: normalizedIdentifier) else {
                    let error = NSError(
                        domain: "ICloudContainerResolver",
                        code: -1,
                        userInfo: [NSLocalizedDescriptionKey: "Container not available or user not signed in"]
                    )
                    completion(nil, error)
                    return
                }
                
                // Cache the result
                self?.queue.async(flags: .barrier) {
                    self?.containerUrl = url
                    self?.containerIdentifier = normalizedIdentifier
                }
                
                completion(url, nil)
            }
        }
    }
    
    /// Get cached container URL if available
    /// - Parameter identifier: iCloud container identifier
    /// - Returns: Cached URL or nil if not resolved yet
    public func getCachedContainerUrl(identifier: String) -> URL? {
        var url: URL?
        queue.sync {
            if self.containerIdentifier == Self.normalizeIdentifier(identifier) {
                url = self.containerUrl
            }
        }
        return url
    }
    
    /// Check if container is available (user signed in to iCloud)
    /// - Parameter identifier: iCloud container identifier
    /// - Returns: True if container is accessible
    public func isContainerAvailable(identifier: String?) -> Bool {
        let fileManager = FileManager.default
        return fileManager.url(forUbiquityContainerIdentifier: Self.normalizeIdentifier(identifier)) != nil
    }
    
    /// Get reason why container is unavailable
    /// - Parameter identifier: iCloud container identifier
    /// - Returns: Human-readable reason or nil if available
    public func getUnavailableReason(identifier: String?) -> String? {
        // Check if user is signed in
        if FileManager.default.ubiquityIdentityToken == nil {
            return "User not signed into iCloud"
        }
        
        // Try to resolve and see if it works
        if FileManager.default.url(forUbiquityContainerIdentifier: Self.normalizeIdentifier(identifier)) == nil {
            return "iCloud container not available or invalid identifier"
        }
        
        return nil
    }
    
    /// Invalidate cached container URL (called on identity change)
    private func invalidateCache() {
        queue.async(flags: .barrier) {
            self.containerUrl = nil
            self.containerIdentifier = nil
        }
    }

    static func normalizeIdentifier(_ identifier: String?) -> String? {
        guard let identifier else {
            return nil
        }

        let trimmed = identifier.trimmingCharacters(in: .whitespacesAndNewlines)
        return trimmed.isEmpty ? nil : trimmed
    }
}
