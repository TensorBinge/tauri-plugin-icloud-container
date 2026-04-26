use crate::error::PluginError;
use crate::models::{
    ContainerStatus, FileContent, FileProtectionType, FolderEntry, ItemAttributes, ItemExistence,
    SyncStatus, TrashItemResult,
};
use serde::de::DeserializeOwned;
use tauri::{
    plugin::{PluginApi, PluginHandle},
    AppHandle, Runtime,
};

#[cfg(target_os = "ios")]
tauri::ios_plugin_binding!(init_plugin_icloud_container);

pub struct IcloudContainer<R: Runtime> {
    handle: PluginHandle<R>,
    default_identifier: Option<String>,
}

unsafe impl<R: Runtime> Send for IcloudContainer<R> {}
unsafe impl<R: Runtime> Sync for IcloudContainer<R> {}

pub fn init<R: Runtime, C: DeserializeOwned>(
    _app: &AppHandle<R>,
    api: PluginApi<R, C>,
    default_identifier: Option<String>,
) -> Result<IcloudContainer<R>, PluginError> {
    #[cfg(target_os = "ios")]
    let handle = api
        .register_ios_plugin(init_plugin_icloud_container)
        .map_err(|e| PluginError::IoError {
            detail: Some(e.to_string()),
        })?;

    #[cfg(not(target_os = "ios"))]
    let handle = {
        let _ = api;
        return Err(PluginError::ContainerUnavailable {
            detail: Some("iCloud container plugin only available on iOS".to_string()),
        });
    };

    Ok(IcloudContainer {
        handle,
        default_identifier: normalize_identifier(default_identifier),
    })
}

impl<R: Runtime> IcloudContainer<R> {
    pub async fn get_container_status(
        &self,
        identifier: Option<String>,
    ) -> Result<ContainerStatus, PluginError> {
        self.handle
            .run_mobile_plugin_async(
                "getContainerStatus",
                serde_json::json!({ "identifier": self.resolve_identifier(identifier) }),
            )
            .await
            .map_err(normalize_mobile_error)
    }

    pub async fn get_container_url(
        &self,
        identifier: Option<String>,
    ) -> Result<String, PluginError> {
        self.handle
            .run_mobile_plugin_async(
                "getContainerUrl",
                serde_json::json!({ "identifier": self.resolve_identifier(identifier) }),
            )
            .await
            .map_err(normalize_mobile_error)
    }

    pub async fn read_file(
        &self,
        path: String,
        identifier: Option<String>,
        encoding: String,
    ) -> Result<FileContent, PluginError> {
        self.handle
            .run_mobile_plugin_async(
                "readFile",
                serde_json::json!({
                    "identifier": self.resolve_identifier(identifier),
                    "path": path,
                    "encoding": encoding,
                }),
            )
            .await
            .map_err(normalize_mobile_error)
    }

    pub async fn write_file(
        &self,
        path: String,
        content: FileContent,
        identifier: Option<String>,
        encoding: String,
        overwrite: bool,
        file_protection: FileProtectionType,
    ) -> Result<(), PluginError> {
        self.handle
            .run_mobile_plugin_async(
                "writeFile",
                serde_json::json!({
                    "identifier": self.resolve_identifier(identifier),
                    "path": path,
                    "content": file_content_to_bytes(content),
                    "encoding": encoding,
                    "overwrite": overwrite,
                    "fileProtection": file_protection_to_wire(file_protection),
                }),
            )
            .await
            .map_err(normalize_mobile_error)
    }

    pub async fn create_file(
        &self,
        path: String,
        content: Vec<u8>,
        identifier: Option<String>,
        encoding: String,
        file_protection: FileProtectionType,
    ) -> Result<FolderEntry, PluginError> {
        self.handle
            .run_mobile_plugin_async(
                "createFile",
                serde_json::json!({
                    "identifier": self.resolve_identifier(identifier),
                    "path": path,
                    "content": content,
                    "encoding": encoding,
                    "fileProtection": file_protection_to_wire(file_protection),
                }),
            )
            .await
            .map_err(normalize_mobile_error)
    }

    pub async fn item_exists(
        &self,
        path: String,
        identifier: Option<String>,
    ) -> Result<ItemExistence, PluginError> {
        self.handle
            .run_mobile_plugin_async(
                "itemExists",
                serde_json::json!({ "identifier": self.resolve_identifier(identifier), "path": path }),
            )
            .await
            .map_err(normalize_mobile_error)
    }

    pub async fn get_attributes(
        &self,
        path: String,
        identifier: Option<String>,
    ) -> Result<ItemAttributes, PluginError> {
        self.handle
            .run_mobile_plugin_async(
                "getAttributes",
                serde_json::json!({ "identifier": self.resolve_identifier(identifier), "path": path }),
            )
            .await
            .map_err(normalize_mobile_error)
    }

    pub async fn create_directory(
        &self,
        path: String,
        identifier: Option<String>,
        with_intermediate_directories: bool,
        file_protection: FileProtectionType,
    ) -> Result<(), PluginError> {
        self.handle
            .run_mobile_plugin_async(
                "createDirectory",
                serde_json::json!({
                    "identifier": self.resolve_identifier(identifier),
                    "path": path,
                    "withIntermediateDirectories": with_intermediate_directories,
                    "fileProtection": file_protection_to_wire(file_protection),
                }),
            )
            .await
            .map_err(normalize_mobile_error)
    }

    pub async fn list_directory(
        &self,
        path: String,
        identifier: Option<String>,
        recursive: bool,
        skips_hidden_files: bool,
    ) -> Result<Vec<FolderEntry>, PluginError> {
        self.handle
            .run_mobile_plugin_async(
                "listDirectory",
                serde_json::json!({
                    "identifier": self.resolve_identifier(identifier),
                    "path": path,
                    "recursive": recursive,
                    "skipsHiddenFiles": skips_hidden_files,
                }),
            )
            .await
            .map_err(normalize_mobile_error)
    }

    pub async fn delete_item(
        &self,
        path: String,
        identifier: Option<String>,
    ) -> Result<(), PluginError> {
        self.handle
            .run_mobile_plugin_async(
                "deleteItem",
                serde_json::json!({ "identifier": self.resolve_identifier(identifier), "path": path }),
            )
            .await
            .map_err(normalize_mobile_error)
    }

    pub async fn trash_item(
        &self,
        path: String,
        identifier: Option<String>,
    ) -> Result<TrashItemResult, PluginError> {
        self.handle
            .run_mobile_plugin_async(
                "trashItem",
                serde_json::json!({ "identifier": self.resolve_identifier(identifier), "path": path }),
            )
            .await
            .map_err(normalize_mobile_error)
    }

    pub async fn move_item(
        &self,
        source_path: String,
        destination_path: String,
        identifier: Option<String>,
    ) -> Result<FolderEntry, PluginError> {
        self.handle
            .run_mobile_plugin_async(
                "moveItem",
                serde_json::json!({
                    "identifier": self.resolve_identifier(identifier),
                    "sourcePath": source_path,
                    "destinationPath": destination_path,
                }),
            )
            .await
            .map_err(normalize_mobile_error)
    }

    pub async fn copy_item(
        &self,
        source_path: String,
        destination_path: String,
        identifier: Option<String>,
    ) -> Result<FolderEntry, PluginError> {
        self.handle
            .run_mobile_plugin_async(
                "copyItem",
                serde_json::json!({
                    "identifier": self.resolve_identifier(identifier),
                    "sourcePath": source_path,
                    "destinationPath": destination_path,
                }),
            )
            .await
            .map_err(normalize_mobile_error)
    }

    pub async fn get_item_sync_status(
        &self,
        path: String,
        identifier: Option<String>,
    ) -> Result<SyncStatus, PluginError> {
        self.handle
            .run_mobile_plugin_async(
                "getItemSyncStatus",
                serde_json::json!({ "identifier": self.resolve_identifier(identifier), "path": path }),
            )
            .await
            .map_err(normalize_mobile_error)
    }

    pub async fn start_download(
        &self,
        path: String,
        identifier: Option<String>,
    ) -> Result<(), PluginError> {
        self.handle
            .run_mobile_plugin_async(
                "startDownload",
                serde_json::json!({ "identifier": self.resolve_identifier(identifier), "path": path }),
            )
            .await
            .map_err(normalize_mobile_error)
    }

    pub async fn evict_item(
        &self,
        path: String,
        identifier: Option<String>,
    ) -> Result<(), PluginError> {
        self.handle
            .run_mobile_plugin_async(
                "evictItem",
                serde_json::json!({ "identifier": self.resolve_identifier(identifier), "path": path }),
            )
            .await
            .map_err(normalize_mobile_error)
    }

    pub async fn is_ubiquitous(
        &self,
        path: String,
        identifier: Option<String>,
    ) -> Result<bool, PluginError> {
        self.handle
            .run_mobile_plugin_async(
                "isUbiquitous",
                serde_json::json!({ "identifier": self.resolve_identifier(identifier), "path": path }),
            )
            .await
            .map_err(normalize_mobile_error)
    }

    pub async fn watch_directory(
        &self,
        path: String,
        recursive: bool,
        identifier: Option<String>,
    ) -> Result<String, PluginError> {
        self.handle
            .run_mobile_plugin_async(
                "watchDirectory",
                serde_json::json!({
                    "identifier": self.resolve_identifier(identifier),
                    "path": path,
                    "recursive": recursive,
                }),
            )
            .await
            .map_err(normalize_mobile_error)
    }

    pub async fn unwatch(&self, watch_id: String) -> Result<(), PluginError> {
        self.handle
            .run_mobile_plugin_async("unwatch", serde_json::json!({ "watchId": watch_id }))
            .await
            .map_err(normalize_mobile_error)
    }

    pub async fn watch_file(
        &self,
        path: String,
        identifier: Option<String>,
    ) -> Result<String, PluginError> {
        self.handle
            .run_mobile_plugin_async(
                "watchFile",
                serde_json::json!({ "identifier": self.resolve_identifier(identifier), "path": path }),
            )
            .await
            .map_err(normalize_mobile_error)
    }

    pub async fn unwatch_file(&self, watch_id: String) -> Result<(), PluginError> {
        self.handle
            .run_mobile_plugin_async("unwatchFile", serde_json::json!({ "watchId": watch_id }))
            .await
            .map_err(normalize_mobile_error)
    }

    fn resolve_identifier(&self, override_identifier: Option<String>) -> Option<String> {
        normalize_identifier(override_identifier).or_else(|| self.default_identifier.clone())
    }
}

fn normalize_identifier(identifier: Option<String>) -> Option<String> {
    identifier.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn file_content_to_bytes(content: FileContent) -> Vec<u8> {
    match content {
        FileContent::Utf8 { content, .. } => content.into_bytes(),
        FileContent::Bytes { content, .. } => content,
    }
}

fn file_protection_to_wire(value: FileProtectionType) -> &'static str {
    match value {
        FileProtectionType::Complete => "complete",
        FileProtectionType::CompleteUnlessOpen => "completeUnlessOpen",
        FileProtectionType::CompleteUntilFirstUserAuth => "completeUntilFirstUserAuth",
        FileProtectionType::None => "none",
    }
}

fn normalize_mobile_error<E: ToString>(error: E) -> PluginError {
    let message = error.to_string();

    let Some(rest) = message.strip_prefix("ICLOUD_CONTAINER_ERROR:") else {
        return PluginError::IoError {
            detail: Some(message),
        };
    };

    let mut parts = rest.splitn(2, ':');
    let code = parts.next().unwrap_or_default();
    let detail = parts.next().map(ToString::to_string);

    match code {
        "CONTAINER_UNAVAILABLE" => PluginError::ContainerUnavailable { detail },
        "NOT_SIGNED_IN" => PluginError::NotSignedIn { detail },
        "PERMISSION_DENIED" => PluginError::PermissionDenied { detail },
        "PATH_OUTSIDE_CONTAINER" => PluginError::PathOutsideContainer { detail },
        "NOT_FOUND" => PluginError::NotFound { detail },
        "ALREADY_EXISTS" => PluginError::AlreadyExists { detail },
        "IO_ERROR" => PluginError::IoError { detail },
        "SYNC_ERROR" => PluginError::SyncError { detail },
        "INVALID_ARGUMENT" => PluginError::InvalidArgument { detail },
        _ => PluginError::IoError {
            detail: Some(message),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_mobile_error_maps_known_codes() {
        let err =
            normalize_mobile_error("ICLOUD_CONTAINER_ERROR:NOT_FOUND:file missing".to_string());
        match err {
            PluginError::NotFound { detail } => {
                assert_eq!(detail.as_deref(), Some("file missing"));
            }
            _ => panic!("expected not_found mapping"),
        }
    }

    #[test]
    fn normalize_mobile_error_falls_back_to_io_error_for_unprefixed() {
        let err = normalize_mobile_error("something unexpected happened".to_string());
        match err {
            PluginError::IoError { detail } => {
                assert_eq!(detail.as_deref(), Some("something unexpected happened"));
            }
            _ => panic!("expected io_error fallback"),
        }
    }

    #[test]
    fn file_content_to_bytes_preserves_binary_payload() {
        let raw = vec![0x00, 0x7F, 0xFF];
        let bytes = file_content_to_bytes(FileContent::bytes(raw.clone()));
        assert_eq!(bytes, raw);
    }

    #[test]
    fn file_protection_to_wire_matches_expected_strings() {
        assert_eq!(
            file_protection_to_wire(FileProtectionType::Complete),
            "complete"
        );
        assert_eq!(
            file_protection_to_wire(FileProtectionType::CompleteUnlessOpen),
            "completeUnlessOpen"
        );
        assert_eq!(
            file_protection_to_wire(FileProtectionType::CompleteUntilFirstUserAuth),
            "completeUntilFirstUserAuth"
        );
        assert_eq!(file_protection_to_wire(FileProtectionType::None), "none");
    }

    #[test]
    fn normalize_identifier_trims_and_discards_empty_values() {
        assert_eq!(
            normalize_identifier(Some("  iCloud.com.example.app  ".to_string())),
            Some("iCloud.com.example.app".to_string())
        );
        assert_eq!(normalize_identifier(Some("   ".to_string())), None);
        assert_eq!(normalize_identifier(None), None);
    }
}
