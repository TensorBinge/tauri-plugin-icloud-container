import Foundation

#if canImport(UIKit)
import UIKit
#endif

private let directoryChangedEventName = "icloud://directory-changed"
private let fileChangedEventName = "icloud://file-changed"

private final class DirectoryWatcher {
    let watchId: String
    let rootUrl: URL
    let relativePath: String
    let recursive: Bool
    let emit: ([String: Any]) -> Void
    private let query = NSMetadataQuery()
    private var finishGatheringObserver: NSObjectProtocol?
    private var updateObserver: NSObjectProtocol?

    init(
        watchId: String,
        rootUrl: URL,
        relativePath: String,
        recursive: Bool,
        emit: @escaping ([String: Any]) -> Void
    ) {
        self.watchId = watchId
        self.rootUrl = rootUrl
        self.relativePath = relativePath
        self.recursive = recursive
        self.emit = emit
    }

    func start() {
        query.searchScopes = [rootUrl.path]
        query.predicate = NSPredicate(format: "%K BEGINSWITH %@", NSMetadataItemPathKey, rootUrl.path)

        finishGatheringObserver = NotificationCenter.default.addObserver(
            forName: NSNotification.Name.NSMetadataQueryDidFinishGathering,
            object: query,
            queue: .main
        ) { [weak self] _ in
            self?.emitSnapshot()
        }

        updateObserver = NotificationCenter.default.addObserver(
            forName: NSNotification.Name.NSMetadataQueryDidUpdate,
            object: query,
            queue: .main
        ) { [weak self] _ in
            self?.emitSnapshot()
        }

        query.start()
    }

    func stop() {
        query.stop()
        if let finishGatheringObserver {
            NotificationCenter.default.removeObserver(finishGatheringObserver)
        }
        if let updateObserver {
            NotificationCenter.default.removeObserver(updateObserver)
        }
    }

    private func emitSnapshot() {
        let entries = query.results.compactMap { item -> String? in
            guard let metadataItem = item as? NSMetadataItem,
                  let itemPath = metadataItem.value(forAttribute: NSMetadataItemPathKey) as? String else {
                return nil
            }

            let itemUrl = URL(fileURLWithPath: itemPath).standardizedFileURL
            guard itemUrl.path != rootUrl.path else {
                return nil
            }

            if !recursive {
                let parentPath = itemUrl.deletingLastPathComponent().path
                guard parentPath == rootUrl.path else {
                    return nil
                }
            }

            return relativePath(from: itemUrl)
        }.sorted()

        emit([
            "watchId": watchId,
            "path": relativePath,
            "recursive": recursive,
            "entries": entries,
        ])
    }

    private func relativePath(from url: URL) -> String {
        return relativePathString(from: url, rootUrl: rootUrl) ?? url.lastPathComponent
    }
}

private final class FileWatcher: NSObject, NSFilePresenter {
    let watchId: String
    let rootUrl: URL
    let watchedPath: String
    let presentedItemURL: URL?
    let presentedItemOperationQueue: OperationQueue
    let emit: ([String: Any]) -> Void
    private var isPresenting = false

    init(
        watchId: String,
        rootUrl: URL,
        watchedPath: String,
        fileUrl: URL,
        emit: @escaping ([String: Any]) -> Void
    ) {
        self.watchId = watchId
        self.rootUrl = rootUrl
        self.watchedPath = watchedPath
        self.presentedItemURL = fileUrl
        self.presentedItemOperationQueue = OperationQueue()
        self.presentedItemOperationQueue.maxConcurrentOperationCount = 1
        self.emit = emit
        super.init()
    }

    func start(emitInitialChange: Bool = true) {
        guard !isPresenting else {
            return
        }

        NSFileCoordinator.addFilePresenter(self)
        isPresenting = true

        if emitInitialChange {
            emitChange(path: watchedPath)
        }
    }

    func stop() {
        guard isPresenting else {
            return
        }

        NSFileCoordinator.removeFilePresenter(self)
        isPresenting = false
    }

    func presentedItemDidChange() {
        emitChange(path: watchedPath)
    }

    func presentedItemDidMove(to newURL: URL) {
        emitChange(path: relativePath(from: newURL))
    }

    private func emitChange(path: String) {
        emit([
            "watchId": watchId,
            "path": path,
        ])
    }

    private func relativePath(from url: URL) -> String {
        if url.resolvingSymlinksInPath().standardizedFileURL == rootUrl.resolvingSymlinksInPath().standardizedFileURL {
            return "."
        }

        return relativePathString(from: url, rootUrl: rootUrl) ?? watchedPath
    }
}

func relativePathString(from url: URL, rootUrl: URL) -> String? {
    let canonicalRoot = rootUrl.resolvingSymlinksInPath().standardizedFileURL
    let canonicalItem = url.resolvingSymlinksInPath().standardizedFileURL

    if canonicalItem == canonicalRoot {
        return "."
    }

    let rootComponents = canonicalRoot.pathComponents
    let itemComponents = canonicalItem.pathComponents

    guard itemComponents.count >= rootComponents.count,
          Array(itemComponents.prefix(rootComponents.count)) == rootComponents else {
        return nil
    }

    return itemComponents.dropFirst(rootComponents.count).joined(separator: "/")
}

/// Main iCloud Container Plugin class
/// Provides bridge between Rust/Tauri and Swift iOS APIs
public class ICloudContainerPlugin {
    public static let shared = ICloudContainerPlugin()

    private let resolver = ICloudContainerResolver()
    private let watcherQueue = DispatchQueue(label: "com.icloud.container.watchers", attributes: .concurrent)
    private var directoryWatchers: [String: DirectoryWatcher] = [:]
    private var fileWatchers: [String: FileWatcher] = [:]
    private var didEnterBackgroundObserver: NSObjectProtocol?
    private var willEnterForegroundObserver: NSObjectProtocol?
    
    public init() {
        setupApplicationLifecycleObservers()
    }

    deinit {
        if let didEnterBackgroundObserver {
            NotificationCenter.default.removeObserver(didEnterBackgroundObserver)
        }
        if let willEnterForegroundObserver {
            NotificationCenter.default.removeObserver(willEnterForegroundObserver)
        }
    }
    
    // MARK: - Container Identity Commands
    
    /// Get container status (availability and reason if unavailable)
    /// - Parameters:
    ///   - identifier: iCloud container identifier
    ///   - completion: Called with (available, reason) dictionary
    public func getContainerStatus(
        identifier: String?,
        completion: @escaping ([String: Any]) -> Void
    ) {
        // Route through resolveContainerUrl so the result is cached after the
        // first call. This avoids calling url(forUbiquityContainerIdentifier:)
        // twice (once for availability, once for reason) which is expensive on
        // first install when iCloud provisions the container for the first time.
        resolver.resolveContainerUrl(identifier: identifier) { url, _ in
            let available = url != nil
            var result: [String: Any] = ["available": available]
            if !available {
                // Only derive the reason when the container is not available.
                if FileManager.default.ubiquityIdentityToken == nil {
                    result["reason"] = "User not signed into iCloud"
                } else {
                    result["reason"] = "iCloud container not available or invalid identifier"
                }
            }
            completion(result)
        }
    }
    
    /// Get the absolute URL path of the ubiquity container
    /// - Parameters:
    ///   - identifier: iCloud container identifier
    ///   - completion: Called with (url, error) where error is nil on success
    public func getContainerUrl(
        identifier: String?,
        completion: @escaping (String?, Error?) -> Void
    ) {
        resolver.resolveContainerUrl(identifier: identifier) { url, error in
            if let error = error {
                completion(nil, error)
                return
            }
            if let url = url {
                completion(url.path, nil)
            } else {
                let error = NSError(
                    domain: "ICloudContainerPlugin",
                    code: -1,
                    userInfo: [NSLocalizedDescriptionKey: "Failed to resolve container URL"]
                )
                completion(nil, error)
            }
        }
    }

    // MARK: - Group 2: Coordinated File I/O

    public func readFile(
        identifier: String?,
        path: String,
        encoding: String,
        completion: @escaping ([String: Any]?, Error?) -> Void
    ) {
        if encoding != "utf8" && encoding != "bytes" {
            completion(nil, self.pluginError("Invalid encoding. Expected 'utf8' or 'bytes'"))
            return
        }

        resolveSafeContainerPath(identifier: identifier, relativePath: path) { url, error in
            if let error = error {
                completion(nil, error)
                return
            }

            guard let fileUrl = url else {
                completion(nil, self.pluginError("Failed to resolve file path"))
                return
            }

            let coordinator = NSFileCoordinator()
            var coordinationError: NSError?
            var payload: [String: Any]?
            var readError: Error?

            coordinator.coordinate(readingItemAt: fileUrl, options: [], error: &coordinationError) { coordinatedUrl in
                do {
                    let data = try Data(contentsOf: coordinatedUrl)
                    if encoding == "bytes" {
                        payload = ["encoding": "bytes", "content": [UInt8](data)]
                        return
                    }

                    guard let text = String(data: data, encoding: .utf8) else {
                        throw self.pluginError("File is not valid UTF-8")
                    }

                    payload = ["encoding": "utf8", "content": text]
                } catch {
                    readError = error
                }
            }

            if let error = readError ?? coordinationError {
                completion(nil, error)
                return
            }

            completion(payload, nil)
        }
    }

    public func writeFile(
        identifier: String?,
        path: String,
        content: Data,
        overwrite: Bool,
        fileProtection: String?,
        completion: @escaping (Error?) -> Void
    ) {
        resolveSafeContainerPath(identifier: identifier, relativePath: path) { url, error in
            if let error = error {
                completion(error)
                return
            }

            guard let fileUrl = url else {
                completion(self.pluginError("Failed to resolve file path"))
                return
            }

            let coordinator = NSFileCoordinator()
            var coordinationError: NSError?
            var writeError: Error?

            coordinator.coordinate(writingItemAt: fileUrl, options: [], error: &coordinationError) { coordinatedUrl in
                do {
                    if !overwrite && FileManager.default.fileExists(atPath: coordinatedUrl.path) {
                        throw self.pluginError("File already exists and overwrite is false")
                    }

                    try content.write(to: coordinatedUrl, options: overwrite ? [] : .withoutOverwriting)
                    try self.applyFileProtection(fileProtection, to: coordinatedUrl)
                } catch {
                    writeError = error
                }
            }

            completion(writeError ?? coordinationError)
        }
    }

    public func createFile(
        identifier: String?,
        path: String,
        content: Data,
        fileProtection: String?,
        completion: @escaping ([String: Any]?, Error?) -> Void
    ) {
        writeFile(
            identifier: identifier,
            path: path,
            content: content,
            overwrite: false,
            fileProtection: fileProtection
        ) { error in
            if let error = error {
                completion(nil, error)
                return
            }

            let now = Int64(Date().timeIntervalSince1970)
            let result: [String: Any] = [
                "name": URL(fileURLWithPath: path).lastPathComponent,
                "path": path,
                "isDirectory": false,
                "size": content.count,
                "modifiedDate": now,
                "createdDate": now,
            ]
            completion(result, nil)
        }
    }

    public func itemExists(
        identifier: String?,
        path: String,
        completion: @escaping ([String: Any]?, Error?) -> Void
    ) {
        resolveSafeContainerPath(identifier: identifier, relativePath: path) { url, error in
            if let error = error {
                completion(nil, error)
                return
            }

            guard let itemUrl = url else {
                completion(nil, self.pluginError("Failed to resolve item path"))
                return
            }

            var isDirectory = ObjCBool(false)
            let exists = FileManager.default.fileExists(atPath: itemUrl.path, isDirectory: &isDirectory)
            completion([
                "exists": exists,
                "isDirectory": isDirectory.boolValue,
            ], nil)
        }
    }

    public func getAttributes(
        identifier: String?,
        path: String,
        completion: @escaping ([String: Any]?, Error?) -> Void
    ) {
        resolveSafeContainerPath(identifier: identifier, relativePath: path) { url, error in
            if let error = error {
                completion(nil, error)
                return
            }

            guard let itemUrl = url else {
                completion(nil, self.pluginError("Failed to resolve item path"))
                return
            }

            do {
                let attrs = try FileManager.default.attributesOfItem(atPath: itemUrl.path)
                let size = attrs[.size] as? NSNumber ?? 0
                let modified = attrs[.modificationDate] as? Date ?? Date(timeIntervalSince1970: 0)
                let created = attrs[.creationDate] as? Date ?? Date(timeIntervalSince1970: 0)
                let fileType = attrs[.type] as? FileAttributeType
                let itemType = fileType == .typeDirectory ? "dir" : "file"
                let resourceValues = try? itemUrl.resourceValues(forKeys: [
                    .ubiquitousItemDownloadingStatusKey,
                    .ubiquitousItemDownloadingErrorKey,
                    .ubiquitousItemIsDownloadingKey,
                    .ubiquitousItemIsUploadedKey,
                    .ubiquitousItemIsUploadingKey,
                    .ubiquitousItemUploadingErrorKey,
                ])

                var payload: [String: Any] = [
                    "size": size.uint64Value,
                    "modifiedDate": Int64(modified.timeIntervalSince1970),
                    "createdDate": Int64(created.timeIntervalSince1970),
                    "type": itemType,
                ]

                if let values = resourceValues,
                   let syncStatus = self.syncStatusPayload(from: values) {
                    payload["syncStatus"] = syncStatus
                }

                completion(payload, nil)
            } catch {
                completion(nil, error)
            }
        }
    }

    // MARK: - Group 3: Directory Operations

    public func createDirectory(
        identifier: String?,
        path: String,
        withIntermediateDirectories: Bool,
        fileProtection: String?,
        completion: @escaping (Error?) -> Void
    ) {
        resolveSafeContainerPath(identifier: identifier, relativePath: path) { url, error in
            if let error = error {
                completion(error)
                return
            }

            guard let directoryUrl = url else {
                completion(self.pluginError("Failed to resolve directory path"))
                return
            }

            let coordinator = NSFileCoordinator()
            var coordinationError: NSError?
            var operationError: Error?

            coordinator.coordinate(writingItemAt: directoryUrl, options: [], error: &coordinationError) { coordinatedUrl in
                do {
                    try FileManager.default.createDirectory(
                        at: coordinatedUrl,
                        withIntermediateDirectories: withIntermediateDirectories
                    )
                    try self.applyFileProtection(fileProtection, to: coordinatedUrl)
                } catch {
                    operationError = error
                }
            }

            completion(operationError ?? coordinationError)
        }
    }

    public func listDirectory(
        identifier: String?,
        path: String,
        recursive: Bool,
        skipsHiddenFiles: Bool,
        completion: @escaping ([[String: Any]]?, Error?) -> Void
    ) {
        resolveSafeContainerPath(identifier: identifier, relativePath: path) { url, error in
            if let error = error {
                completion(nil, error)
                return
            }

            guard let directoryUrl = url else {
                completion(nil, self.pluginError("Failed to resolve directory path"))
                return
            }

            // iCloud metadata and NSFileCoordinator can block while files hydrate.
            // Run directory enumeration off the main thread so the UI stays responsive.
            DispatchQueue.global(qos: .userInitiated).async {
                let coordinator = NSFileCoordinator()
                var coordinationError: NSError?
                var operationError: Error?
                var payload: [[String: Any]]?

                coordinator.coordinate(readingItemAt: directoryUrl, options: [], error: &coordinationError) { coordinatedUrl in
                    do {
                        let rootUrl = coordinatedUrl.resolvingSymlinksInPath().standardizedFileURL
                        let entries = try self.collectDirectoryEntries(
                            rootUrl: rootUrl,
                            recursive: recursive,
                            skipsHiddenFiles: skipsHiddenFiles
                        )
                        payload = try entries.map { try self.folderEntryPayload(for: $0, rootUrl: rootUrl) }
                    } catch {
                        operationError = error
                    }
                }

                if let completionError = operationError ?? coordinationError {
                    completion(nil, completionError)
                    return
                }

                completion(payload ?? [], nil)
            }
        }
    }

    public func deleteItem(
        identifier: String?,
        path: String,
        completion: @escaping (Error?) -> Void
    ) {
        resolveSafeContainerPath(identifier: identifier, relativePath: path) { url, error in
            if let error = error {
                completion(error)
                return
            }

            guard let itemUrl = url else {
                completion(self.pluginError("Failed to resolve item path"))
                return
            }

            let coordinator = NSFileCoordinator()
            var coordinationError: NSError?
            var operationError: Error?

            coordinator.coordinate(writingItemAt: itemUrl, options: .forDeleting, error: &coordinationError) { coordinatedUrl in
                do {
                    try FileManager.default.removeItem(at: coordinatedUrl)
                } catch {
                    operationError = error
                }
            }

            completion(operationError ?? coordinationError)
        }
    }

    public func trashItem(
        identifier: String?,
        path: String,
        completion: @escaping ([String: Any]?, Error?) -> Void
    ) {
        do {
            let sanitized = try sanitizeRelativePath(path)

            resolver.resolveContainerUrl(identifier: identifier) { rootUrl, error in
                if let error = error {
                    completion(nil, error)
                    return
                }

                guard let rootUrl else {
                    completion(nil, self.pluginError("Container root unavailable"))
                    return
                }

                let canonicalRoot = rootUrl.resolvingSymlinksInPath().standardizedFileURL
                let sourceUrl = canonicalRoot.appendingPathComponent(sanitized).resolvingSymlinksInPath().standardizedFileURL
                guard self.isWithinRoot(sourceUrl, rootUrl: canonicalRoot) else {
                    completion(nil, self.pluginError("Path escapes container root"))
                    return
                }

                let coordinator = NSFileCoordinator()
                var coordinationError: NSError?
                var operationError: Error?
                var resultPayload: [String: Any]?

                coordinator.coordinate(writingItemAt: sourceUrl, options: .forDeleting, error: &coordinationError, byAccessor: { coordinatedSource in
                    do {
                        var resultingItemUrl: NSURL?
                        try FileManager.default.trashItem(at: coordinatedSource, resultingItemURL: &resultingItemUrl)
                        guard let resultingPath = resultingItemUrl?.path else {
                            throw self.pluginError("Failed to resolve trashed item path")
                        }
                        resultPayload = [
                            "path": resultingPath
                        ]
                    } catch {
                        operationError = error
                    }
                })

                if let error = operationError ?? coordinationError {
                    completion(nil, error)
                    return
                }

                completion(resultPayload, nil)
            }
        } catch {
            completion(nil, error)
        }
    }

    public func moveItem(
        identifier: String?,
        sourcePath: String,
        destinationPath: String,
        completion: @escaping ([String: Any]?, Error?) -> Void
    ) {
        coordinateTransfer(
            identifier: identifier,
            sourcePath: sourcePath,
            destinationPath: destinationPath,
            operation: .move,
            completion: completion
        )
    }

    public func copyItem(
        identifier: String?,
        sourcePath: String,
        destinationPath: String,
        completion: @escaping ([String: Any]?, Error?) -> Void
    ) {
        coordinateTransfer(
            identifier: identifier,
            sourcePath: sourcePath,
            destinationPath: destinationPath,
            operation: .copy,
            completion: completion
        )
    }

    // MARK: - Group 4: Sync Controls

    public func getItemSyncStatus(
        identifier: String?,
        path: String,
        completion: @escaping ([String: Any]?, Error?) -> Void
    ) {
        resolveSafeContainerPath(identifier: identifier, relativePath: path) { url, error in
            if let error = error {
                completion(nil, error)
                return
            }

            guard let itemUrl = url else {
                completion(nil, self.pluginError("Failed to resolve item path"))
                return
            }

            do {
                completion(try self.syncStatusPayload(for: itemUrl), nil)
            } catch {
                completion(nil, error)
            }
        }
    }

    public func startDownload(
        identifier: String?,
        path: String,
        completion: @escaping (Error?) -> Void
    ) {
        resolveSafeContainerPath(identifier: identifier, relativePath: path) { url, error in
            if let error = error {
                completion(error)
                return
            }

            guard let itemUrl = url else {
                completion(self.pluginError("Failed to resolve item path"))
                return
            }

            do {
                try FileManager.default.startDownloadingUbiquitousItem(at: itemUrl)
                completion(nil)
            } catch {
                completion(error)
            }
        }
    }

    public func evictItem(
        identifier: String?,
        path: String,
        completion: @escaping (Error?) -> Void
    ) {
        resolveSafeContainerPath(identifier: identifier, relativePath: path) { url, error in
            if let error = error {
                completion(error)
                return
            }

            guard let itemUrl = url else {
                completion(self.pluginError("Failed to resolve item path"))
                return
            }

            do {
                try FileManager.default.evictUbiquitousItem(at: itemUrl)
                completion(nil)
            } catch {
                completion(error)
            }
        }
    }

    public func isUbiquitous(
        identifier: String?,
        path: String,
        completion: @escaping (Bool?, Error?) -> Void
    ) {
        resolveSafeContainerPath(identifier: identifier, relativePath: path) { url, error in
            if let error = error {
                completion(nil, error)
                return
            }

            guard let itemUrl = url else {
                completion(nil, self.pluginError("Failed to resolve item path"))
                return
            }

            completion(FileManager.default.isUbiquitousItem(at: itemUrl), nil)
        }
    }

    // MARK: - Group 5: Watchers

    public func watchDirectory(
        identifier: String?,
        path: String,
        recursive: Bool,
        emit: @escaping ([String: Any]) -> Void,
        completion: @escaping (String?, Error?) -> Void
    ) {
        resolveSafeContainerPath(identifier: identifier, relativePath: path) { url, error in
            if let error = error {
                completion(nil, error)
                return
            }

            guard let directoryUrl = url else {
                completion(nil, self.pluginError("Failed to resolve directory path"))
                return
            }

            let watchId = UUID().uuidString
            let watcher = DirectoryWatcher(
                watchId: watchId,
                rootUrl: directoryUrl,
                relativePath: path,
                recursive: recursive,
                emit: emit
            )

            self.watcherQueue.async(flags: .barrier) {
                self.directoryWatchers[watchId] = watcher
            }

            DispatchQueue.main.async {
                watcher.start()
                completion(watchId, nil)
            }
        }
    }

    public func unwatch(
        watchId: String,
        completion: @escaping (Error?) -> Void
    ) {
        watcherQueue.async(flags: .barrier) {
            let watcher = self.directoryWatchers.removeValue(forKey: watchId)
            DispatchQueue.main.async {
                guard let watcher else {
                    completion(self.pluginError("Directory watcher not found"))
                    return
                }
                watcher.stop()
                completion(nil)
            }
        }
    }

    public func watchFile(
        identifier: String?,
        path: String,
        emit: @escaping ([String: Any]) -> Void,
        completion: @escaping (String?, Error?) -> Void
    ) {
        do {
            let sanitized = try sanitizeRelativePath(path)
            resolver.resolveContainerUrl(identifier: identifier) { rootUrl, error in
                if let error = error {
                    completion(nil, error)
                    return
                }

                guard let rootUrl else {
                    completion(nil, self.pluginError("Container root unavailable"))
                    return
                }

                let canonicalRoot = rootUrl.resolvingSymlinksInPath().standardizedFileURL
                let fileUrl = canonicalRoot.appendingPathComponent(sanitized).resolvingSymlinksInPath().standardizedFileURL
                guard self.isWithinRoot(fileUrl, rootUrl: canonicalRoot) else {
                    completion(nil, self.pluginError("Path escapes container root"))
                    return
                }

                let watchId = UUID().uuidString
                let watcher = FileWatcher(
                    watchId: watchId,
                    rootUrl: canonicalRoot,
                    watchedPath: path,
                    fileUrl: fileUrl,
                    emit: emit
                )

                self.watcherQueue.async(flags: .barrier) {
                    self.fileWatchers[watchId] = watcher
                }

                DispatchQueue.main.async {
                    watcher.start()
                    completion(watchId, nil)
                }
            }
        } catch {
            completion(nil, error)
        }
    }

    public func unwatchFile(
        watchId: String,
        completion: @escaping (Error?) -> Void
    ) {
        watcherQueue.async(flags: .barrier) {
            let watcher = self.fileWatchers.removeValue(forKey: watchId)
            DispatchQueue.main.async {
                guard let watcher else {
                    completion(self.pluginError("File watcher not found"))
                    return
                }
                watcher.stop()
                completion(nil)
            }
        }
    }

    public func hasDirectoryWatcher(watchId: String) -> Bool {
        var exists = false
        watcherQueue.sync {
            exists = directoryWatchers[watchId] != nil
        }
        return exists
    }

    public func hasFileWatcher(watchId: String) -> Bool {
        var exists = false
        watcherQueue.sync {
            exists = fileWatchers[watchId] != nil
        }
        return exists
    }

    // MARK: - Private Helpers

    private func setupApplicationLifecycleObservers() {
#if canImport(UIKit)
        didEnterBackgroundObserver = NotificationCenter.default.addObserver(
            forName: UIApplication.didEnterBackgroundNotification,
            object: nil,
            queue: .main
        ) { [weak self] _ in
            self?.suspendFileWatchers()
        }

        willEnterForegroundObserver = NotificationCenter.default.addObserver(
            forName: UIApplication.willEnterForegroundNotification,
            object: nil,
            queue: .main
        ) { [weak self] _ in
            self?.resumeFileWatchers()
        }
#endif
    }

    private func suspendFileWatchers() {
        let watchers = watcherQueue.sync {
            Array(fileWatchers.values)
        }

        for watcher in watchers {
            watcher.stop()
        }
    }

    private func resumeFileWatchers() {
        let watchers = watcherQueue.sync {
            Array(fileWatchers.values)
        }

        for watcher in watchers {
            watcher.start(emitInitialChange: false)
        }
    }

    private enum TransferOperation {
        case move
        case copy
    }

    private func resolveSafeContainerPath(
        identifier: String?,
        relativePath: String,
        completion: @escaping (URL?, Error?) -> Void
    ) {
        do {
            let sanitized = try sanitizeRelativePath(relativePath)

            resolver.resolveContainerUrl(identifier: identifier) { rootUrl, error in
                if let error = error {
                    completion(nil, error)
                    return
                }

                guard let rootUrl else {
                    completion(nil, self.pluginError("Container root unavailable"))
                    return
                }

                let canonicalRoot = rootUrl.resolvingSymlinksInPath().standardizedFileURL
                let resolved = canonicalRoot.appendingPathComponent(sanitized).resolvingSymlinksInPath().standardizedFileURL
                let rootPath = canonicalRoot.path
                let itemPath = resolved.path

                if itemPath == rootPath || itemPath.hasPrefix(rootPath + "/") {
                    completion(resolved, nil)
                } else {
                    completion(nil, self.pluginError("Path escapes container root"))
                }
            }
        } catch {
            completion(nil, error)
        }
    }

    private func sanitizeRelativePath(_ path: String) throws -> String {
        let trimmed = path.trimmingCharacters(in: .whitespacesAndNewlines)
        if trimmed.isEmpty {
            throw pluginError("Path is required")
        }

        if trimmed.hasPrefix("/") {
            throw pluginError("Path must be relative")
        }

        for component in trimmed.split(separator: "/") {
            if component == ".." {
                throw pluginError("Parent traversal is not allowed")
            }
        }

        return trimmed
    }

    private func pluginError(_ message: String) -> NSError {
        NSError(
            domain: "ICloudContainerPlugin",
            code: -1,
            userInfo: [NSLocalizedDescriptionKey: message]
        )
    }

    private func collectDirectoryEntries(
        rootUrl: URL,
        recursive: Bool,
        skipsHiddenFiles: Bool
    ) throws -> [URL] {
        let resourceKeys: [URLResourceKey] = [
            .isDirectoryKey,
            .isHiddenKey,
            .fileSizeKey,
            .contentModificationDateKey,
            .creationDateKey,
            .ubiquitousItemDownloadingStatusKey,
            .ubiquitousItemDownloadingErrorKey,
            .ubiquitousItemIsDownloadingKey,
            .ubiquitousItemIsUploadedKey,
            .ubiquitousItemIsUploadingKey,
            .ubiquitousItemUploadingErrorKey,
        ]

        if recursive {
            let options: FileManager.DirectoryEnumerationOptions = skipsHiddenFiles ? [.skipsHiddenFiles] : []
            guard let enumerator = FileManager.default.enumerator(
                at: rootUrl,
                includingPropertiesForKeys: resourceKeys,
                options: options
            ) else {
                throw pluginError("Failed to enumerate directory")
            }

            var entries: [URL] = []
            for case let entryUrl as URL in enumerator {
                if skipsHiddenFiles,
                   let values = try? entryUrl.resourceValues(forKeys: [.isHiddenKey]),
                   values.isHidden == true {
                    continue
                }
                entries.append(entryUrl)
            }
            return entries
        }

        let options: FileManager.DirectoryEnumerationOptions = skipsHiddenFiles ? [.skipsHiddenFiles] : []
        return try FileManager.default.contentsOfDirectory(
            at: rootUrl,
            includingPropertiesForKeys: resourceKeys,
            options: options
        ).filter { entryUrl in
            if !skipsHiddenFiles {
                return true
            }

            let values = try? entryUrl.resourceValues(forKeys: [.isHiddenKey])
            return values?.isHidden != true
        }
    }

    private func coordinateTransfer(
        identifier: String?,
        sourcePath: String,
        destinationPath: String,
        operation: TransferOperation,
        completion: @escaping ([String: Any]?, Error?) -> Void
    ) {
        do {
            let sanitizedSource = try sanitizeRelativePath(sourcePath)
            let sanitizedDestination = try sanitizeRelativePath(destinationPath)

            resolver.resolveContainerUrl(identifier: identifier) { rootUrl, error in
                if let error = error {
                    completion(nil, error)
                    return
                }

                guard let rootUrl else {
                    completion(nil, self.pluginError("Container root unavailable"))
                    return
                }

                let canonicalRoot = rootUrl.resolvingSymlinksInPath().standardizedFileURL
                let sourceUrl = canonicalRoot.appendingPathComponent(sanitizedSource).resolvingSymlinksInPath().standardizedFileURL
                let destinationUrl = canonicalRoot.appendingPathComponent(sanitizedDestination).standardizedFileURL

                guard self.isWithinRoot(sourceUrl, rootUrl: canonicalRoot),
                      self.isWithinRoot(destinationUrl, rootUrl: canonicalRoot) else {
                    completion(nil, self.pluginError("Path escapes container root"))
                    return
                }

                let coordinator = NSFileCoordinator()
                var coordinationError: NSError?
                var operationError: Error?
                var payload: [String: Any]?

                switch operation {
                case .move:
                    coordinator.coordinate(
                        writingItemAt: sourceUrl,
                        options: .forMoving,
                        writingItemAt: destinationUrl,
                        options: [],
                        error: &coordinationError,
                        byAccessor: { coordinatedSource, coordinatedDestination in
                            do {
                                try FileManager.default.moveItem(at: coordinatedSource, to: coordinatedDestination)
                                payload = try self.folderEntryPayload(
                                    for: coordinatedDestination,
                                    rootUrl: canonicalRoot
                                )
                            } catch {
                                operationError = error
                            }
                        }
                    )
                case .copy:
                    coordinator.coordinate(readingItemAt: sourceUrl, options: [], error: &coordinationError, byAccessor: { coordinatedSource in
                        do {
                            try FileManager.default.copyItem(at: coordinatedSource, to: destinationUrl)
                            payload = try self.folderEntryPayload(
                                for: destinationUrl,
                                rootUrl: canonicalRoot
                            )
                        } catch {
                            operationError = error
                        }
                    })
                }

                if let error = operationError ?? coordinationError {
                    completion(nil, error)
                    return
                }

                completion(payload, nil)
            }
        } catch {
            completion(nil, error)
        }
    }

    private func folderEntryPayload(for url: URL, rootUrl: URL) throws -> [String: Any] {
        let attrs = try FileManager.default.attributesOfItem(atPath: url.path)
        let size = (attrs[.size] as? NSNumber)?.uint64Value
        let modified = attrs[.modificationDate] as? Date
        let created = attrs[.creationDate] as? Date
        let fileType = attrs[.type] as? FileAttributeType
        let resourceValues = try? url.resourceValues(forKeys: [
            .ubiquitousItemDownloadingStatusKey,
            .ubiquitousItemDownloadingErrorKey,
            .ubiquitousItemIsDownloadingKey,
            .ubiquitousItemIsUploadedKey,
            .ubiquitousItemIsUploadingKey,
            .ubiquitousItemUploadingErrorKey,
        ])
        let syncStatus = resourceValues.flatMap { self.syncStatusPayload(from: $0) }

        return [
            "name": url.lastPathComponent,
            "path": relativePath(from: url, rootUrl: rootUrl),
            "isDirectory": fileType == .typeDirectory,
            "size": size as Any,
            "modifiedDate": modified.map { Int64($0.timeIntervalSince1970) } as Any,
            "createdDate": created.map { Int64($0.timeIntervalSince1970) } as Any,
            "syncStatus": syncStatus as Any,
        ]
    }

    private func syncStatusPayload(for url: URL) throws -> [String: Any] {
        let values = try url.resourceValues(forKeys: [
            .ubiquitousItemDownloadingStatusKey,
            .ubiquitousItemDownloadingErrorKey,
            .ubiquitousItemIsDownloadingKey,
            .ubiquitousItemIsUploadedKey,
            .ubiquitousItemIsUploadingKey,
            .ubiquitousItemUploadingErrorKey,
        ])

        return syncStatusPayload(from: values) ?? [
            "phase": "notDownloaded",
            "isDownloading": false,
            "isUploading": false,
            "isUploaded": false,
            "downloadError": NSNull(),
            "uploadError": NSNull(),
        ]
    }

    private func syncStatusPayload(from values: URLResourceValues) -> [String: Any]? {
        let hasUbiquitySignal = values.ubiquitousItemDownloadingStatus != nil
            || values.ubiquitousItemIsDownloading != nil
            || values.ubiquitousItemIsUploaded != nil
            || values.ubiquitousItemIsUploading != nil
            || values.ubiquitousItemDownloadingError != nil
            || values.ubiquitousItemUploadingError != nil

        guard hasUbiquitySignal else {
            return nil
        }

        let phase: String
        switch values.ubiquitousItemDownloadingStatus {
        case URLUbiquitousItemDownloadingStatus.current:
            phase = "current"
        case URLUbiquitousItemDownloadingStatus.downloaded:
            phase = "downloaded"
        default:
            phase = "notDownloaded"
        }

        return [
            "phase": phase,
            "isDownloading": values.ubiquitousItemIsDownloading ?? false,
            "isUploading": values.ubiquitousItemIsUploading ?? false,
            "isUploaded": values.ubiquitousItemIsUploaded ?? false,
            "downloadError": values.ubiquitousItemDownloadingError?.localizedDescription as Any,
            "uploadError": values.ubiquitousItemUploadingError?.localizedDescription as Any,
        ]
    }

    private func relativePath(from url: URL, rootUrl: URL) -> String {
        return relativePathString(from: url, rootUrl: rootUrl) ?? url.lastPathComponent
    }

    private func isWithinRoot(_ url: URL, rootUrl: URL) -> Bool {
        let rootPath = rootUrl.path
        let itemPath = url.path
        return itemPath == rootPath || itemPath.hasPrefix(rootPath + "/")
    }

    private func applyFileProtection(_ protection: String?, to url: URL) throws {
        let protectionType: FileProtectionType

        switch protection ?? "complete" {
        case "complete":
            protectionType = .complete
        case "completeUnlessOpen":
            protectionType = .completeUnlessOpen
        case "completeUntilFirstUserAuth":
            protectionType = .completeUntilFirstUserAuthentication
        case "none":
            protectionType = .none
        default:
            throw pluginError("Invalid fileProtection value")
        }

        try FileManager.default.setAttributes([.protectionKey: protectionType], ofItemAtPath: url.path)
    }
}
