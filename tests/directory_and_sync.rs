use std::fs;
use std::path::PathBuf;

use tauri_plugin_icloud_container::commands::{
    resolve_create_directory_options, resolve_list_directory_options, validate_relative_path,
};
use tauri_plugin_icloud_container::{
    CreateDirectoryOptions, FileProtectionType, ListDirectoryOptions, PluginError,
};

#[test]
fn create_directory_defaults_match_spec() {
    let (with_intermediate_directories, protection) = resolve_create_directory_options(None);
    assert!(with_intermediate_directories);
    assert_eq!(protection, FileProtectionType::Complete);
}

#[test]
fn create_directory_explicit_options_are_preserved() {
    let options = CreateDirectoryOptions {
        with_intermediate_directories: Some(false),
        file_protection: Some(FileProtectionType::None),
    };

    let (with_intermediate_directories, protection) =
        resolve_create_directory_options(Some(options));

    assert!(!with_intermediate_directories);
    assert_eq!(protection, FileProtectionType::None);
}

#[test]
fn list_directory_defaults_match_spec() {
    let (recursive, skips_hidden_files) = resolve_list_directory_options(None);
    assert!(!recursive);
    assert!(!skips_hidden_files);
}

#[test]
fn list_directory_explicit_options_are_preserved() {
    let options = ListDirectoryOptions {
        recursive: Some(true),
        skips_hidden_files: Some(true),
    };

    let (recursive, skips_hidden_files) = resolve_list_directory_options(Some(options));
    assert!(recursive);
    assert!(skips_hidden_files);
}

#[test]
fn move_and_copy_paths_reuse_relative_path_sandboxing() {
    let valid_source = validate_relative_path("workspace/source.md".to_string()).unwrap();
    let valid_destination =
        validate_relative_path("workspace/archive/source.md".to_string()).unwrap();

    assert_eq!(valid_source, "workspace/source.md");
    assert_eq!(valid_destination, "workspace/archive/source.md");

    let err =
        validate_relative_path("../escape.txt".to_string()).expect_err("must reject traversal");
    match err {
        PluginError::PathOutsideContainer { .. } => {}
        _ => panic!("expected path_outside_container"),
    }
}

#[test]
fn swift_plugin_exposes_directory_and_sync_methods() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let swift_plugin =
        fs::read_to_string(root.join("ios/Sources/ICloudContainerTauriPlugin.swift"))
            .expect("read ICloudContainerTauriPlugin.swift");

    for method in [
        "@objc public func createDirectory",
        "@objc public func listDirectory",
        "@objc public func deleteItem",
        "@objc public func trashItem",
        "@objc public func moveItem",
        "@objc public func copyItem",
        "@objc public func getItemSyncStatus",
        "@objc public func startDownload",
        "@objc public func evictItem",
        "@objc public func isUbiquitous",
    ] {
        assert!(
            swift_plugin.contains(method),
            "missing Swift method: {method}"
        );
    }
}

#[test]
fn swift_service_uses_native_trash_and_non_destructive_transfer_semantics() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let service = fs::read_to_string(root.join("ios/Sources/ICloudContainerPlugin.swift"))
        .expect("read ICloudContainerPlugin.swift");
    let package =
        fs::read_to_string(root.join("ios/Package.swift")).expect("read ios/Package.swift");

    assert!(service.contains("FileManager.default.trashItem"));
    assert!(!service.contains("appendingPathComponent(\".Trash\""));
    assert!(!service.contains("try FileManager.default.removeItem(at: coordinatedDestination)"));
    assert!(!service.contains("try FileManager.default.removeItem(at: destinationUrl)"));
    assert!(!service.contains("options: .forReplacing"));
    assert!(!package.contains("-suppress-warnings"));
}
