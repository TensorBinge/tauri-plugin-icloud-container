#[cfg(desktop)]
mod desktop;
#[cfg(mobile)]
mod mobile;

pub mod commands;
mod error;
mod logging;
mod models;

#[cfg(desktop)]
pub use desktop::IcloudContainer;
#[cfg(mobile)]
pub use mobile::IcloudContainer;

pub use error::PluginError;
pub use models::{
    ContainerStatus, CreateDirectoryOptions, CreateFileOptions, DirectoryWatchEvent, FileContent,
    FileProtectionType, FileWatchEvent, FolderEntry, ItemAttributes, ItemExistence,
    ListDirectoryOptions, ReadFileOptions, SyncPhase, SyncStatus, TrashItemResult,
    WriteFileOptions, DIRECTORY_CHANGED_EVENT, FILE_CHANGED_EVENT,
};
use serde::{Deserialize, Serialize};

use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IcloudContainerConfig {
    pub identifier: Option<String>,
}

pub fn init<R: Runtime>() -> TauriPlugin<R> {
    init_with_config(IcloudContainerConfig::default())
}

pub fn init_with_config<R: Runtime>(config: IcloudContainerConfig) -> TauriPlugin<R> {
    Builder::new("icloud-container")
        .invoke_handler(tauri::generate_handler![
            commands::get_container_status,
            commands::get_container_url,
            commands::read_file,
            commands::write_file,
            commands::create_file,
            commands::item_exists,
            commands::get_attributes,
            commands::create_directory,
            commands::list_directory,
            commands::delete_item,
            commands::trash_item,
            commands::move_item,
            commands::copy_item,
            commands::get_item_sync_status,
            commands::start_download,
            commands::evict_item,
            commands::is_ubiquitous,
            commands::watch_directory,
            commands::unwatch,
            commands::watch_file,
            commands::unwatch_file,
        ])
        .setup(move |app, api| {
            let configured_identifier = config.identifier.clone();
            crate::plugin_log_info!(
                "icloud-container.plugin",
                "setup-started",
                "configured_identifier" => configured_identifier
            );

            #[cfg(mobile)]
            let icloud = mobile::init(app, api, configured_identifier)?;
            #[cfg(desktop)]
            let icloud = desktop::init(app, api, configured_identifier)?;

            app.manage(icloud);
            crate::plugin_log_info!("icloud-container.plugin", "setup-succeeded");
            Ok(())
        })
        .build()
}
