use crate::error::PluginError;
use crate::models::{
    ContainerStatus, FileContent, FileProtectionType, FolderEntry, ItemAttributes, ItemExistence,
    SyncStatus, TrashItemResult,
};
use serde::de::DeserializeOwned;
use std::future::Future;
use tauri::{
    plugin::{PluginApi, PluginHandle},
    AppHandle, Runtime,
};

const MOBILE_SCOPE: &str = "icloud-container.mobile";
const WATCHER_SCOPE: &str = "icloud-container.watcher";

fn field<T: serde::Serialize>(key: &'static str, value: &T) -> (&'static str, String) {
    (key, crate::logging::serialize_log_value(value))
}

async fn run_mobile_call<T, F>(
    scope: &str,
    started_event: &str,
    succeeded_event: &str,
    failed_event: &str,
    fields: &[(&'static str, String)],
    future: F,
) -> Result<T, PluginError>
where
    F: Future<Output = Result<T, PluginError>>,
{
    crate::logging::info(scope, started_event, fields);
    let result = future.await;
    match &result {
        Ok(_) => {
            let _ = (scope, succeeded_event, fields);
        }
        Err(error) => {
            let mut failed_fields = fields.to_vec();
            failed_fields.push((
                "error",
                crate::logging::serialize_log_value(&error.to_string()),
            ));
            crate::logging::warn(scope, failed_event, &failed_fields);
        }
    }
    result
}

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
    crate::plugin_log_info!(
        "icloud-container.mobile",
        "ios-plugin-registration-started",
        "default_identifier" => default_identifier
    );
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

    let bridge = IcloudContainer {
        handle,
        default_identifier: normalize_identifier(default_identifier),
    };

    crate::plugin_log_info!(
        "icloud-container.mobile",
        "ios-plugin-registration-succeeded"
    );

    Ok(bridge)
}

impl<R: Runtime> IcloudContainer<R> {
    pub async fn get_container_status(
        &self,
        identifier: Option<String>,
    ) -> Result<ContainerStatus, PluginError> {
        let resolved_identifier = self.resolve_identifier(identifier);
        let fields = [field("identifier", &resolved_identifier)];

        run_mobile_call(
            MOBILE_SCOPE,
            "get-container-status-started",
            "get-container-status-succeeded",
            "get-container-status-failed",
            &fields,
            async {
                self.handle
                    .run_mobile_plugin_async(
                        "getContainerStatus",
                        serde_json::json!({ "identifier": resolved_identifier }),
                    )
                    .await
                    .map_err(normalize_mobile_error)
            },
        )
        .await
    }

    pub async fn get_container_url(
        &self,
        identifier: Option<String>,
    ) -> Result<String, PluginError> {
        let resolved_identifier = self.resolve_identifier(identifier);
        let fields = [field("identifier", &resolved_identifier)];

        run_mobile_call(
            MOBILE_SCOPE,
            "get-container-url-started",
            "get-container-url-succeeded",
            "get-container-url-failed",
            &fields,
            async {
                self.handle
                    .run_mobile_plugin_async(
                        "getContainerUrl",
                        serde_json::json!({ "identifier": resolved_identifier }),
                    )
                    .await
                    .map_err(normalize_mobile_error)
            },
        )
        .await
    }

    pub async fn read_file(
        &self,
        path: String,
        identifier: Option<String>,
        encoding: String,
    ) -> Result<FileContent, PluginError> {
        let resolved_identifier = self.resolve_identifier(identifier);
        let fields = [
            field("identifier", &resolved_identifier),
            field("path", &path),
            field("encoding", &encoding),
        ];

        run_mobile_call(
            MOBILE_SCOPE,
            "read-file-started",
            "read-file-succeeded",
            "read-file-failed",
            &fields,
            async {
                self.handle
                    .run_mobile_plugin_async(
                        "readFile",
                        serde_json::json!({
                            "identifier": resolved_identifier,
                            "path": path,
                            "encoding": encoding,
                        }),
                    )
                    .await
                    .map_err(normalize_mobile_error)
            },
        )
        .await
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
        let resolved_identifier = self.resolve_identifier(identifier);
        let encoded_content = file_content_to_bytes(content);
        let wire_file_protection = file_protection_to_wire(file_protection);
        let fields = [
            field("identifier", &resolved_identifier),
            field("path", &path),
            field("content_length", &encoded_content.len()),
            field("encoding", &encoding),
            field("overwrite", &overwrite),
            field("file_protection", &wire_file_protection),
        ];

        run_mobile_call(
            MOBILE_SCOPE,
            "write-file-started",
            "write-file-succeeded",
            "write-file-failed",
            &fields,
            async {
                self.handle
                    .run_mobile_plugin_async(
                        "writeFile",
                        serde_json::json!({
                            "identifier": resolved_identifier,
                            "path": path,
                            "content": encoded_content,
                            "encoding": encoding,
                            "overwrite": overwrite,
                            "fileProtection": wire_file_protection,
                        }),
                    )
                    .await
                    .map_err(normalize_mobile_error)
            },
        )
        .await
    }

    pub async fn create_file(
        &self,
        path: String,
        content: Vec<u8>,
        identifier: Option<String>,
        encoding: String,
        file_protection: FileProtectionType,
    ) -> Result<FolderEntry, PluginError> {
        let resolved_identifier = self.resolve_identifier(identifier);
        let wire_file_protection = file_protection_to_wire(file_protection);
        let fields = [
            field("identifier", &resolved_identifier),
            field("path", &path),
            field("content_length", &content.len()),
            field("encoding", &encoding),
            field("file_protection", &wire_file_protection),
        ];

        run_mobile_call(
            MOBILE_SCOPE,
            "create-file-started",
            "create-file-succeeded",
            "create-file-failed",
            &fields,
            async {
                self.handle
                    .run_mobile_plugin_async(
                        "createFile",
                        serde_json::json!({
                            "identifier": resolved_identifier,
                            "path": path,
                            "content": content,
                            "encoding": encoding,
                            "fileProtection": wire_file_protection,
                        }),
                    )
                    .await
                    .map_err(normalize_mobile_error)
            },
        )
        .await
    }

    pub async fn item_exists(
        &self,
        path: String,
        identifier: Option<String>,
    ) -> Result<ItemExistence, PluginError> {
        let resolved_identifier = self.resolve_identifier(identifier);
        let fields = [
            field("identifier", &resolved_identifier),
            field("path", &path),
        ];

        run_mobile_call(
            MOBILE_SCOPE,
            "item-exists-started",
            "item-exists-succeeded",
            "item-exists-failed",
            &fields,
            async {
                self.handle
                    .run_mobile_plugin_async(
                        "itemExists",
                        serde_json::json!({ "identifier": resolved_identifier, "path": path }),
                    )
                    .await
                    .map_err(normalize_mobile_error)
            },
        )
        .await
    }

    pub async fn get_attributes(
        &self,
        path: String,
        identifier: Option<String>,
    ) -> Result<ItemAttributes, PluginError> {
        let resolved_identifier = self.resolve_identifier(identifier);
        let fields = [
            field("identifier", &resolved_identifier),
            field("path", &path),
        ];

        run_mobile_call(
            MOBILE_SCOPE,
            "get-attributes-started",
            "get-attributes-succeeded",
            "get-attributes-failed",
            &fields,
            async {
                self.handle
                    .run_mobile_plugin_async(
                        "getAttributes",
                        serde_json::json!({ "identifier": resolved_identifier, "path": path }),
                    )
                    .await
                    .map_err(normalize_mobile_error)
            },
        )
        .await
    }

    pub async fn create_directory(
        &self,
        path: String,
        identifier: Option<String>,
        with_intermediate_directories: bool,
        file_protection: FileProtectionType,
    ) -> Result<(), PluginError> {
        let resolved_identifier = self.resolve_identifier(identifier);
        let wire_file_protection = file_protection_to_wire(file_protection);
        let fields = [
            field("identifier", &resolved_identifier),
            field("path", &path),
            field(
                "with_intermediate_directories",
                &with_intermediate_directories,
            ),
            field("file_protection", &wire_file_protection),
        ];

        run_mobile_call(
            MOBILE_SCOPE,
            "create-directory-started",
            "create-directory-succeeded",
            "create-directory-failed",
            &fields,
            async {
                self.handle
                    .run_mobile_plugin_async(
                        "createDirectory",
                        serde_json::json!({
                            "identifier": resolved_identifier,
                            "path": path,
                            "withIntermediateDirectories": with_intermediate_directories,
                            "fileProtection": wire_file_protection,
                        }),
                    )
                    .await
                    .map_err(normalize_mobile_error)
            },
        )
        .await
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
        let resolved_identifier = self.resolve_identifier(identifier);
        let fields = [
            field("identifier", &resolved_identifier),
            field("path", &path),
            field("recursive", &recursive),
        ];

        run_mobile_call(
            WATCHER_SCOPE,
            "watch-directory-started",
            "watch-directory-succeeded",
            "watch-directory-failed",
            &fields,
            async {
                self.handle
                    .run_mobile_plugin_async(
                        "watchDirectory",
                        serde_json::json!({
                            "identifier": resolved_identifier,
                            "path": path,
                            "recursive": recursive,
                        }),
                    )
                    .await
                    .map_err(normalize_mobile_error)
            },
        )
        .await
    }

    pub async fn unwatch(&self, watch_id: String) -> Result<(), PluginError> {
        let fields = [field("watch_id", &watch_id)];

        run_mobile_call(
            WATCHER_SCOPE,
            "unwatch-started",
            "unwatch-succeeded",
            "unwatch-failed",
            &fields,
            async {
                self.handle
                    .run_mobile_plugin_async("unwatch", serde_json::json!({ "watchId": watch_id }))
                    .await
                    .map_err(normalize_mobile_error)
            },
        )
        .await
    }

    pub async fn watch_file(
        &self,
        path: String,
        identifier: Option<String>,
    ) -> Result<String, PluginError> {
        let resolved_identifier = self.resolve_identifier(identifier);
        let fields = [
            field("identifier", &resolved_identifier),
            field("path", &path),
        ];

        run_mobile_call(
            WATCHER_SCOPE,
            "watch-file-started",
            "watch-file-succeeded",
            "watch-file-failed",
            &fields,
            async {
                self.handle
                    .run_mobile_plugin_async(
                        "watchFile",
                        serde_json::json!({ "identifier": resolved_identifier, "path": path }),
                    )
                    .await
                    .map_err(normalize_mobile_error)
            },
        )
        .await
    }

    pub async fn unwatch_file(&self, watch_id: String) -> Result<(), PluginError> {
        let fields = [field("watch_id", &watch_id)];

        run_mobile_call(
            WATCHER_SCOPE,
            "unwatch-file-started",
            "unwatch-file-succeeded",
            "unwatch-file-failed",
            &fields,
            async {
                self.handle
                    .run_mobile_plugin_async(
                        "unwatchFile",
                        serde_json::json!({ "watchId": watch_id }),
                    )
                    .await
                    .map_err(normalize_mobile_error)
            },
        )
        .await
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
    use std::sync::{Mutex, OnceLock};

    struct TestLogger {
        entries: &'static Mutex<Vec<String>>,
    }

    impl log::Log for TestLogger {
        fn enabled(&self, _metadata: &log::Metadata<'_>) -> bool {
            true
        }

        fn log(&self, record: &log::Record<'_>) {
            self.entries
                .lock()
                .expect("test logger mutex poisoned")
                .push(record.args().to_string());
        }

        fn flush(&self) {}
    }

    fn test_log_entries() -> &'static Mutex<Vec<String>> {
        static ENTRIES: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
        ENTRIES.get_or_init(|| Mutex::new(Vec::new()))
    }

    fn install_test_logger() {
        static LOGGER_INSTALLED: OnceLock<()> = OnceLock::new();
        LOGGER_INSTALLED.get_or_init(|| {
            let logger = Box::leak(Box::new(TestLogger {
                entries: test_log_entries(),
            }));

            log::set_logger(logger).expect("test logger should install once");
            log::set_max_level(log::LevelFilter::Trace);
        });
    }

    fn take_logged_messages() -> Vec<String> {
        let mut entries = test_log_entries()
            .lock()
            .expect("test logger mutex poisoned");
        std::mem::take(&mut *entries)
    }

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

    #[test]
    fn run_mobile_call_logs_start_but_not_success_on_ok_result() {
        install_test_logger();
        take_logged_messages();

        let result = tauri::async_runtime::block_on(run_mobile_call(
            MOBILE_SCOPE,
            "write-file-started",
            "write-file-succeeded",
            "write-file-failed",
            &[],
            async { Ok::<(), PluginError>(()) },
        ));

        assert!(result.is_ok());

        let entries = take_logged_messages();
        assert!(entries
            .iter()
            .any(|entry| entry.contains("write-file-started")));
        assert!(!entries
            .iter()
            .any(|entry| entry.contains("write-file-succeeded")));
    }
}
