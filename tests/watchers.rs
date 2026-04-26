use std::fs;
use std::path::PathBuf;

use tauri_plugin_icloud_container::{
    commands::validate_relative_path, DirectoryWatchEvent, FileWatchEvent, DIRECTORY_CHANGED_EVENT,
    FILE_CHANGED_EVENT,
};

#[test]
fn watcher_event_names_are_stable() {
    assert_eq!(DIRECTORY_CHANGED_EVENT, "icloud://directory-changed");
    assert_eq!(FILE_CHANGED_EVENT, "icloud://file-changed");
}

#[test]
fn directory_watch_event_serialization_is_camel_case() {
    let event = DirectoryWatchEvent {
        watch_id: "watch-1".to_string(),
        path: "Documents".to_string(),
        recursive: true,
        entries: vec!["Documents/file.txt".to_string()],
    };

    let json = serde_json::to_string(&event).expect("serialize");
    assert!(json.contains("\"watchId\":\"watch-1\""));
    assert!(json.contains("\"recursive\":true"));
    assert!(json.contains("\"entries\""));
}

#[test]
fn file_watch_event_serialization_is_camel_case() {
    let event = FileWatchEvent {
        watch_id: "watch-2".to_string(),
        path: "Documents/file.txt".to_string(),
    };

    let json = serde_json::to_string(&event).expect("serialize");
    assert!(json.contains("\"watchId\":\"watch-2\""));
    assert!(json.contains("\"path\":\"Documents/file.txt\""));
}

#[test]
fn watcher_paths_reuse_relative_path_sandboxing() {
    let valid = validate_relative_path("Documents/file.txt".to_string()).unwrap();
    assert_eq!(valid, "Documents/file.txt");

    assert!(validate_relative_path("/tmp/file.txt".to_string()).is_err());
}

#[test]
fn swift_plugin_exposes_watcher_commands_and_native_primitives() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let tauri_plugin =
        fs::read_to_string(root.join("ios/Sources/ICloudContainerTauriPlugin.swift"))
            .expect("read ICloudContainerTauriPlugin.swift");
    let service = fs::read_to_string(root.join("ios/Sources/ICloudContainerPlugin.swift"))
        .expect("read ICloudContainerPlugin.swift");

    for method in [
        "@objc public func watchDirectory",
        "@objc public func unwatch",
        "@objc public func watchFile",
        "@objc public func unwatchFile",
    ] {
        assert!(
            tauri_plugin.contains(method),
            "missing watcher command: {method}"
        );
    }

    assert!(tauri_plugin.contains("trigger(directoryChangedEventName"));
    assert!(tauri_plugin.contains("trigger(fileChangedEventName"));
    assert!(service.contains("NSMetadataQuery"));
    assert!(service.contains("NSFilePresenter"));
    assert!(service.contains("directoryWatchers"));
    assert!(service.contains("fileWatchers"));
}

#[test]
fn swift_service_exposes_cleanup_helpers() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let service = fs::read_to_string(root.join("ios/Sources/ICloudContainerPlugin.swift"))
        .expect("read ICloudContainerPlugin.swift");

    assert!(service.contains("public func unwatch("));
    assert!(service.contains("public func unwatchFile("));
    assert!(service.contains("removeValue(forKey:"));
}

#[test]
fn swift_file_watchers_handle_app_lifecycle() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let service = fs::read_to_string(root.join("ios/Sources/ICloudContainerPlugin.swift"))
        .expect("read ICloudContainerPlugin.swift");

    assert!(service.contains("setupApplicationLifecycleObservers()"));
    assert!(service.contains("UIApplication.didEnterBackgroundNotification"));
    assert!(service.contains("UIApplication.willEnterForegroundNotification"));
    assert!(service.contains("suspendFileWatchers()"));
    assert!(service.contains("resumeFileWatchers()"));
    assert!(service.contains("watcher.start(emitInitialChange: false)"));
}

#[test]
fn swift_file_watchers_make_presenter_registration_idempotent() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let service = fs::read_to_string(root.join("ios/Sources/ICloudContainerPlugin.swift"))
        .expect("read ICloudContainerPlugin.swift");

    assert!(service.contains("private var isPresenting = false"));
    assert!(service.contains("guard !isPresenting else"));
    assert!(service.contains("guard isPresenting else"));
}
