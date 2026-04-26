#if os(iOS)
import Foundation
import Tauri

private let icloudErrorPrefix = "ICLOUD_CONTAINER_ERROR"
private let directoryChangedEventName = "icloud://directory-changed"
private let fileChangedEventName = "icloud://file-changed"

private func unwrapOptional(_ value: Any?) -> Any? {
  guard let value else {
    return nil
  }

  let mirror = Mirror(reflecting: value)
  guard mirror.displayStyle == .optional else {
    return value
  }

  return mirror.children.first?.value
}

private func stringValue(_ value: Any?) -> String? {
  unwrapOptional(value) as? String
}

private func boolValue(_ value: Any?) -> Bool? {
  if let value = unwrapOptional(value) as? Bool {
    return value
  }

  if let number = unwrapOptional(value) as? NSNumber {
    return number.boolValue
  }

  return nil
}

private func int64Value(_ value: Any?) -> Int64? {
  if let value = unwrapOptional(value) as? Int64 {
    return value
  }

  if let value = unwrapOptional(value) as? Int {
    return Int64(value)
  }

  if let number = unwrapOptional(value) as? NSNumber {
    return number.int64Value
  }

  return nil
}

private func uint64Value(_ value: Any?) -> UInt64? {
  if let value = unwrapOptional(value) as? UInt64 {
    return value
  }

  if let value = unwrapOptional(value) as? Int {
    return value >= 0 ? UInt64(value) : nil
  }

  if let number = unwrapOptional(value) as? NSNumber {
    return number.int64Value >= 0 ? UInt64(number.int64Value) : nil
  }

  return nil
}

private func bytesValue(_ value: Any?) -> [UInt8]? {
  if let value = unwrapOptional(value) as? [UInt8] {
    return value
  }

  if let numbers = unwrapOptional(value) as? [NSNumber] {
    return numbers.map { $0.uint8Value }
  }

  return nil
}

private func stringArrayValue(_ value: Any?) -> [String]? {
  if let value = unwrapOptional(value) as? [String] {
    return value
  }

  if let values = unwrapOptional(value) as? [Any] {
    return values.compactMap { stringValue($0) }
  }

  return nil
}

private func dictionaryValue(_ value: Any?) -> [String: Any]? {
  unwrapOptional(value) as? [String: Any]
}

private func payloadDecodingError(_ field: String) -> NSError {
  NSError(
    domain: "ICloudContainerTauriPlugin",
    code: -1,
    userInfo: [NSLocalizedDescriptionKey: "Invalid payload for \(field)"]
  )
}

private func containerStatus(from payload: [String: Any]) throws -> ContainerStatusDTO {
  guard let available = boolValue(payload["available"]) else {
    throw payloadDecodingError("container status")
  }

  return ContainerStatusDTO(
    available: available,
    reason: stringValue(payload["reason"])
  )
}

private func fileContent(from payload: [String: Any]) throws -> FileContentDTO {
  guard let encoding = stringValue(payload["encoding"]) else {
    throw payloadDecodingError("file content encoding")
  }

  switch encoding {
  case "utf8":
    guard let content = stringValue(payload["content"]) else {
      throw payloadDecodingError("utf8 file content")
    }
    return .utf8(content)
  case "bytes":
    guard let content = bytesValue(payload["content"]) else {
      throw payloadDecodingError("binary file content")
    }
    return .bytes(content)
  default:
    throw payloadDecodingError("file content encoding")
  }
}

private func syncStatus(from payload: [String: Any]) throws -> SyncStatusDTO {
  guard let phase = stringValue(payload["phase"]),
        let isDownloading = boolValue(payload["isDownloading"]),
        let isUploading = boolValue(payload["isUploading"]),
        let isUploaded = boolValue(payload["isUploaded"]) else {
    throw payloadDecodingError("sync status")
  }

  return SyncStatusDTO(
    phase: phase,
    isDownloading: isDownloading,
    isUploading: isUploading,
    isUploaded: isUploaded,
    downloadError: stringValue(payload["downloadError"]),
    uploadError: stringValue(payload["uploadError"])
  )
}

private func folderEntry(from payload: [String: Any]) throws -> FolderEntryDTO {
  guard let name = stringValue(payload["name"]),
        let path = stringValue(payload["path"]),
        let isDirectory = boolValue(payload["isDirectory"]) else {
    throw payloadDecodingError("folder entry")
  }

  let syncStatusPayload = dictionaryValue(payload["syncStatus"])

  return FolderEntryDTO(
    name: name,
    path: path,
    isDirectory: isDirectory,
    size: uint64Value(payload["size"]),
    modifiedDate: int64Value(payload["modifiedDate"]),
    createdDate: int64Value(payload["createdDate"]),
    syncStatus: try syncStatusPayload.map(syncStatus(from:))
  )
}

private func itemExistence(from payload: [String: Any]) throws -> ItemExistenceDTO {
  guard let exists = boolValue(payload["exists"]),
        let isDirectory = boolValue(payload["isDirectory"]) else {
    throw payloadDecodingError("item existence")
  }

  return ItemExistenceDTO(exists: exists, isDirectory: isDirectory)
}

private func itemAttributes(from payload: [String: Any]) throws -> ItemAttributesDTO {
  guard let size = uint64Value(payload["size"]),
        let modifiedDate = int64Value(payload["modifiedDate"]),
        let createdDate = int64Value(payload["createdDate"]),
        let type = stringValue(payload["type"]) else {
    throw payloadDecodingError("item attributes")
  }

  let syncStatusPayload = dictionaryValue(payload["syncStatus"])

  return ItemAttributesDTO(
    size: size,
    modifiedDate: modifiedDate,
    createdDate: createdDate,
    type: type,
    syncStatus: try syncStatusPayload.map(syncStatus(from:))
  )
}

private func trashItemResult(from payload: [String: Any]) throws -> TrashItemResultDTO {
  guard let path = stringValue(payload["path"]) else {
    throw payloadDecodingError("trash item result")
  }

  return TrashItemResultDTO(path: path)
}

private func directoryWatchEvent(from payload: [String: Any]) throws -> DirectoryWatchEventDTO {
  guard let watchId = stringValue(payload["watchId"]),
        let path = stringValue(payload["path"]),
        let recursive = boolValue(payload["recursive"]),
        let entries = stringArrayValue(payload["entries"]) else {
    throw payloadDecodingError("directory watch event")
  }

  return DirectoryWatchEventDTO(
    watchId: watchId,
    path: path,
    recursive: recursive,
    entries: entries
  )
}

private func fileWatchEvent(from payload: [String: Any]) throws -> FileWatchEventDTO {
  guard let watchId = stringValue(payload["watchId"]),
        let path = stringValue(payload["path"]) else {
    throw payloadDecodingError("file watch event")
  }

  return FileWatchEventDTO(watchId: watchId, path: path)
}

private func rejectMessage(for error: Error) -> String {
  let detail = (error as NSError).localizedDescription

  if detail.localizedCaseInsensitiveContains("not signed in") {
    return "\(icloudErrorPrefix):NOT_SIGNED_IN:\(detail)"
  }

  if detail.localizedCaseInsensitiveContains("outside") || detail.localizedCaseInsensitiveContains("relative") || detail.localizedCaseInsensitiveContains("traversal") {
    return "\(icloudErrorPrefix):PATH_OUTSIDE_CONTAINER:\(detail)"
  }

  if detail.localizedCaseInsensitiveContains("required") || detail.localizedCaseInsensitiveContains("invalid") {
    return "\(icloudErrorPrefix):INVALID_ARGUMENT:\(detail)"
  }

  if detail.localizedCaseInsensitiveContains("already exists") {
    return "\(icloudErrorPrefix):ALREADY_EXISTS:\(detail)"
  }

  if detail.localizedCaseInsensitiveContains("no such file") || detail.localizedCaseInsensitiveContains("not found") {
    return "\(icloudErrorPrefix):NOT_FOUND:\(detail)"
  }

  if detail.localizedCaseInsensitiveContains("permission") || detail.localizedCaseInsensitiveContains("denied") {
    return "\(icloudErrorPrefix):PERMISSION_DENIED:\(detail)"
  }

  if detail.localizedCaseInsensitiveContains("ubiquitous") || detail.localizedCaseInsensitiveContains("download") || detail.localizedCaseInsensitiveContains("evict") || detail.localizedCaseInsensitiveContains("sync") {
    return "\(icloudErrorPrefix):SYNC_ERROR:\(detail)"
  }

  return "\(icloudErrorPrefix):IO_ERROR:\(detail)"
}

private struct IdentifierArgs: Decodable {
  let identifier: String?
}

private struct ReadFileArgs: Decodable {
  let identifier: String?
  let path: String
  let encoding: String
}

private struct WriteFileArgs: Decodable {
  let identifier: String?
  let path: String
  let content: [UInt8]
  let encoding: String
  let overwrite: Bool
  let fileProtection: String?
}

private struct CreateFileArgs: Decodable {
  let identifier: String?
  let path: String
  let content: [UInt8]?
  let encoding: String
  let fileProtection: String?
}

private struct PathArgs: Decodable {
  let identifier: String?
  let path: String
}

private struct CreateDirectoryArgs: Decodable {
  let identifier: String?
  let path: String
  let withIntermediateDirectories: Bool?
  let fileProtection: String?
}

private struct ListDirectoryArgs: Decodable {
  let identifier: String?
  let path: String
  let recursive: Bool?
  let skipsHiddenFiles: Bool?
}

private struct MoveCopyArgs: Decodable {
  let identifier: String?
  let sourcePath: String
  let destinationPath: String
}

private struct WatchDirectoryArgs: Decodable {
  let identifier: String?
  let path: String
  let recursive: Bool?
}

private struct WatchIdArgs: Decodable {
  let watchId: String
}

final class ICloudContainerTauriPlugin: Plugin {
  private let service = ICloudContainerPlugin.shared

  @objc public func getContainerStatus(_ invoke: Invoke) {
    do {
      let args = try invoke.parseArgs(IdentifierArgs.self)
      service.getContainerStatus(identifier: args.identifier) { payload in
        do {
          invoke.resolve(try containerStatus(from: payload))
        } catch {
          invoke.reject(rejectMessage(for: error))
        }
      }
    } catch {
      invoke.reject(rejectMessage(for: error))
    }
  }

  @objc public func getContainerUrl(_ invoke: Invoke) {
    do {
      let args = try invoke.parseArgs(IdentifierArgs.self)
      service.getContainerUrl(identifier: args.identifier) { path, error in
        if let error {
          invoke.reject(rejectMessage(for: error))
          return
        }

        if let path {
          invoke.resolve(path)
        } else {
          invoke.reject("\(icloudErrorPrefix):IO_ERROR:missing container url")
        }
      }
    } catch {
      invoke.reject(rejectMessage(for: error))
    }
  }

  @objc public func readFile(_ invoke: Invoke) {
    do {
      let args = try invoke.parseArgs(ReadFileArgs.self)
      service.readFile(identifier: args.identifier, path: args.path, encoding: args.encoding) { payload, error in
        if let error {
          invoke.reject(rejectMessage(for: error))
          return
        }

        if let payload {
          do {
            invoke.resolve(try fileContent(from: payload))
          } catch {
            invoke.reject(rejectMessage(for: error))
          }
        } else {
          invoke.reject("\(icloudErrorPrefix):IO_ERROR:missing read payload")
        }
      }
    } catch {
      invoke.reject(rejectMessage(for: error))
    }
  }

  @objc public func writeFile(_ invoke: Invoke) {
    do {
      let args = try invoke.parseArgs(WriteFileArgs.self)
      service.writeFile(
        identifier: args.identifier,
        path: args.path,
        content: Data(args.content),
        overwrite: args.overwrite,
        fileProtection: args.fileProtection
      ) { error in
        if let error {
          invoke.reject(rejectMessage(for: error))
          return
        }

        invoke.resolve()
      }
    } catch {
      invoke.reject(rejectMessage(for: error))
    }
  }

  @objc public func createFile(_ invoke: Invoke) {
    do {
      let args = try invoke.parseArgs(CreateFileArgs.self)
      service.createFile(
        identifier: args.identifier,
        path: args.path,
        content: Data(args.content ?? []),
        fileProtection: args.fileProtection
      ) { payload, error in
        if let error {
          invoke.reject(rejectMessage(for: error))
          return
        }

        if let payload {
          do {
            invoke.resolve(try folderEntry(from: payload))
          } catch {
            invoke.reject(rejectMessage(for: error))
          }
        } else {
          invoke.reject("\(icloudErrorPrefix):IO_ERROR:missing create payload")
        }
      }
    } catch {
      invoke.reject(rejectMessage(for: error))
    }
  }

  @objc public func itemExists(_ invoke: Invoke) {
    do {
      let args = try invoke.parseArgs(PathArgs.self)
      service.itemExists(identifier: args.identifier, path: args.path) { payload, error in
        if let error {
          invoke.reject(rejectMessage(for: error))
          return
        }

        if let payload {
          do {
            invoke.resolve(try itemExistence(from: payload))
          } catch {
            invoke.reject(rejectMessage(for: error))
          }
        } else {
          invoke.reject("\(icloudErrorPrefix):IO_ERROR:missing existence payload")
        }
      }
    } catch {
      invoke.reject(rejectMessage(for: error))
    }
  }

  @objc public func getAttributes(_ invoke: Invoke) {
    do {
      let args = try invoke.parseArgs(PathArgs.self)
      service.getAttributes(identifier: args.identifier, path: args.path) { payload, error in
        if let error {
          invoke.reject(rejectMessage(for: error))
          return
        }

        if let payload {
          do {
            invoke.resolve(try itemAttributes(from: payload))
          } catch {
            invoke.reject(rejectMessage(for: error))
          }
        } else {
          invoke.reject("\(icloudErrorPrefix):IO_ERROR:missing attributes payload")
        }
      }
    } catch {
      invoke.reject(rejectMessage(for: error))
    }
  }

  @objc public func createDirectory(_ invoke: Invoke) {
    do {
      let args = try invoke.parseArgs(CreateDirectoryArgs.self)
      service.createDirectory(
        identifier: args.identifier,
        path: args.path,
        withIntermediateDirectories: args.withIntermediateDirectories ?? true,
        fileProtection: args.fileProtection
      ) { error in
        if let error {
          invoke.reject(rejectMessage(for: error))
          return
        }

        invoke.resolve()
      }
    } catch {
      invoke.reject(rejectMessage(for: error))
    }
  }

  @objc public func listDirectory(_ invoke: Invoke) {
    do {
      let args = try invoke.parseArgs(ListDirectoryArgs.self)
      service.listDirectory(
        identifier: args.identifier,
        path: args.path,
        recursive: args.recursive ?? false,
        skipsHiddenFiles: args.skipsHiddenFiles ?? false
      ) { payload, error in
        if let error {
          invoke.reject(rejectMessage(for: error))
          return
        }

        do {
          let entries = try (payload ?? []).map { payload in
            try folderEntry(from: payload)
          }
          invoke.resolve(entries)
        } catch {
          invoke.reject(rejectMessage(for: error))
        }
      }
    } catch {
      invoke.reject(rejectMessage(for: error))
    }
  }

  @objc public func deleteItem(_ invoke: Invoke) {
    do {
      let args = try invoke.parseArgs(PathArgs.self)
      service.deleteItem(identifier: args.identifier, path: args.path) { error in
        if let error {
          invoke.reject(rejectMessage(for: error))
          return
        }

        invoke.resolve()
      }
    } catch {
      invoke.reject(rejectMessage(for: error))
    }
  }

  @objc public func trashItem(_ invoke: Invoke) {
    do {
      let args = try invoke.parseArgs(PathArgs.self)
      service.trashItem(identifier: args.identifier, path: args.path) { payload, error in
        if let error {
          invoke.reject(rejectMessage(for: error))
          return
        }

        if let payload {
          do {
            invoke.resolve(try trashItemResult(from: payload))
          } catch {
            invoke.reject(rejectMessage(for: error))
          }
        } else {
          invoke.reject("\(icloudErrorPrefix):IO_ERROR:missing trash payload")
        }
      }
    } catch {
      invoke.reject(rejectMessage(for: error))
    }
  }

  @objc public func moveItem(_ invoke: Invoke) {
    do {
      let args = try invoke.parseArgs(MoveCopyArgs.self)
      service.moveItem(
        identifier: args.identifier,
        sourcePath: args.sourcePath,
        destinationPath: args.destinationPath
      ) { payload, error in
        if let error {
          invoke.reject(rejectMessage(for: error))
          return
        }

        if let payload {
          do {
            invoke.resolve(try folderEntry(from: payload))
          } catch {
            invoke.reject(rejectMessage(for: error))
          }
        } else {
          invoke.reject("\(icloudErrorPrefix):IO_ERROR:missing move payload")
        }
      }
    } catch {
      invoke.reject(rejectMessage(for: error))
    }
  }

  @objc public func copyItem(_ invoke: Invoke) {
    do {
      let args = try invoke.parseArgs(MoveCopyArgs.self)
      service.copyItem(
        identifier: args.identifier,
        sourcePath: args.sourcePath,
        destinationPath: args.destinationPath
      ) { payload, error in
        if let error {
          invoke.reject(rejectMessage(for: error))
          return
        }

        if let payload {
          do {
            invoke.resolve(try folderEntry(from: payload))
          } catch {
            invoke.reject(rejectMessage(for: error))
          }
        } else {
          invoke.reject("\(icloudErrorPrefix):IO_ERROR:missing copy payload")
        }
      }
    } catch {
      invoke.reject(rejectMessage(for: error))
    }
  }

  @objc public func getItemSyncStatus(_ invoke: Invoke) {
    do {
      let args = try invoke.parseArgs(PathArgs.self)
      service.getItemSyncStatus(identifier: args.identifier, path: args.path) { payload, error in
        if let error {
          invoke.reject(rejectMessage(for: error))
          return
        }

        if let payload {
          do {
            invoke.resolve(try syncStatus(from: payload))
          } catch {
            invoke.reject(rejectMessage(for: error))
          }
        } else {
          invoke.reject("\(icloudErrorPrefix):IO_ERROR:missing sync status payload")
        }
      }
    } catch {
      invoke.reject(rejectMessage(for: error))
    }
  }

  @objc public func startDownload(_ invoke: Invoke) {
    do {
      let args = try invoke.parseArgs(PathArgs.self)
      service.startDownload(identifier: args.identifier, path: args.path) { error in
        if let error {
          invoke.reject(rejectMessage(for: error))
          return
        }

        invoke.resolve()
      }
    } catch {
      invoke.reject(rejectMessage(for: error))
    }
  }

  @objc public func evictItem(_ invoke: Invoke) {
    do {
      let args = try invoke.parseArgs(PathArgs.self)
      service.evictItem(identifier: args.identifier, path: args.path) { error in
        if let error {
          invoke.reject(rejectMessage(for: error))
          return
        }

        invoke.resolve()
      }
    } catch {
      invoke.reject(rejectMessage(for: error))
    }
  }

  @objc public func isUbiquitous(_ invoke: Invoke) {
    do {
      let args = try invoke.parseArgs(PathArgs.self)
      service.isUbiquitous(identifier: args.identifier, path: args.path) { value, error in
        if let error {
          invoke.reject(rejectMessage(for: error))
          return
        }

        invoke.resolve(value ?? false)
      }
    } catch {
      invoke.reject(rejectMessage(for: error))
    }
  }

  @objc public func watchDirectory(_ invoke: Invoke) {
    do {
      let args = try invoke.parseArgs(WatchDirectoryArgs.self)
      service.watchDirectory(
        identifier: args.identifier,
        path: args.path,
        recursive: args.recursive ?? false,
        emit: { [weak self] payload in
          guard let self else {
            return
          }

          do {
            try self.trigger(directoryChangedEventName, data: directoryWatchEvent(from: payload))
          } catch {
            return
          }
        }
      ) { watchId, error in
        if let error {
          invoke.reject(rejectMessage(for: error))
          return
        }

        if let watchId {
          invoke.resolve(watchId)
        } else {
          invoke.reject("\(icloudErrorPrefix):IO_ERROR:missing watch id")
        }
      }
    } catch {
      invoke.reject(rejectMessage(for: error))
    }
  }

  @objc public func unwatch(_ invoke: Invoke) {
    do {
      let args = try invoke.parseArgs(WatchIdArgs.self)
      service.unwatch(watchId: args.watchId) { error in
        if let error {
          invoke.reject(rejectMessage(for: error))
          return
        }

        invoke.resolve()
      }
    } catch {
      invoke.reject(rejectMessage(for: error))
    }
  }

  @objc public func watchFile(_ invoke: Invoke) {
    do {
      let args = try invoke.parseArgs(PathArgs.self)
      service.watchFile(
        identifier: args.identifier,
        path: args.path,
        emit: { [weak self] payload in
          guard let self else {
            return
          }

          do {
            try self.trigger(fileChangedEventName, data: fileWatchEvent(from: payload))
          } catch {
            return
          }
        }
      ) { watchId, error in
        if let error {
          invoke.reject(rejectMessage(for: error))
          return
        }

        if let watchId {
          invoke.resolve(watchId)
        } else {
          invoke.reject("\(icloudErrorPrefix):IO_ERROR:missing file watch id")
        }
      }
    } catch {
      invoke.reject(rejectMessage(for: error))
    }
  }

  @objc public func unwatchFile(_ invoke: Invoke) {
    do {
      let args = try invoke.parseArgs(WatchIdArgs.self)
      service.unwatchFile(watchId: args.watchId) { error in
        if let error {
          invoke.reject(rejectMessage(for: error))
          return
        }

        invoke.resolve()
      }
    } catch {
      invoke.reject(rejectMessage(for: error))
    }
  }
}

@_cdecl("init_plugin_icloud_container")
func initPluginIcloudContainer() -> Plugin {
  ICloudContainerTauriPlugin()
}
#endif
