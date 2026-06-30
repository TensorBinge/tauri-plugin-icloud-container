use tauri_plugin_icloud_container::commands::{
    resolve_create_file_options, resolve_encoding, resolve_read_options, resolve_write_options,
    validate_relative_path,
};
use tauri_plugin_icloud_container::{
    CreateFileOptions, FileProtectionType, PluginError, ReadFileOptions, WriteFileOptions,
};

#[test]
fn test_validate_relative_path_accepts_normal_relative_path() {
    let validated = validate_relative_path("docs/note.md".to_string()).expect("valid path");
    assert_eq!(validated, "docs/note.md");
}

#[test]
fn test_validate_relative_path_rejects_empty_path() {
    let err = validate_relative_path("   ".to_string()).expect_err("must reject empty");
    match err {
        PluginError::InvalidArgument { detail } => {
            assert!(detail.unwrap_or_default().contains("path is required"));
        }
        _ => panic!("expected invalid_argument"),
    }
}

#[test]
fn test_validate_relative_path_rejects_absolute_path() {
    // Use a path that is absolute on all platforms
    let absolute = if cfg!(windows) {
        r"C:\Windows\System32\drivers\etc\hosts"
    } else {
        "/etc/passwd"
    };
    let err = validate_relative_path(absolute.to_string()).expect_err("must reject absolute");
    match err {
        PluginError::PathOutsideContainer { detail } => {
            assert!(detail.unwrap_or_default().contains("relative"));
        }
        _ => panic!("expected path_outside_container"),
    }
}

#[test]
fn test_validate_relative_path_rejects_parent_traversal() {
    let err =
        validate_relative_path("../secret.txt".to_string()).expect_err("must reject traversal");
    match err {
        PluginError::PathOutsideContainer { detail } => {
            assert!(detail.unwrap_or_default().contains("traverse"));
        }
        _ => panic!("expected path_outside_container"),
    }
}

#[test]
fn test_resolve_encoding_defaults_to_utf8() {
    let encoding = resolve_encoding(None).expect("default encoding");
    assert_eq!(encoding, "utf8");
}

#[test]
fn test_resolve_encoding_rejects_invalid_encoding() {
    let err = resolve_encoding(Some("base64".to_string())).expect_err("invalid encoding");
    match err {
        PluginError::InvalidArgument { detail } => {
            assert!(detail.unwrap_or_default().contains("invalid encoding"));
        }
        _ => panic!("expected invalid_argument"),
    }
}

#[test]
fn test_resolve_read_options_defaults_to_utf8() {
    let encoding = resolve_read_options(None).expect("default read encoding");
    assert_eq!(encoding, "utf8");
}

#[test]
fn test_resolve_read_options_respects_bytes_encoding() {
    let opts = ReadFileOptions {
        encoding: Some("bytes".to_string()),
    };

    let encoding = resolve_read_options(Some(opts)).expect("bytes encoding");
    assert_eq!(encoding, "bytes");
}

#[test]
fn test_resolve_write_options_defaults() {
    let (encoding, overwrite, protection) = resolve_write_options(None).expect("defaults");
    assert_eq!(encoding, "utf8");
    assert!(overwrite);
    assert_eq!(protection, FileProtectionType::Complete);
}

#[test]
fn test_resolve_write_options_uses_explicit_values() {
    let options = WriteFileOptions {
        encoding: Some("bytes".to_string()),
        overwrite: Some(false),
        file_protection: Some(FileProtectionType::None),
    };

    let (encoding, overwrite, protection) =
        resolve_write_options(Some(options)).expect("resolved options");

    assert_eq!(encoding, "bytes");
    assert!(!overwrite);
    assert_eq!(protection, FileProtectionType::None);
}

#[test]
fn test_resolve_create_file_options_defaults() {
    let (content, encoding, protection) = resolve_create_file_options(None).expect("defaults");
    assert!(content.is_empty());
    assert_eq!(encoding, "utf8");
    assert_eq!(protection, FileProtectionType::Complete);
}

#[test]
fn test_resolve_create_file_options_preserves_binary_content() {
    let raw = vec![0x00, 0x10, 0xFF];
    let options = CreateFileOptions {
        content: Some(raw.clone()),
        encoding: Some("bytes".to_string()),
        file_protection: Some(FileProtectionType::CompleteUntilFirstUserAuth),
    };

    let (content, encoding, protection) =
        resolve_create_file_options(Some(options)).expect("resolved options");

    assert_eq!(content, raw);
    assert_eq!(encoding, "bytes");
    assert_eq!(protection, FileProtectionType::CompleteUntilFirstUserAuth);
}

#[test]
fn swift_write_file_does_not_create_parent_directories_implicitly() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let service = std::fs::read_to_string(root.join("ios/Sources/ICloudContainerPlugin.swift"))
        .expect("read ICloudContainerPlugin.swift");

    assert!(!service.contains("let parent = coordinatedUrl.deletingLastPathComponent()"));
    assert!(!service.contains("createDirectory(at: parent, withIntermediateDirectories: true)"));
}
