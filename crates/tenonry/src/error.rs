use serde::{Deserialize, Serialize};
use std::fmt;

/// Structured error categories returned by Tenonry.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum ErrorKind {
    /// Caller-supplied arguments failed validation.
    InvalidInput,
    /// The requested tool, operation, or package record could not be found.
    NotFound,
    /// The operation is not supported by the target platform or backend.
    Unsupported,
    /// Conflicting installation sources or backend state prevented execution.
    Conflict,
    /// Another Tenonry operation or package manager lock is currently active.
    Busy,
    /// Live system or package state changed after the confirmation plan was created.
    PlanExpired,
    /// A system-protected component cannot be modified automatically.
    Protected,
    /// Administrative elevation or authorization failed or was unavailable.
    Authorization,
    /// An underlying package manager or helper command exited with an error.
    CommandFailed,
    /// Reading or writing local journal, cache, or lock files failed.
    Persistence,
    /// An unexpected internal failure occurred.
    Internal,
}

/// Library error combining a machine-readable category with a user-facing message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Error {
    /// Machine-readable error classification.
    pub kind: ErrorKind,
    /// Human-readable error summary.
    pub message: String,
}

/// Convenience result alias using [`Error`].
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    /// Creates an error with an explicit [`ErrorKind`] and message.
    pub fn new(kind: ErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    /// Creates an [`ErrorKind::InvalidInput`] error.
    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::InvalidInput, message)
    }

    /// Creates an [`ErrorKind::NotFound`] error.
    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::NotFound, message)
    }

    /// Creates an [`ErrorKind::Unsupported`] error.
    pub fn unsupported(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Unsupported, message)
    }

    /// Creates an [`ErrorKind::Conflict`] error.
    pub fn conflict(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Conflict, message)
    }

    /// Creates an [`ErrorKind::Busy`] error.
    pub fn busy(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Busy, message)
    }

    /// Creates an [`ErrorKind::PlanExpired`] error.
    pub fn plan_expired(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::PlanExpired, message)
    }

    /// Creates an [`ErrorKind::Protected`] error.
    pub fn protected(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Protected, message)
    }

    /// Creates an [`ErrorKind::Authorization`] error.
    pub fn authorization(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Authorization, message)
    }

    /// Creates an [`ErrorKind::CommandFailed`] error.
    pub fn command_failed(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::CommandFailed, message)
    }

    /// Creates an [`ErrorKind::Persistence`] error.
    pub fn persistence(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Persistence, message)
    }

    /// Creates an [`ErrorKind::Internal`] error.
    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Internal, message)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for Error {}
