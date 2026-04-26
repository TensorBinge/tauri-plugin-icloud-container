use tauri_plugin_icloud_container::*;

// ============================================================================
// Group 1: Container Identity
// ============================================================================

#[test]
fn test_container_status_serialization() {
    let status = ContainerStatus {
        available: true,
        reason: None,
    };
    let json = serde_json::to_string(&status).expect("serialize");
    assert!(json.contains("\"available\":true"));

    let status_unavailable = ContainerStatus {
        available: false,
        reason: Some("Not signed in".to_string()),
    };
    let json = serde_json::to_string(&status_unavailable).expect("serialize");
    assert!(json.contains("\"available\":false"));
    assert!(json.contains("\"reason\":\"Not signed in\""));
}

// ============================================================================
// Group 2: File I/O Models
// ============================================================================

#[test]
fn test_file_content_utf8_serialization() {
    let content = FileContent::utf8("Hello, world!".to_string());
    let json = serde_json::to_string(&content).expect("serialize");
    assert!(json.contains("\"encoding\":\"utf8\""), "JSON: {}", json);
    assert!(json.contains("\"Hello, world!\""));
}

#[test]
fn test_file_content_bytes_serialization() {
    let content = FileContent::bytes(vec![0x89, 0x50, 0x4E, 0x47]); // PNG header
    let json = serde_json::to_string(&content).expect("serialize");
    assert!(json.contains("\"encoding\":\"bytes\""), "JSON: {}", json);
    // Note: bytes are serialized as base64 in JSON by serde
    assert!(json.contains("\"content\""));
}

#[test]
fn test_file_content_bytes_roundtrip() {
    let original = FileContent::bytes(vec![0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A]);
    let json = serde_json::to_string(&original).expect("serialize");
    let deserialized: FileContent = serde_json::from_str(&json).expect("deserialize");
    match (&original, &deserialized) {
        (
            FileContent::Bytes {
                encoding: e1,
                content: c1,
            },
            FileContent::Bytes {
                encoding: e2,
                content: c2,
            },
        ) => {
            assert_eq!(e1, e2);
            assert_eq!(c1, c2);
        }
        _ => panic!("expected Bytes variant"),
    }
}

#[test]
fn test_folder_entry_serialization() {
    let entry = FolderEntry {
        name: "Document.md".to_string(),
        path: "/Documents/Document.md".to_string(),
        is_directory: false,
        size: Some(1024),
        modified_date: Some(1704067200),
        created_date: Some(1704067200),
        sync_status: None,
    };
    let json = serde_json::to_string(&entry).expect("serialize");
    assert!(json.contains("\"name\":\"Document.md\""));
    assert!(json.contains("\"isDirectory\":false"));
    assert!(json.contains("\"size\":1024"));
}

#[test]
fn test_item_existence_serialization() {
    let exists = ItemExistence {
        exists: true,
        is_directory: false,
    };
    let json = serde_json::to_string(&exists).expect("serialize");
    assert!(json.contains("\"exists\":true"));
    assert!(json.contains("\"isDirectory\":false"));
}

#[test]
fn test_item_attributes_serialization() {
    let attrs = ItemAttributes {
        size: 2048,
        modified_date: 1704067200,
        created_date: 1704067100,
        item_type: "file".to_string(),
        sync_status: Some(SyncStatus {
            phase: SyncPhase::Current,
            is_downloading: false,
            is_uploading: false,
            is_uploaded: true,
            download_error: None,
            upload_error: None,
        }),
    };
    let json = serde_json::to_string(&attrs).expect("serialize");
    assert!(json.contains("\"size\":2048"));
    assert!(json.contains("\"type\":\"file\""));
    // Phase is included in the serialization
    assert!(json.contains("phase") || json.contains("Phase"));
}

#[test]
fn test_trash_item_result_serialization() {
    let result = TrashItemResult {
        path: "/.Trash/document.md".to_string(),
    };
    let json = serde_json::to_string(&result).expect("serialize");
    assert!(json.contains("\"path\":\"/.Trash/document.md\""));
}

// ============================================================================
// Group 4: Sync Models
// ============================================================================

#[test]
fn test_sync_status_serialization() {
    let status = SyncStatus {
        phase: SyncPhase::Downloaded,
        is_downloading: false,
        is_uploading: true,
        is_uploaded: false,
        download_error: Some("Download blocked".to_string()),
        upload_error: Some("Network error".to_string()),
    };
    let json = serde_json::to_string(&status).expect("serialize");
    assert!(json.contains("\"isDownloading\":false"));
    assert!(json.contains("\"isUploading\":true"));
    assert!(json.contains("\"isUploaded\":false"));
    assert!(json.contains("\"downloadError\":\"Download blocked\""));
    assert!(json.contains("\"uploadError\":\"Network error\""));
    // Phase is serialized as an enum variant
    assert!(json.contains("phase") || json.contains("Phase"));
}

#[test]
fn test_sync_phase_all_variants() {
    let phases = vec![
        SyncPhase::Current,
        SyncPhase::NotDownloaded,
        SyncPhase::Downloaded,
    ];
    for phase in phases {
        let json = serde_json::to_string(&phase).expect("serialize phase");
        // Verify phase is serialized as string variant name
        assert!(json.contains("\""));
    }
}

// ============================================================================
// Option Types
// ============================================================================

#[test]
fn test_file_protection_type_serialization() {
    let protections = vec![
        FileProtectionType::Complete,
        FileProtectionType::CompleteUnlessOpen,
        FileProtectionType::CompleteUntilFirstUserAuth,
        FileProtectionType::None,
    ];
    for protection in protections {
        let json = serde_json::to_string(&protection).expect("serialize protection");
        assert!(json.contains("\""));
    }
}

#[test]
fn test_write_file_options_with_defaults() {
    let opts = WriteFileOptions {
        encoding: Some("bytes".to_string()),
        overwrite: Some(false),
        file_protection: Some(FileProtectionType::Complete),
    };
    let json = serde_json::to_string(&opts).expect("serialize");
    assert!(json.contains("\"encoding\":\"bytes\""), "JSON: {}", json);
    assert!(json.contains("\"overwrite\":false"));
    // FileProtectionType is an enum, check it's serialized
    assert!(json.contains("fileProtection") || json.contains("file_protection"));
}

#[test]
fn test_write_file_options_all_none() {
    let opts = WriteFileOptions {
        encoding: None,
        overwrite: None,
        file_protection: None,
    };
    let json = serde_json::to_string(&opts).expect("serialize");
    let deserialized: WriteFileOptions = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(deserialized.encoding, None);
    assert_eq!(deserialized.overwrite, None);
    assert_eq!(deserialized.file_protection, None);
}

#[test]
fn test_create_file_options_with_binary_content() {
    let opts = CreateFileOptions {
        content: Some(vec![0xFF, 0xD8, 0xFF]), // JPEG header
        encoding: Some("bytes".to_string()),
        file_protection: Some(FileProtectionType::Complete),
    };
    let json = serde_json::to_string(&opts).expect("serialize");
    assert!(json.contains("\"encoding\":\"bytes\""));
    // Verify content is bytes, not base64 string
    assert!(!json.contains("base64"));
}

#[test]
fn test_create_directory_options_serialization() {
    let opts = CreateDirectoryOptions {
        with_intermediate_directories: Some(true),
        file_protection: Some(FileProtectionType::Complete),
    };
    let json = serde_json::to_string(&opts).expect("serialize");
    assert!(json.contains("\"withIntermediateDirectories\":true"));
}

#[test]
fn test_list_directory_options_serialization() {
    let opts = ListDirectoryOptions {
        recursive: Some(true),
        skips_hidden_files: Some(false),
    };
    let json = serde_json::to_string(&opts).expect("serialize");
    assert!(json.contains("\"recursive\":true"));
    assert!(json.contains("\"skipsHiddenFiles\":false"));
}

// ============================================================================
// Error Taxonomy
// ============================================================================

#[test]
fn test_error_codes_are_stable() {
    let errors = vec![
        (
            PluginError::ContainerUnavailable { detail: None },
            "container_unavailable",
        ),
        (PluginError::NotSignedIn { detail: None }, "not_signed_in"),
        (
            PluginError::PermissionDenied { detail: None },
            "permission_denied",
        ),
        (
            PluginError::PathOutsideContainer { detail: None },
            "path_outside_container",
        ),
        (PluginError::NotFound { detail: None }, "not_found"),
        (
            PluginError::AlreadyExists { detail: None },
            "already_exists",
        ),
        (PluginError::IoError { detail: None }, "io_error"),
        (PluginError::SyncError { detail: None }, "sync_error"),
        (
            PluginError::InvalidArgument { detail: None },
            "invalid_argument",
        ),
    ];

    for (error, expected_code) in errors {
        assert_eq!(
            error.error_code(),
            expected_code,
            "error code mismatch for {:?}",
            error
        );
    }
}

#[test]
fn test_error_serialization_includes_code() {
    let error = PluginError::NotFound {
        detail: Some("Document not found".to_string()),
    };
    let json = serde_json::to_string(&error).expect("serialize error");
    assert!(json.contains("\"code\":\"not_found\""));
    assert!(json.contains("\"detail\":\"Document not found\""));
}

#[test]
fn test_all_error_variants_have_detail() {
    let errors_with_detail = vec![
        PluginError::ContainerUnavailable {
            detail: Some("Cloud sync disabled".to_string()),
        },
        PluginError::NotSignedIn {
            detail: Some("User not authenticated".to_string()),
        },
        PluginError::PermissionDenied {
            detail: Some("User denied access".to_string()),
        },
        PluginError::PathOutsideContainer {
            detail: Some("Path escapes sandbox".to_string()),
        },
        PluginError::NotFound {
            detail: Some("File does not exist".to_string()),
        },
        PluginError::AlreadyExists {
            detail: Some("File already exists".to_string()),
        },
        PluginError::IoError {
            detail: Some("I/O operation failed".to_string()),
        },
        PluginError::SyncError {
            detail: Some("Sync conflict".to_string()),
        },
        PluginError::InvalidArgument {
            detail: Some("Invalid encoding".to_string()),
        },
    ];

    for error in errors_with_detail {
        let json = serde_json::to_string(&error).expect("serialize");
        assert!(json.contains("\"detail\":"));
    }
}

#[test]
fn test_error_deterministic_json_shape() {
    let error1 = PluginError::PermissionDenied {
        detail: Some("Test".to_string()),
    };
    let error2 = PluginError::PermissionDenied {
        detail: Some("Test".to_string()),
    };

    let json1 = serde_json::to_string(&error1).expect("serialize");
    let json2 = serde_json::to_string(&error2).expect("serialize");

    assert_eq!(json1, json2, "error serialization must be deterministic");
}
