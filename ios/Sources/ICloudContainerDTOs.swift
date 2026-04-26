#if os(iOS)
import Foundation

struct ContainerStatusDTO: Encodable {
  let available: Bool
  let reason: String?
}

enum FileContentDTO: Encodable {
  case utf8(String)
  case bytes([UInt8])

  private enum CodingKeys: String, CodingKey {
    case encoding
    case content
  }

  func encode(to encoder: Encoder) throws {
    var container = encoder.container(keyedBy: CodingKeys.self)

    switch self {
    case .utf8(let content):
      try container.encode("utf8", forKey: .encoding)
      try container.encode(content, forKey: .content)
    case .bytes(let content):
      try container.encode("bytes", forKey: .encoding)
      try container.encode(content, forKey: .content)
    }
  }
}

struct FolderEntryDTO: Encodable {
  let name: String
  let path: String
  let isDirectory: Bool
  let size: UInt64?
  let modifiedDate: Int64?
  let createdDate: Int64?
  let syncStatus: SyncStatusDTO?
}

struct ItemExistenceDTO: Encodable {
  let exists: Bool
  let isDirectory: Bool
}

struct SyncStatusDTO: Encodable {
  let phase: String
  let isDownloading: Bool
  let isUploading: Bool
  let isUploaded: Bool
  let downloadError: String?
  let uploadError: String?
}

struct ItemAttributesDTO: Encodable {
  let size: UInt64
  let modifiedDate: Int64
  let createdDate: Int64
  let type: String
  let syncStatus: SyncStatusDTO?
}

struct TrashItemResultDTO: Encodable {
  let path: String
}

struct DirectoryWatchEventDTO: Encodable {
  let watchId: String
  let path: String
  let recursive: Bool
  let entries: [String]
}

struct FileWatchEventDTO: Encodable {
  let watchId: String
  let path: String
}
#endif