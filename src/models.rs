use serde::{Deserialize, Serialize};

// ============================================================================
// Group 1: Container Identity
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContainerStatus {
    pub available: bool,
    pub reason: Option<String>,
}

// ============================================================================
// Group 2: Coordinated File I/O - Models
// ============================================================================

/// Read file response - either UTF-8 or bytes based on encoding parameter
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FileContent {
    Utf8 { encoding: String, content: String },
    Bytes { encoding: String, content: Vec<u8> },
}

impl FileContent {
    /// Convenience constructor for UTF-8 content
    pub fn utf8(content: String) -> Self {
        FileContent::Utf8 {
            encoding: "utf8".to_string(),
            content,
        }
    }

    /// Convenience constructor for binary content
    pub fn bytes(content: Vec<u8>) -> Self {
        FileContent::Bytes {
            encoding: "bytes".to_string(),
            content,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FolderEntry {
    pub name: String,
    pub path: String,
    pub is_directory: bool,
    pub size: Option<u64>,
    pub modified_date: Option<i64>,
    pub created_date: Option<i64>,
    pub sync_status: Option<SyncStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemExistence {
    pub exists: bool,
    pub is_directory: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemAttributes {
    pub size: u64,
    pub modified_date: i64,
    pub created_date: i64,
    #[serde(rename = "type")]
    pub item_type: String, // "file" or "dir"
    pub sync_status: Option<SyncStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrashItemResult {
    pub path: String,
}

// ============================================================================
// Group 4: iCloud Sync Controls
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyncStatus {
    pub phase: SyncPhase,
    pub is_downloading: bool,
    pub is_uploading: bool,
    pub is_uploaded: bool,
    pub download_error: Option<String>,
    pub upload_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SyncPhase {
    Current,
    NotDownloaded,
    Downloaded,
}

// ============================================================================
// Option Types
// ============================================================================

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FileProtectionType {
    Complete,
    CompleteUnlessOpen,
    CompleteUntilFirstUserAuth,
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WriteFileOptions {
    pub encoding: Option<String>, // "utf8" or "bytes"
    pub overwrite: Option<bool>,
    pub file_protection: Option<FileProtectionType>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReadFileOptions {
    pub encoding: Option<String>, // "utf8" or "bytes"
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateFileOptions {
    pub content: Option<Vec<u8>>, // Native bytes: string UTF-8 encoded or raw bytes (Uint8Array in JS)
    pub encoding: Option<String>, // "utf8" or "bytes" — tells us how to interpret content bytes
    pub file_protection: Option<FileProtectionType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateDirectoryOptions {
    pub with_intermediate_directories: Option<bool>,
    pub file_protection: Option<FileProtectionType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListDirectoryOptions {
    pub recursive: Option<bool>,
    pub skips_hidden_files: Option<bool>,
}

// ============================================================================
// Group 5: Watch Events
// ============================================================================

pub const DIRECTORY_CHANGED_EVENT: &str = "icloud://directory-changed";
pub const FILE_CHANGED_EVENT: &str = "icloud://file-changed";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryWatchEvent {
    pub watch_id: String,
    pub path: String,
    pub recursive: bool,
    pub entries: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileWatchEvent {
    pub watch_id: String,
    pub path: String,
}
