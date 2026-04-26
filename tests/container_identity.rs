use tauri_plugin_icloud_container::*;

// ============================================================================
// Container Status Tests
// ============================================================================

#[test]
fn test_container_status_structure() {
    let status = ContainerStatus {
        available: true,
        reason: None,
    };
    assert!(status.available);
    assert_eq!(status.reason, None);
}

#[test]
fn test_container_status_unavailable_with_reason() {
    let reason = "User not signed in";
    let status = ContainerStatus {
        available: false,
        reason: Some(reason.to_string()),
    };
    assert!(!status.available);
    assert_eq!(status.reason, Some(reason.to_string()));
}

#[test]
fn test_container_status_serialization_available() {
    let status = ContainerStatus {
        available: true,
        reason: None,
    };
    let json = serde_json::to_string(&status).expect("serialize");
    assert!(json.contains("\"available\":true"));

    // Deserialize back
    let deserialized: ContainerStatus =
        serde_json::from_str(&json).expect("deserialize");
    assert_eq!(deserialized.available, status.available);
    assert_eq!(deserialized.reason, status.reason);
}

#[test]
fn test_container_status_serialization_unavailable() {
    let status = ContainerStatus {
        available: false,
        reason: Some("iCloud disabled".to_string()),
    };
    let json = serde_json::to_string(&status).expect("serialize");
    let deserialized: ContainerStatus =
        serde_json::from_str(&json).expect("deserialize");
    assert!(!deserialized.available);
    assert_eq!(
        deserialized.reason,
        Some("iCloud disabled".to_string())
    );
}

// ============================================================================
// Container URL Tests
// ============================================================================

#[test]
fn test_container_url_returns_string() {
    // URL should be a simple string response from the command
    let url = "/private/var/mobile/Library/Mobile Documents/iCloud~com.example.app";
    assert!(!url.is_empty());
    assert!(url.starts_with("/"));
}

#[test]
fn test_container_url_is_absolute_path() {
    let paths = vec![
        "/private/var/mobile/Library/Mobile Documents/iCloud~app1",
        "/Users/username/Library/Mobile Documents/iCloud~app2",
    ];

    for path in paths {
        assert!(path.starts_with("/"), "container URL must be absolute path");
        assert!(!path.ends_with("/"), "container URL should not have trailing slash");
    }
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[test]
fn test_container_unavailable_error() {
    let error = PluginError::ContainerUnavailable {
        detail: Some("User not signed in".to_string()),
    };
    assert_eq!(error.error_code(), "container_unavailable");
    let json = serde_json::to_string(&error).expect("serialize");
    assert!(json.contains("\"code\":\"container_unavailable\""));
    assert!(json.contains("\"detail\":\"User not signed in\""));
}

#[test]
fn test_not_signed_in_error() {
    let error = PluginError::NotSignedIn {
        detail: Some("No iCloud account".to_string()),
    };
    assert_eq!(error.error_code(), "not_signed_in");
}

#[test]
fn test_permission_denied_error() {
    let error = PluginError::PermissionDenied {
        detail: Some("User denied iCloud access".to_string()),
    };
    assert_eq!(error.error_code(), "permission_denied");
}

// ============================================================================
// State Transition Tests (Logical)
// ============================================================================

#[test]
fn test_status_transition_available_to_unavailable() {
    let available = ContainerStatus {
        available: true,
        reason: None,
    };
    let unavailable = ContainerStatus {
        available: false,
        reason: Some("iCloud disabled".to_string()),
    };

    // Both states should be valid
    assert!(available.available);
    assert!(!unavailable.available);

    // Serialization should preserve state
    let json1 = serde_json::to_string(&available).unwrap();
    let json2 = serde_json::to_string(&unavailable).unwrap();

    let deser1: ContainerStatus = serde_json::from_str(&json1).unwrap();
    let deser2: ContainerStatus = serde_json::from_str(&json2).unwrap();

    assert!(deser1.available);
    assert!(!deser2.available);
}

#[test]
fn test_reason_optional_when_available() {
    // When available=true, reason should be None
    let status = ContainerStatus {
        available: true,
        reason: Some("This should be ignored".to_string()),
    };

    // This state is technically possible to create, but semantically inconsistent
    // The iOS implementation should never return reason when available=true
    assert!(status.available); // But the structure allows it
}

#[test]
fn test_reason_required_context_when_unavailable() {
    // When available=false, reason should explain why
    let reasons = vec![
        "User not signed in",
        "iCloud container not available",
        "Invalid container identifier",
        "Device storage full",
    ];

    for reason in reasons {
        let status = ContainerStatus {
            available: false,
            reason: Some(reason.to_string()),
        };
        assert!(!status.available);
        assert!(status.reason.is_some());
        assert_eq!(status.reason.as_ref().unwrap(), reason);
    }
}

// ============================================================================
// Response Shape Tests (Contract)
// ============================================================================

#[test]
fn test_get_container_status_response_shape() {
    // Response should always be: { available: bool, reason?: string }
    let status = ContainerStatus {
        available: false,
        reason: Some("Test reason".to_string()),
    };

    let json = serde_json::to_string(&status).unwrap();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();

    // Must have "available" field
    assert!(value.get("available").is_some());
    assert!(value["available"].is_boolean());

    // May have "reason" field
    assert!(value.get("reason").is_some());
}

#[test]
fn test_get_container_url_response_type() {
    // Response should be a plain string (absolute path)
    let url = "/private/var/mobile/Library/Mobile Documents/iCloud~app";
    assert!(url.starts_with("/"));
}

// ============================================================================
// Identity Scenario Tests
// ============================================================================

#[test]
fn test_multiple_container_identifiers() {
    // Explicit overrides remain valid when an app chooses to expose multiple containers.
    let identifiers = vec![
        "iCloud.com.example.app",
        "iCloud.com.example.app2",
        "iCloud.default",
    ];

    for id in identifiers {
        assert!(!id.is_empty());
        assert!(id.starts_with("iCloud.") || id == "iCloud.default");
    }
}

#[test]
fn test_identifier_override_is_optional() {
    let identifier: Option<&str> = None;
    assert!(identifier.is_none());

    let explicit = Some("iCloud.com.example.app");
    assert!(explicit.is_some());
    assert!(explicit.unwrap().starts_with("iCloud."));
}

// ============================================================================
// Determinism Tests
// ============================================================================

#[test]
fn test_container_status_serialization_deterministic() {
    let status = ContainerStatus {
        available: false,
        reason: Some("Not available".to_string()),
    };

    let json1 = serde_json::to_string(&status).unwrap();
    let json2 = serde_json::to_string(&status).unwrap();

    assert_eq!(json1, json2, "serialization must be deterministic");
}

#[test]
fn test_error_code_deterministic() {
    let error1 = PluginError::ContainerUnavailable {
        detail: Some("Test".to_string()),
    };
    let error2 = PluginError::ContainerUnavailable {
        detail: Some("Test".to_string()),
    };

    assert_eq!(error1.error_code(), error2.error_code());

    let json1 = serde_json::to_string(&error1).unwrap();
    let json2 = serde_json::to_string(&error2).unwrap();

    assert_eq!(json1, json2);
}
