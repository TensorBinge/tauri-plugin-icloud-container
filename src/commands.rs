use crate::{
    error::{unsupported, PluginError},
    models::{
        ContainerStatus, CreateDirectoryOptions, CreateFileOptions, FileContent,
        FileProtectionType, FolderEntry, ItemAttributes, ItemExistence, ListDirectoryOptions,
        ReadFileOptions, SyncStatus, TrashItemResult, WriteFileOptions,
    },
};
use std::path::{Component, Path};
use tauri::{AppHandle, Runtime};

#[cfg(mobile)]
use crate::IcloudContainer;
#[cfg(mobile)]
use std::future::Future;

#[cfg(mobile)]
const COMMAND_SCOPE: &str = "icloud-container.command";

fn field<T: serde::Serialize>(key: &'static str, value: &T) -> (&'static str, String) {
    (key, crate::logging::serialize_log_value(value))
}

#[cfg(mobile)]
async fn run_command_with_logging<T, F>(
    started_event: &str,
    succeeded_event: &str,
    failed_event: &str,
    fields: &[(&'static str, String)],
    future: F,
) -> Result<T, PluginError>
where
    F: Future<Output = Result<T, PluginError>>,
{
    crate::logging::info(COMMAND_SCOPE, started_event, fields);
    let result = future.await;
    match &result {
        Ok(_) => crate::logging::info(COMMAND_SCOPE, succeeded_event, fields),
        Err(error) => {
            let mut failed_fields = fields.to_vec();
            failed_fields.push((
                "error",
                crate::logging::serialize_log_value(&error.to_string()),
            ));
            crate::logging::warn(COMMAND_SCOPE, failed_event, &failed_fields);
        }
    }
    result
}

fn validate_identifier(identifier: String) -> Result<String, PluginError> {
    let trimmed = identifier.trim();
    if trimmed.is_empty() {
        return Err(PluginError::InvalidArgument {
            detail: Some("identifier is required and cannot be empty".to_string()),
        });
    }

    Ok(trimmed.to_string())
}

fn resolve_optional_identifier(identifier: Option<String>) -> Result<Option<String>, PluginError> {
    match identifier {
        Some(value) => validate_identifier(value).map(Some),
        None => Ok(None),
    }
}

fn validate_watch_id(watch_id: String) -> Result<String, PluginError> {
    let trimmed = watch_id.trim();
    if trimmed.is_empty() {
        return Err(PluginError::InvalidArgument {
            detail: Some("watch_id is required and cannot be empty".to_string()),
        });
    }

    Ok(trimmed.to_string())
}

pub fn validate_relative_path(path: String) -> Result<String, PluginError> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err(PluginError::InvalidArgument {
            detail: Some("path is required and cannot be empty".to_string()),
        });
    }

    let raw_path = Path::new(trimmed);
    if raw_path.is_absolute() {
        return Err(PluginError::PathOutsideContainer {
            detail: Some("path must be relative to container root".to_string()),
        });
    }

    for component in raw_path.components() {
        if matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        ) {
            return Err(PluginError::PathOutsideContainer {
                detail: Some("path cannot traverse outside container root".to_string()),
            });
        }
    }

    Ok(trimmed.to_string())
}

pub fn resolve_encoding(encoding: Option<String>) -> Result<String, PluginError> {
    match encoding
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        None => Ok("utf8".to_string()),
        Some("utf8") => Ok("utf8".to_string()),
        Some("bytes") => Ok("bytes".to_string()),
        Some(other) => Err(PluginError::InvalidArgument {
            detail: Some(format!(
                "invalid encoding '{other}', expected 'utf8' or 'bytes'"
            )),
        }),
    }
}

pub fn resolve_write_options(
    options: Option<WriteFileOptions>,
) -> Result<(String, bool, FileProtectionType), PluginError> {
    let opts = options.unwrap_or(WriteFileOptions {
        encoding: None,
        overwrite: None,
        file_protection: None,
    });

    let encoding = resolve_encoding(opts.encoding)?;
    let overwrite = opts.overwrite.unwrap_or(true);
    let protection = opts.file_protection.unwrap_or(FileProtectionType::Complete);

    Ok((encoding, overwrite, protection))
}

pub fn resolve_read_options(options: Option<ReadFileOptions>) -> Result<String, PluginError> {
    let opts = options.unwrap_or(ReadFileOptions { encoding: None });
    resolve_encoding(opts.encoding)
}

pub fn resolve_create_file_options(
    options: Option<CreateFileOptions>,
) -> Result<(Vec<u8>, String, FileProtectionType), PluginError> {
    let opts = options.unwrap_or(CreateFileOptions {
        content: None,
        encoding: None,
        file_protection: None,
    });

    let encoding = resolve_encoding(opts.encoding)?;
    let protection = opts.file_protection.unwrap_or(FileProtectionType::Complete);
    let content = opts.content.unwrap_or_default();

    Ok((content, encoding, protection))
}

pub fn resolve_create_directory_options(
    options: Option<CreateDirectoryOptions>,
) -> (bool, FileProtectionType) {
    let opts = options.unwrap_or(CreateDirectoryOptions {
        with_intermediate_directories: None,
        file_protection: None,
    });

    (
        opts.with_intermediate_directories.unwrap_or(true),
        opts.file_protection.unwrap_or(FileProtectionType::Complete),
    )
}

pub fn resolve_list_directory_options(options: Option<ListDirectoryOptions>) -> (bool, bool) {
    let opts = options.unwrap_or(ListDirectoryOptions {
        recursive: None,
        skips_hidden_files: None,
    });

    (
        opts.recursive.unwrap_or(false),
        opts.skips_hidden_files.unwrap_or(false),
    )
}

// ============================================================================
// Group 1: Container Identity
// ============================================================================

#[tauri::command]
pub async fn get_container_status<R: Runtime>(
    app: AppHandle<R>,
    identifier: Option<String>,
) -> Result<ContainerStatus, PluginError> {
    let id = resolve_optional_identifier(identifier)?;
    let fields = [field("identifier", &id)];
    #[cfg(not(mobile))]
    let _ = &fields;

    #[cfg(mobile)]
    {
        let bridge = app.state::<IcloudContainer<R>>();
        run_command_with_logging(
            "get-container-status-started",
            "get-container-status-succeeded",
            "get-container-status-failed",
            &fields,
            bridge.get_container_status(id),
        )
        .await
    }

    #[cfg(not(mobile))]
    {
        let _ = (app, id);
        unsupported()
    }
}

#[tauri::command]
pub async fn get_container_url<R: Runtime>(
    app: AppHandle<R>,
    identifier: Option<String>,
) -> Result<String, PluginError> {
    let id = resolve_optional_identifier(identifier)?;
    let fields = [field("identifier", &id)];
    #[cfg(not(mobile))]
    let _ = &fields;

    #[cfg(mobile)]
    {
        let bridge = app.state::<IcloudContainer<R>>();
        run_command_with_logging(
            "get-container-url-started",
            "get-container-url-succeeded",
            "get-container-url-failed",
            &fields,
            bridge.get_container_url(id),
        )
        .await
    }

    #[cfg(not(mobile))]
    {
        let _ = (app, id);
        unsupported()
    }
}

// ============================================================================
// Group 2: Coordinated File I/O
// ============================================================================

#[tauri::command]
pub async fn read_file<R: Runtime>(
    app: AppHandle<R>,
    path: String,
    identifier: Option<String>,
    options: Option<ReadFileOptions>,
) -> Result<FileContent, PluginError> {
    let valid_path = validate_relative_path(path)?;
    let id = resolve_optional_identifier(identifier)?;
    let encoding = resolve_read_options(options)?;
    let fields = [
        field("path", &valid_path),
        field("identifier", &id),
        field("encoding", &encoding),
    ];
    #[cfg(not(mobile))]
    let _ = &fields;

    #[cfg(mobile)]
    {
        let bridge = app.state::<IcloudContainer<R>>();
        run_command_with_logging(
            "read-file-started",
            "read-file-succeeded",
            "read-file-failed",
            &fields,
            bridge.read_file(valid_path, id, encoding),
        )
        .await
    }

    #[cfg(not(mobile))]
    {
        let _ = (app, valid_path, id, encoding);
        unsupported()
    }
}

#[tauri::command]
pub async fn write_file<R: Runtime>(
    app: AppHandle<R>,
    path: String,
    content: FileContent,
    identifier: Option<String>,
    options: Option<WriteFileOptions>,
) -> Result<(), PluginError> {
    let valid_path = validate_relative_path(path)?;
    let id = resolve_optional_identifier(identifier)?;
    let (encoding, overwrite, file_protection) = resolve_write_options(options)?;
    let fields = [
        field("path", &valid_path),
        field("identifier", &id),
        field("encoding", &encoding),
        field("overwrite", &overwrite),
        field("file_protection", &file_protection),
    ];
    #[cfg(not(mobile))]
    let _ = &fields;

    #[cfg(mobile)]
    {
        let bridge = app.state::<IcloudContainer<R>>();
        run_command_with_logging(
            "write-file-started",
            "write-file-succeeded",
            "write-file-failed",
            &fields,
            bridge.write_file(
                valid_path,
                content,
                id,
                encoding,
                overwrite,
                file_protection,
            ),
        )
        .await
    }

    #[cfg(not(mobile))]
    {
        let _ = (
            app,
            valid_path,
            content,
            id,
            encoding,
            overwrite,
            file_protection,
        );
        unsupported()
    }
}

#[tauri::command]
pub async fn create_file<R: Runtime>(
    app: AppHandle<R>,
    path: String,
    identifier: Option<String>,
    options: Option<CreateFileOptions>,
) -> Result<FolderEntry, PluginError> {
    let valid_path = validate_relative_path(path)?;
    let id = resolve_optional_identifier(identifier)?;
    let (content, encoding, file_protection) = resolve_create_file_options(options)?;
    let fields = [
        field("path", &valid_path),
        field("identifier", &id),
        field("content_length", &content.len()),
        field("encoding", &encoding),
        field("file_protection", &file_protection),
    ];
    #[cfg(not(mobile))]
    let _ = &fields;

    #[cfg(mobile)]
    {
        let bridge = app.state::<IcloudContainer<R>>();
        run_command_with_logging(
            "create-file-started",
            "create-file-succeeded",
            "create-file-failed",
            &fields,
            bridge.create_file(valid_path, content, id, encoding, file_protection),
        )
        .await
    }

    #[cfg(not(mobile))]
    {
        let _ = (app, valid_path, content, id, encoding, file_protection);
        unsupported()
    }
}

#[tauri::command]
pub async fn item_exists<R: Runtime>(
    app: AppHandle<R>,
    path: String,
    identifier: Option<String>,
) -> Result<ItemExistence, PluginError> {
    let valid_path = validate_relative_path(path)?;
    let id = resolve_optional_identifier(identifier)?;
    let fields = [field("path", &valid_path), field("identifier", &id)];
    #[cfg(not(mobile))]
    let _ = &fields;

    #[cfg(mobile)]
    {
        let bridge = app.state::<IcloudContainer<R>>();
        run_command_with_logging(
            "item-exists-started",
            "item-exists-succeeded",
            "item-exists-failed",
            &fields,
            bridge.item_exists(valid_path, id),
        )
        .await
    }

    #[cfg(not(mobile))]
    {
        let _ = (app, valid_path, id);
        unsupported()
    }
}

#[tauri::command]
pub async fn get_attributes<R: Runtime>(
    app: AppHandle<R>,
    path: String,
    identifier: Option<String>,
) -> Result<ItemAttributes, PluginError> {
    let valid_path = validate_relative_path(path)?;
    let id = resolve_optional_identifier(identifier)?;
    let fields = [field("path", &valid_path), field("identifier", &id)];
    #[cfg(not(mobile))]
    let _ = &fields;

    #[cfg(mobile)]
    {
        let bridge = app.state::<IcloudContainer<R>>();
        run_command_with_logging(
            "get-attributes-started",
            "get-attributes-succeeded",
            "get-attributes-failed",
            &fields,
            bridge.get_attributes(valid_path, id),
        )
        .await
    }

    #[cfg(not(mobile))]
    {
        let _ = (app, valid_path, id);
        unsupported()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_optional_identifier_allows_missing_identifier() {
        assert_eq!(resolve_optional_identifier(None).unwrap(), None);
    }

    #[test]
    fn resolve_optional_identifier_trims_valid_identifier() {
        assert_eq!(
            resolve_optional_identifier(Some("  iCloud.com.example.app  ".to_string())).unwrap(),
            Some("iCloud.com.example.app".to_string())
        );
    }

    #[test]
    fn resolve_optional_identifier_rejects_empty_identifier_override() {
        let err = resolve_optional_identifier(Some("   ".to_string()))
            .expect_err("must reject empty override");
        match err {
            PluginError::InvalidArgument { detail } => {
                assert!(detail
                    .unwrap_or_default()
                    .contains("identifier is required"));
            }
            _ => panic!("expected invalid_argument"),
        }
    }
}

// ============================================================================
// Group 3: Directory Operations
// ============================================================================

#[tauri::command]
pub async fn create_directory<R: Runtime>(
    app: AppHandle<R>,
    path: String,
    identifier: Option<String>,
    options: Option<CreateDirectoryOptions>,
) -> Result<(), PluginError> {
    let valid_path = validate_relative_path(path)?;
    let id = resolve_optional_identifier(identifier)?;
    let (with_intermediate_directories, file_protection) =
        resolve_create_directory_options(options);
    let fields = [
        field("path", &valid_path),
        field("identifier", &id),
        field(
            "with_intermediate_directories",
            &with_intermediate_directories,
        ),
        field("file_protection", &file_protection),
    ];
    #[cfg(not(mobile))]
    let _ = &fields;

    #[cfg(mobile)]
    {
        let bridge = app.state::<IcloudContainer<R>>();
        run_command_with_logging(
            "create-directory-started",
            "create-directory-succeeded",
            "create-directory-failed",
            &fields,
            bridge.create_directory(
                valid_path,
                id,
                with_intermediate_directories,
                file_protection,
            ),
        )
        .await
    }

    #[cfg(not(mobile))]
    {
        let _ = (
            app,
            valid_path,
            id,
            with_intermediate_directories,
            file_protection,
        );
        unsupported()
    }
}

#[tauri::command]
pub async fn list_directory<R: Runtime>(
    app: AppHandle<R>,
    path: String,
    identifier: Option<String>,
    options: Option<ListDirectoryOptions>,
) -> Result<Vec<FolderEntry>, PluginError> {
    let valid_path = validate_relative_path(path)?;
    let id = resolve_optional_identifier(identifier)?;
    let (recursive, skips_hidden_files) = resolve_list_directory_options(options);
    let fields = [
        field("path", &valid_path),
        field("identifier", &id),
        field("recursive", &recursive),
        field("skips_hidden_files", &skips_hidden_files),
    ];
    #[cfg(not(mobile))]
    let _ = &fields;

    #[cfg(mobile)]
    {
        let bridge = app.state::<IcloudContainer<R>>();
        run_command_with_logging(
            "list-directory-started",
            "list-directory-succeeded",
            "list-directory-failed",
            &fields,
            bridge.list_directory(valid_path, id, recursive, skips_hidden_files),
        )
        .await
    }

    #[cfg(not(mobile))]
    {
        let _ = (app, valid_path, id, recursive, skips_hidden_files);
        unsupported()
    }
}

#[tauri::command]
pub async fn delete_item<R: Runtime>(
    app: AppHandle<R>,
    path: String,
    identifier: Option<String>,
) -> Result<(), PluginError> {
    let valid_path = validate_relative_path(path)?;
    let id = resolve_optional_identifier(identifier)?;
    let fields = [field("path", &valid_path), field("identifier", &id)];
    #[cfg(not(mobile))]
    let _ = &fields;

    #[cfg(mobile)]
    {
        let bridge = app.state::<IcloudContainer<R>>();
        run_command_with_logging(
            "delete-item-started",
            "delete-item-succeeded",
            "delete-item-failed",
            &fields,
            bridge.delete_item(valid_path, id),
        )
        .await
    }

    #[cfg(not(mobile))]
    {
        let _ = (app, valid_path, id);
        unsupported()
    }
}

#[tauri::command]
pub async fn trash_item<R: Runtime>(
    app: AppHandle<R>,
    path: String,
    identifier: Option<String>,
) -> Result<TrashItemResult, PluginError> {
    let valid_path = validate_relative_path(path)?;
    let id = resolve_optional_identifier(identifier)?;
    let fields = [field("path", &valid_path), field("identifier", &id)];
    #[cfg(not(mobile))]
    let _ = &fields;

    #[cfg(mobile)]
    {
        let bridge = app.state::<IcloudContainer<R>>();
        run_command_with_logging(
            "trash-item-started",
            "trash-item-succeeded",
            "trash-item-failed",
            &fields,
            bridge.trash_item(valid_path, id),
        )
        .await
    }

    #[cfg(not(mobile))]
    {
        let _ = (app, valid_path, id);
        unsupported()
    }
}

#[tauri::command]
pub async fn move_item<R: Runtime>(
    app: AppHandle<R>,
    source_path: String,
    destination_path: String,
    identifier: Option<String>,
) -> Result<FolderEntry, PluginError> {
    let valid_source_path = validate_relative_path(source_path)?;
    let valid_destination_path = validate_relative_path(destination_path)?;
    let id = resolve_optional_identifier(identifier)?;
    let fields = [
        field("source_path", &valid_source_path),
        field("destination_path", &valid_destination_path),
        field("identifier", &id),
    ];
    #[cfg(not(mobile))]
    let _ = &fields;

    #[cfg(mobile)]
    {
        let bridge = app.state::<IcloudContainer<R>>();
        run_command_with_logging(
            "move-item-started",
            "move-item-succeeded",
            "move-item-failed",
            &fields,
            bridge.move_item(valid_source_path, valid_destination_path, id),
        )
        .await
    }

    #[cfg(not(mobile))]
    {
        let _ = (app, valid_source_path, valid_destination_path, id);
        unsupported()
    }
}

#[tauri::command]
pub async fn copy_item<R: Runtime>(
    app: AppHandle<R>,
    source_path: String,
    destination_path: String,
    identifier: Option<String>,
) -> Result<FolderEntry, PluginError> {
    let valid_source_path = validate_relative_path(source_path)?;
    let valid_destination_path = validate_relative_path(destination_path)?;
    let id = resolve_optional_identifier(identifier)?;
    let fields = [
        field("source_path", &valid_source_path),
        field("destination_path", &valid_destination_path),
        field("identifier", &id),
    ];
    #[cfg(not(mobile))]
    let _ = &fields;

    #[cfg(mobile)]
    {
        let bridge = app.state::<IcloudContainer<R>>();
        run_command_with_logging(
            "copy-item-started",
            "copy-item-succeeded",
            "copy-item-failed",
            &fields,
            bridge.copy_item(valid_source_path, valid_destination_path, id),
        )
        .await
    }

    #[cfg(not(mobile))]
    {
        let _ = (app, valid_source_path, valid_destination_path, id);
        unsupported()
    }
}

// ============================================================================
// Group 4: iCloud Sync Controls
// ============================================================================

#[tauri::command]
pub async fn get_item_sync_status<R: Runtime>(
    app: AppHandle<R>,
    path: String,
    identifier: Option<String>,
) -> Result<SyncStatus, PluginError> {
    let valid_path = validate_relative_path(path)?;
    let id = resolve_optional_identifier(identifier)?;
    let fields = [field("path", &valid_path), field("identifier", &id)];
    #[cfg(not(mobile))]
    let _ = &fields;

    #[cfg(mobile)]
    {
        let bridge = app.state::<IcloudContainer<R>>();
        run_command_with_logging(
            "get-item-sync-status-started",
            "get-item-sync-status-succeeded",
            "get-item-sync-status-failed",
            &fields,
            bridge.get_item_sync_status(valid_path, id),
        )
        .await
    }

    #[cfg(not(mobile))]
    {
        let _ = (app, valid_path, id);
        unsupported()
    }
}

#[tauri::command]
pub async fn start_download<R: Runtime>(
    app: AppHandle<R>,
    path: String,
    identifier: Option<String>,
) -> Result<(), PluginError> {
    let valid_path = validate_relative_path(path)?;
    let id = resolve_optional_identifier(identifier)?;
    let fields = [field("path", &valid_path), field("identifier", &id)];
    #[cfg(not(mobile))]
    let _ = &fields;

    #[cfg(mobile)]
    {
        let bridge = app.state::<IcloudContainer<R>>();
        run_command_with_logging(
            "start-download-started",
            "start-download-succeeded",
            "start-download-failed",
            &fields,
            bridge.start_download(valid_path, id),
        )
        .await
    }

    #[cfg(not(mobile))]
    {
        let _ = (app, valid_path, id);
        unsupported()
    }
}

#[tauri::command]
pub async fn evict_item<R: Runtime>(
    app: AppHandle<R>,
    path: String,
    identifier: Option<String>,
) -> Result<(), PluginError> {
    let valid_path = validate_relative_path(path)?;
    let id = resolve_optional_identifier(identifier)?;
    let fields = [field("path", &valid_path), field("identifier", &id)];
    #[cfg(not(mobile))]
    let _ = &fields;

    #[cfg(mobile)]
    {
        let bridge = app.state::<IcloudContainer<R>>();
        run_command_with_logging(
            "evict-item-started",
            "evict-item-succeeded",
            "evict-item-failed",
            &fields,
            bridge.evict_item(valid_path, id),
        )
        .await
    }

    #[cfg(not(mobile))]
    {
        let _ = (app, valid_path, id);
        unsupported()
    }
}

#[tauri::command]
pub async fn is_ubiquitous<R: Runtime>(
    app: AppHandle<R>,
    path: String,
    identifier: Option<String>,
) -> Result<bool, PluginError> {
    let valid_path = validate_relative_path(path)?;
    let id = resolve_optional_identifier(identifier)?;
    let fields = [field("path", &valid_path), field("identifier", &id)];
    #[cfg(not(mobile))]
    let _ = &fields;

    #[cfg(mobile)]
    {
        let bridge = app.state::<IcloudContainer<R>>();
        run_command_with_logging(
            "is-ubiquitous-started",
            "is-ubiquitous-succeeded",
            "is-ubiquitous-failed",
            &fields,
            bridge.is_ubiquitous(valid_path, id),
        )
        .await
    }

    #[cfg(not(mobile))]
    {
        let _ = (app, valid_path, id);
        unsupported()
    }
}

// ============================================================================
// Group 5: File Watching
// ============================================================================

#[tauri::command]
pub async fn watch_directory<R: Runtime>(
    app: AppHandle<R>,
    path: String,
    recursive: bool,
    identifier: Option<String>,
) -> Result<String, PluginError> {
    let valid_path = validate_relative_path(path)?;
    let id = resolve_optional_identifier(identifier)?;
    let fields = [
        field("path", &valid_path),
        field("identifier", &id),
        field("recursive", &recursive),
    ];
    #[cfg(not(mobile))]
    let _ = &fields;

    #[cfg(mobile)]
    {
        let bridge = app.state::<IcloudContainer<R>>();
        run_command_with_logging(
            "watch-directory-started",
            "watch-directory-succeeded",
            "watch-directory-failed",
            &fields,
            bridge.watch_directory(valid_path, recursive, id),
        )
        .await
    }

    #[cfg(not(mobile))]
    {
        let _ = (app, valid_path, recursive, id);
        unsupported()
    }
}

#[tauri::command]
pub async fn unwatch<R: Runtime>(app: AppHandle<R>, watch_id: String) -> Result<(), PluginError> {
    let valid_watch_id = validate_watch_id(watch_id)?;
    let fields = [field("watch_id", &valid_watch_id)];
    #[cfg(not(mobile))]
    let _ = &fields;

    #[cfg(mobile)]
    {
        let bridge = app.state::<IcloudContainer<R>>();
        run_command_with_logging(
            "unwatch-started",
            "unwatch-succeeded",
            "unwatch-failed",
            &fields,
            bridge.unwatch(valid_watch_id),
        )
        .await
    }

    #[cfg(not(mobile))]
    {
        let _ = (app, valid_watch_id);
        unsupported()
    }
}

#[tauri::command]
pub async fn watch_file<R: Runtime>(
    app: AppHandle<R>,
    path: String,
    identifier: Option<String>,
) -> Result<String, PluginError> {
    let valid_path = validate_relative_path(path)?;
    let id = resolve_optional_identifier(identifier)?;
    let fields = [field("path", &valid_path), field("identifier", &id)];
    #[cfg(not(mobile))]
    let _ = &fields;

    #[cfg(mobile)]
    {
        let bridge = app.state::<IcloudContainer<R>>();
        run_command_with_logging(
            "watch-file-started",
            "watch-file-succeeded",
            "watch-file-failed",
            &fields,
            bridge.watch_file(valid_path, id),
        )
        .await
    }

    #[cfg(not(mobile))]
    {
        let _ = (app, valid_path, id);
        unsupported()
    }
}

#[tauri::command]
pub async fn unwatch_file<R: Runtime>(
    app: AppHandle<R>,
    watch_id: String,
) -> Result<(), PluginError> {
    let valid_watch_id = validate_watch_id(watch_id)?;
    let fields = [field("watch_id", &valid_watch_id)];
    #[cfg(not(mobile))]
    let _ = &fields;

    #[cfg(mobile)]
    {
        let bridge = app.state::<IcloudContainer<R>>();
        run_command_with_logging(
            "unwatch-file-started",
            "unwatch-file-succeeded",
            "unwatch-file-failed",
            &fields,
            bridge.unwatch_file(valid_watch_id),
        )
        .await
    }

    #[cfg(not(mobile))]
    {
        let _ = (app, valid_watch_id);
        unsupported()
    }
}

#[cfg(test)]
mod watch_tests {
    use super::*;

    #[test]
    fn validate_watch_id_rejects_empty_values() {
        let err = validate_watch_id("   ".to_string()).expect_err("must reject empty watch id");
        match err {
            PluginError::InvalidArgument { detail } => {
                assert!(detail.unwrap_or_default().contains("watch_id is required"));
            }
            _ => panic!("expected invalid_argument"),
        }
    }
}
