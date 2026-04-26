use serde::Serialize;
use std::io;

/// Plugin error taxonomy (9 error codes)
#[derive(Debug, thiserror::Error, Serialize)]
#[serde(tag = "code", rename_all = "snake_case")]
pub enum PluginError {
    #[error("iCloud container not accessible")]
    ContainerUnavailable { detail: Option<String> },

    #[error("user not signed into iCloud")]
    NotSignedIn { detail: Option<String> },

    #[error("OS permission denied")]
    PermissionDenied { detail: Option<String> },

    #[error("path escapes container sandbox")]
    PathOutsideContainer { detail: Option<String> },

    #[error("file or directory not found")]
    NotFound { detail: Option<String> },

    #[error("file or directory already exists")]
    AlreadyExists { detail: Option<String> },

    #[error("I/O error")]
    IoError { detail: Option<String> },

    #[error("iCloud sync error")]
    SyncError { detail: Option<String> },

    #[error("invalid command argument")]
    InvalidArgument { detail: Option<String> },
}

impl PluginError {
    /// Return the stable error code as lowercase string
    pub fn error_code(&self) -> &'static str {
        match self {
            PluginError::ContainerUnavailable { .. } => "container_unavailable",
            PluginError::NotSignedIn { .. } => "not_signed_in",
            PluginError::PermissionDenied { .. } => "permission_denied",
            PluginError::PathOutsideContainer { .. } => "path_outside_container",
            PluginError::NotFound { .. } => "not_found",
            PluginError::AlreadyExists { .. } => "already_exists",
            PluginError::IoError { .. } => "io_error",
            PluginError::SyncError { .. } => "sync_error",
            PluginError::InvalidArgument { .. } => "invalid_argument",
        }
    }
}

/// Map standard IO errors to PluginError
impl From<io::Error> for PluginError {
    fn from(err: io::Error) -> Self {
        match err.kind() {
            io::ErrorKind::NotFound => PluginError::NotFound {
                detail: Some(err.to_string()),
            },
            io::ErrorKind::PermissionDenied => PluginError::PermissionDenied {
                detail: Some(err.to_string()),
            },
            io::ErrorKind::AlreadyExists => PluginError::AlreadyExists {
                detail: Some(err.to_string()),
            },
            _ => PluginError::IoError {
                detail: Some(err.to_string()),
            },
        }
    }
}

/// Helper to return unsupported operations (for stubbed commands)
pub fn unsupported<T>() -> Result<T, PluginError> {
    Err(PluginError::InvalidArgument {
        detail: Some("not implemented".to_string()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_codes() {
        let tests = vec![
            (
                PluginError::ContainerUnavailable {
                    detail: Some("test".to_string()),
                },
                "container_unavailable",
            ),
            (
                PluginError::NotSignedIn {
                    detail: Some("test".to_string()),
                },
                "not_signed_in",
            ),
            (
                PluginError::PermissionDenied {
                    detail: Some("test".to_string()),
                },
                "permission_denied",
            ),
            (
                PluginError::PathOutsideContainer {
                    detail: Some("test".to_string()),
                },
                "path_outside_container",
            ),
            (
                PluginError::NotFound {
                    detail: Some("test".to_string()),
                },
                "not_found",
            ),
            (
                PluginError::AlreadyExists {
                    detail: Some("test".to_string()),
                },
                "already_exists",
            ),
            (
                PluginError::IoError {
                    detail: Some("test".to_string()),
                },
                "io_error",
            ),
            (
                PluginError::SyncError {
                    detail: Some("test".to_string()),
                },
                "sync_error",
            ),
            (
                PluginError::InvalidArgument {
                    detail: Some("test".to_string()),
                },
                "invalid_argument",
            ),
        ];

        for (error, expected_code) in tests {
            assert_eq!(error.error_code(), expected_code);
        }
    }

    #[test]
    fn test_io_error_mapping() {
        let not_found = io::Error::new(io::ErrorKind::NotFound, "file not found");
        match PluginError::from(not_found) {
            PluginError::NotFound { detail } => {
                assert!(detail.is_some());
            }
            _ => panic!("expected NotFound error"),
        }

        let perm_denied = io::Error::new(io::ErrorKind::PermissionDenied, "access denied");
        match PluginError::from(perm_denied) {
            PluginError::PermissionDenied { detail } => {
                assert!(detail.is_some());
            }
            _ => panic!("expected PermissionDenied error"),
        }
    }

    #[test]
    fn test_error_serialization() {
        let error = PluginError::ContainerUnavailable {
            detail: Some("not available".to_string()),
        };
        let json = serde_json::to_string(&error).expect("failed to serialize");
        assert!(json.contains("container_unavailable"));
    }
}
