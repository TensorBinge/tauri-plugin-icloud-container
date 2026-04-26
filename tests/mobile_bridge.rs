use std::fs;
use std::path::PathBuf;

#[test]
fn plugin_metadata_includes_group2_commands_and_default_acl() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let build_rs = fs::read_to_string(root.join("build.rs")).expect("read build.rs");
    let default_permissions =
        fs::read_to_string(root.join("permissions/default.toml")).expect("read permissions/default.toml");

    assert!(build_rs.contains("read_file"));
    assert!(build_rs.contains("write_file"));
    assert!(build_rs.contains("create_file"));
    assert!(build_rs.contains("item_exists"));
    assert!(build_rs.contains("get_attributes"));

    assert!(default_permissions.contains("allow-read-file"));
    assert!(default_permissions.contains("allow-write-file"));
    assert!(default_permissions.contains("allow-create-file"));
    assert!(default_permissions.contains("allow-item-exists"));
    assert!(default_permissions.contains("allow-get-attributes"));
}

#[test]
fn swift_tauri_plugin_exposes_expected_group2_methods() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let swift_plugin = fs::read_to_string(root.join("ios/Sources/ICloudContainerTauriPlugin.swift"))
        .expect("read ICloudContainerTauriPlugin.swift");

    assert!(swift_plugin.contains("@objc public func readFile"));
    assert!(swift_plugin.contains("@objc public func writeFile"));
    assert!(swift_plugin.contains("@objc public func createFile"));
    assert!(swift_plugin.contains("@objc public func itemExists"));
    assert!(swift_plugin.contains("@objc public func getAttributes"));
}

#[test]
fn mobile_bridge_threads_identifier_overrides_to_container_backed_calls() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mobile_bridge = fs::read_to_string(root.join("src/mobile.rs"))
        .expect("read src/mobile.rs");

    for method in [
        "pub async fn read_file",
        "pub async fn write_file",
        "pub async fn create_file",
        "pub async fn item_exists",
        "pub async fn get_attributes",
        "pub async fn create_directory",
        "pub async fn list_directory",
        "pub async fn delete_item",
        "pub async fn trash_item",
        "pub async fn move_item",
        "pub async fn copy_item",
        "pub async fn get_item_sync_status",
        "pub async fn start_download",
        "pub async fn evict_item",
        "pub async fn is_ubiquitous",
        "pub async fn watch_directory",
        "pub async fn watch_file",
    ] {
        assert!(mobile_bridge.contains(method), "missing bridge method: {method}");
    }

    assert!(mobile_bridge.contains("self.resolve_identifier(identifier)"));
}

#[test]
fn swift_tauri_plugin_exports_init_symbol() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let swift_plugin = fs::read_to_string(root.join("ios/Sources/ICloudContainerTauriPlugin.swift"))
        .expect("read ICloudContainerTauriPlugin.swift");

    assert!(swift_plugin.contains("@_cdecl(\"init_plugin_icloud_container\")"));
}
