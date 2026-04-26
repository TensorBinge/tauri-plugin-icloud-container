#if os(iOS)
import Foundation
import Tauri

private let icloudErrorPrefix = "ICLOUD_CONTAINER_ERROR"

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
        invoke.resolve(payload)
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
          invoke.resolve(payload)
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
          invoke.resolve(payload)
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
          invoke.resolve(payload)
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
          invoke.resolve(payload)
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

        invoke.resolve(payload ?? [])
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
          invoke.resolve(payload)
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
          invoke.resolve(payload)
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
          invoke.resolve(payload)
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
          invoke.resolve(payload)
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
          self?.trigger(directoryChangedEventName, payload)
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
          self?.trigger(fileChangedEventName, payload)
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
}

@_cdecl("init_plugin_icloud_container")
func initPluginIcloudContainer() -> Plugin {
  ICloudContainerTauriPlugin()
}
#endif
